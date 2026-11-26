// SPDX-License-Identifier: GPL-3.0-only

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, nav_bar};

use crate::boards::{Board, Boards, Id, Store};
use crate::fl;

pub struct AppModel {
    core: cosmic::Core,
    about: About,
    show_about: bool,
    boards: Boards,
    store: Option<Store>,
    nav: nav_bar::Model,
    page: crate::boards::Page,
}

#[derive(Clone, Debug)]
pub enum Message {
    Board(crate::boards::Message),
    About,
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
            show_about: false,
            boards: Boards::default(),
            store: None,
            nav: nav_bar::Model::default(),
            page: crate::boards::Page::default(),
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
            if self.save(boards) {
                self.close_drawer();
            }
        }
        Task::none()
    }

    fn header_end(&self) -> Vec<Element<'_, Message>> {
        vec![
            widget::button::suggested(fl!("new-board"))
                .on_press(Message::Board(crate::boards::Message::NewBoard))
                .into(),
        ]
    }

    fn view(&self) -> Element<'_, Message> {
        self.page.view(self.boards.active()).map(Message::Board)
    }

    fn dialog(&self) -> Option<Element<'_, Message>> {
        self.page.dialog().map(|dialog| dialog.map(Message::Board))
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        if self.show_about {
            return Some(context_drawer::about(
                &self.about,
                |url| Message::OpenUrl(url.to_owned()),
                Message::About,
            ));
        }
        self.page
            .drawer(self.boards.active()?)
            .map(|drawer| drawer.map(Message::Board))
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        let message = match message {
            Message::About => {
                self.show_about = !self.show_about;
                self.core.window.show_context = self.show_about || self.page.show_drawer();
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
        if matches!(
            message,
            crate::boards::Message::Open(_) | crate::boards::Message::ShowArchive
        ) {
            self.show_about = false;
        }
        if matches!(message, crate::boards::Message::Reload) {
            self.load();
        } else if self.store.is_some() {
            let mut next = self.boards.clone();
            if self.page.update(&mut next, message) {
                self.save(next);
            }
        }
        self.core.window.show_context = self.show_about || self.page.show_drawer();
        Task::none()
    }
}

impl AppModel {
    fn load(&mut self) {
        match Store::open().and_then(|store| Ok((store.load()?, store))) {
            Ok((mut boards, store)) => {
                if boards.active().is_none() {
                    boards.selected = boards.boards.first().map(|board| board.id);
                }
                self.boards = boards;
                self.store = Some(store);
                self.page.error = None;
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
        self.show_about = false;
        self.core.window.show_context = false;
    }

    fn update_nav(&mut self) {
        self.nav = nav_bar::Model::default();
        for Board { id, name, .. } in &self.boards.boards {
            let item = self.nav.insert().text(name.clone()).data(*id);
            if Some(*id) == self.boards.selected {
                item.activate();
            }
        }
    }
}
