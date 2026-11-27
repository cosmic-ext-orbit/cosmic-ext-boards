// SPDX-License-Identifier: GPL-3.0-only

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, menu, nav_bar};

use crate::boards::{Board, Boards, Id, Message as BoardMessage, NameTarget, Page, Store};
use crate::config::Config;
use crate::fl;
use crate::settings;

pub struct AppModel {
    core: cosmic::Core,
    about: About,
    drawer: Option<AppDrawer>,
    config: Config,
    settings_error: Option<String>,
    boards: Boards,
    store: Option<Store>,
    nav: nav_bar::Model,
    page: Page,
}

#[derive(Clone, Copy)]
enum AppDrawer {
    About,
    Settings,
}

#[derive(Clone, Debug)]
pub enum Message {
    Board(BoardMessage),
    About,
    Settings,
    CloseDrawer,
    Preference(settings::Change),
    OpenUrl(String),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.edfloreshz.Boards";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, (): ()) -> (Self, Task<cosmic::Action<Message>>) {
        let mut app = Self {
            core,
            about: About::default()
                .name(fl!("app-title"))
                .icon(widget::icon::from_svg_bytes(include_bytes!(
                    "../resources/icons/hicolor/scalable/apps/icon.svg"
                )))
                .version(env!("CARGO_PKG_VERSION"))
                .links([(fl!("repository"), env!("CARGO_PKG_REPOSITORY"))])
                .license(env!("CARGO_PKG_LICENSE")),
            drawer: None,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|config| Config::get_entry(&config).unwrap_or_else(|(_, config)| config))
                .unwrap_or_default(),
            settings_error: None,
            boards: Boards::default(),
            store: None,
            nav: nav_bar::Model::default(),
            page: Page::default(),
        };
        app.load();
        let task = app
            .core
            .main_window_id()
            .map_or_else(Task::none, |id| app.set_window_title(fl!("app-title"), id));
        (app, task)
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Message>> {
        if self.page.pending.is_some() {
            return Task::none();
        }
        if let Some(&id) = self.nav.data::<Id>(id) {
            let mut boards = self.boards.clone();
            boards.selected = Some(id);
            self.save(boards);
        }
        Task::none()
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        let keys = Default::default();
        let mut board = vec![menu::Item::Button(
            fl!("new-board"),
            Some(widget::icon::from_name("list-add-symbolic").handle()),
            MenuAction::NewBoard,
        )];
        board.push(menu::Item::Button(
            fl!("settings"),
            Some(widget::icon::from_name("preferences-system-symbolic").handle()),
            MenuAction::Settings,
        ));
        if self.boards.active().is_some() {
            board.extend([
                menu::Item::Button(
                    fl!("rename-board"),
                    Some(widget::icon::from_name("edit-symbolic").handle()),
                    MenuAction::RenameBoard,
                ),
                menu::Item::Button(
                    fl!("new-column"),
                    Some(widget::icon::from_name("list-add-symbolic").handle()),
                    MenuAction::NewColumn,
                ),
                menu::Item::Button(
                    fl!("archive"),
                    Some(widget::icon::from_name("mail-archive-symbolic").handle()),
                    MenuAction::Archive,
                ),
            ]);
        }
        vec![
            menu::bar(vec![
                menu::Tree::with_children(
                    Element::from(menu::root(fl!("board"))),
                    menu::items(&keys, board),
                ),
                menu::Tree::with_children(
                    Element::from(menu::root(fl!("help"))),
                    menu::items(
                        &keys,
                        vec![menu::Item::Button(
                            fl!("about"),
                            Some(widget::icon::from_name("help-about-symbolic").handle()),
                            MenuAction::About,
                        )],
                    ),
                ),
            ])
            .into(),
        ]
    }

    fn view(&self) -> Element<'_, Message> {
        self.page
            .view(self.boards.active(), &self.config.preferences)
            .map(Message::Board)
    }

    fn dialog(&self) -> Option<Element<'_, Message>> {
        self.page.dialog().map(|dialog| dialog.map(Message::Board))
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        match self.drawer {
            Some(AppDrawer::About) => Some(context_drawer::about(
                &self.about,
                |url| Message::OpenUrl(url.to_owned()),
                Message::CloseDrawer,
            )),
            Some(AppDrawer::Settings) => Some(settings::drawer(
                &self.config.preferences,
                self.settings_error.as_deref(),
                Message::Preference,
                Message::CloseDrawer,
            )),
            None => self
                .page
                .drawer(self.boards.active()?)
                .map(|drawer| drawer.map(Message::Board)),
        }
    }

    fn on_escape(&mut self) -> Task<cosmic::Action<Message>> {
        if !self.page.dismiss_dialog() {
            if self.drawer.is_some() {
                self.drawer = None;
            } else {
                self.page.dismiss_drawer();
            }
        }
        self.sync_drawer();
        Task::none()
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        let message = match message {
            Message::About | Message::Settings | Message::CloseDrawer => {
                self.drawer = match message {
                    Message::About => Some(AppDrawer::About),
                    Message::Settings => Some(AppDrawer::Settings),
                    _ => None,
                };
                self.sync_drawer();
                return Task::none();
            }
            Message::Preference(change) => {
                let mut next = self.config.clone();
                change.apply(&mut next.preferences);
                let save = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                    .and_then(|config| next.write_entry(&config));
                match save {
                    Ok(()) => {
                        self.config = next;
                        self.settings_error = None;
                    }
                    Err(error) => self.settings_error = Some(error.to_string()),
                }
                return Task::none();
            }
            Message::OpenUrl(url) => {
                if let Err(error) = open::that_detached(&url) {
                    eprintln!("{error}");
                }
                return Task::none();
            }
            Message::Board(message) => message,
        };
        if matches!(message, BoardMessage::Open(_) | BoardMessage::ShowArchive) {
            self.drawer = None;
        }
        let mut task = Task::none();
        if matches!(message, BoardMessage::Reload) {
            self.load();
        } else if self.store.is_some() {
            let mut next = self.boards.clone();
            if self.page.update(&mut next, message.clone()) {
                self.save(next);
            }
            task = self
                .page
                .focus(&message)
                .map(|message| cosmic::Action::App(Message::Board(message)));
            if matches!(message, BoardMessage::DragStarted)
                || (!self.page.has_card_bounds()
                    && matches!(
                        message,
                        BoardMessage::DragAt { .. } | BoardMessage::DropAt(..)
                    ))
            {
                task = task.chain(
                    cosmic::iced::runtime::task::widget(self.page.card_bounds_operation()).map(
                        |bounds| {
                            cosmic::Action::App(Message::Board(BoardMessage::CardBounds(bounds)))
                        },
                    ),
                );
            }
        }
        self.sync_drawer();
        task
    }
}

impl AppModel {
    fn sync_drawer(&mut self) {
        self.core.window.show_context = self.drawer.is_some() || self.page.show_drawer();
    }

    fn load(&mut self) {
        match Store::open().and_then(|store| Ok((store.load()?, store))) {
            Ok((mut boards, store)) => {
                if !self.config.preferences.reopen_last_board || boards.active().is_none() {
                    boards.selected = boards.boards.first().map(|board| board.id);
                }
                self.boards = boards;
                self.store = Some(store);
                self.page.error = None;
                self.page.update_drag_widgets(self.boards.active());
                self.update_nav();
            }
            Err(error) => self.page.error = Some(error.to_string()),
        }
    }

    fn save(&mut self, boards: Boards) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        match store.save(&boards) {
            Ok(()) => {
                let switched = self.boards.selected != boards.selected;
                let changed = switched
                    || self.boards.boards.len() != boards.boards.len()
                    || self.boards.active().map(|board| &board.name)
                        != boards.active().map(|board| &board.name);
                self.boards = boards;
                self.page.saved();
                if switched {
                    self.close_drawer();
                }
                self.page.update_drag_widgets(self.boards.active());
                if changed {
                    self.update_nav();
                }
                true
            }
            Err(error) => {
                self.page.error = Some(error.to_string());
                self.page.pending = Some(boards);
                false
            }
        }
    }

    fn close_drawer(&mut self) {
        self.page.reset();
        self.drawer = None;
        self.core.window.show_context = false;
    }

    fn update_nav(&mut self) {
        self.nav = nav_bar::Model::default();
        let icon =
            widget::icon::from_svg_bytes(include_bytes!("../resources/icons/board-symbolic.svg"))
                .symbolic(true);
        for Board { id, name, .. } in &self.boards.boards {
            let item = self
                .nav
                .insert()
                .text(name.clone())
                .icon(icon.clone().icon())
                .data(*id);
            if Some(*id) == self.boards.selected {
                item.activate();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    NewBoard,
    RenameBoard,
    NewColumn,
    Archive,
    About,
    Settings,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        Message::Board(match self {
            Self::NewBoard => BoardMessage::NewBoard,
            Self::RenameBoard => BoardMessage::Name(NameTarget::RenameBoard),
            Self::NewColumn => BoardMessage::Name(NameTarget::Column),
            Self::Archive => BoardMessage::ShowArchive,
            Self::About => return Message::About,
            Self::Settings => return Message::Settings,
        })
    }
}
