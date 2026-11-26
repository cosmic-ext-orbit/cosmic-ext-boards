use cosmic::Element;
use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::iced::{Alignment, Length, widget::scrollable};
use cosmic::widget::{self, button, checkbox, text, text_input};
use jiff::{Zoned, civil::Date};

use super::{
    Board, Boards, Card, ChecklistItem, Column, Id,
    drag::{self, Target},
};
use crate::fl;

#[derive(Default)]
pub struct Page {
    composer: Option<(Id, String)>,
    editor: Option<Editor>,
    name: Option<(NameTarget, String)>,
    search: String,
    archive: bool,
    drop_target: Option<Target>,
    drag_widgets: drag::Widgets,
    pub error: Option<String>,
    pub pending: Option<Boards>,
}

struct Editor {
    card: Card,
    date: String,
    item: String,
    labels: String,
}

#[derive(Clone, Copy, Debug)]
pub enum NameTarget {
    Board,
    RenameBoard,
    Column,
    RenameColumn(Id),
}

#[derive(Clone, Debug)]
pub enum Message {
    NewBoard,
    Name(NameTarget),
    NameChanged(String),
    SaveName,
    CancelName,
    Compose(Id),
    Title(String),
    AddCard,
    Open(Id),
    Close,
    EditTitle(String),
    Description(String),
    Date(String),
    Labels(String),
    Item(String),
    AddItem,
    ToggleItem(Id, bool),
    RemoveItem(Id),
    SaveCard,
    Move(Id, Id, Option<Id>),
    DragEntered(Target),
    DragLeft(Target),
    DragEnded,
    Drop(Id, Target),
    Shift(Id, Id, bool),
    ShiftColumn(Id, bool),
    CompleteColumn(Id, bool),
    Archive(Id, bool),
    Search(String),
    ShowArchive,
    Reload,
    Retry,
    Discard,
}

impl Page {
    pub fn update_drag_widgets(&mut self, board: Option<&Board>) {
        self.drag_widgets.update(board);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn show_drawer(&self) -> bool {
        self.archive || self.editor.is_some()
    }

    pub fn saved(&mut self) {
        self.error = None;
        self.pending = None;
    }

    pub fn update(&mut self, boards: &mut Boards, message: Message) -> bool {
        if self.pending.is_some() {
            match message {
                Message::Retry => {
                    if let Some(next) = &self.pending {
                        *boards = next.clone();
                        return true;
                    }
                }
                Message::Discard => self.saved(),
                _ => {}
            }
            return false;
        }
        match message {
            Message::NewBoard | Message::Name(NameTarget::Board) => {
                self.name = Some((NameTarget::Board, String::new()));
            }
            Message::Name(target) => {
                let Some(board) = boards.active() else {
                    return false;
                };
                let value = match target {
                    NameTarget::RenameBoard => board.name.clone(),
                    NameTarget::RenameColumn(id) => board
                        .columns
                        .iter()
                        .find(|column| column.id == id)
                        .map_or_else(String::new, |column| column.name.clone()),
                    _ => String::new(),
                };
                self.name = Some((target, value));
            }
            Message::NameChanged(value) => {
                if let Some((_, name)) = &mut self.name {
                    *name = value;
                }
            }
            Message::CancelName => self.name = None,
            Message::SaveName => {
                let Some((target, name)) = self.name.take() else {
                    return false;
                };
                let name = name.trim().to_owned();
                if name.is_empty() {
                    return false;
                }
                if matches!(target, NameTarget::Board) {
                    let board = Board::new(
                        name,
                        vec![
                            Column::new(fl!("todo"), false),
                            Column::new(fl!("in-progress"), false),
                            Column::new(fl!("done"), true),
                        ],
                    );
                    boards.selected = Some(board.id);
                    boards.boards.push(board);
                    self.reset();
                    return true;
                }
                let Some(board) = boards.active_mut() else {
                    return false;
                };
                match target {
                    NameTarget::RenameBoard => board.name = name,
                    NameTarget::Column => board.columns.push(Column::new(name, false)),
                    NameTarget::RenameColumn(id) => {
                        let Some(column) = board.columns.iter_mut().find(|column| column.id == id)
                        else {
                            return false;
                        };
                        column.name = name;
                    }
                    NameTarget::Board => unreachable!(),
                }
                return true;
            }
            Message::Compose(id) => self.composer = Some((id, String::new())),
            Message::Title(title) => {
                if let Some((_, value)) = &mut self.composer {
                    *value = title;
                }
            }
            Message::AddCard => {
                let Some((id, title)) = &mut self.composer else {
                    return false;
                };
                if title.trim().is_empty() {
                    return false;
                }
                let Some(column) = boards
                    .active_mut()
                    .and_then(|board| board.columns.iter_mut().find(|column| column.id == *id))
                else {
                    return false;
                };
                column.cards.push(Card::new(title.trim().to_owned()));
                title.clear();
                return true;
            }
            Message::Open(id) => {
                if let Some((_, card)) = boards.active().and_then(|board| board.card(id)) {
                    self.archive = false;
                    self.editor = Some(Editor {
                        card: card.clone(),
                        date: card.due.map_or_else(String::new, |date| date.to_string()),
                        item: String::new(),
                        labels: card.labels.join(", "),
                    });
                }
            }
            Message::Close => {
                self.editor = None;
                self.archive = false;
            }
            Message::ShowArchive => {
                self.editor = None;
                self.archive = true;
            }
            Message::Search(value) => self.search = value,
            Message::DragEntered(target) => self.drop_target = Some(target),
            Message::DragLeft(target) => {
                if self.drop_target == Some(target) {
                    self.drop_target = None;
                }
            }
            Message::DragEnded => self.drop_target = None,
            Message::Drop(id, target) => {
                self.drop_target = None;
                return boards
                    .active_mut()
                    .is_some_and(|board| board.move_card(id, target.column, target.before));
            }
            Message::Move(id, column, before) => {
                return boards
                    .active_mut()
                    .is_some_and(|board| board.move_card(id, column, before));
            }
            Message::Shift(id, column, down) => {
                let Some(board) = boards.active_mut() else {
                    return false;
                };
                let Some(target) = board.columns.iter().find(|entry| entry.id == column) else {
                    return false;
                };
                let cards: Vec<_> = target
                    .cards
                    .iter()
                    .filter(|card| !card.archived)
                    .map(|card| card.id)
                    .collect();
                let Some(index) = cards.iter().position(|card| *card == id) else {
                    return false;
                };
                let before = if down {
                    if index + 1 == cards.len() {
                        return false;
                    }
                    cards.get(index + 2).copied()
                } else {
                    let Some(index) = index.checked_sub(1) else {
                        return false;
                    };
                    Some(cards[index])
                };
                return board.move_card(id, column, before);
            }
            Message::ShiftColumn(id, right) => {
                let Some(board) = boards.active_mut() else {
                    return false;
                };
                let Some(index) = board.columns.iter().position(|column| column.id == id) else {
                    return false;
                };
                let Some(target) = (if right {
                    Some(index + 1)
                } else {
                    index.checked_sub(1)
                }) else {
                    return false;
                };
                if target >= board.columns.len() {
                    return false;
                }
                board.columns.swap(index, target);
                return true;
            }
            Message::CompleteColumn(id, complete) => {
                let Some(column) = boards
                    .active_mut()
                    .and_then(|board| board.columns.iter_mut().find(|column| column.id == id))
                else {
                    return false;
                };
                column.complete = complete;
                return true;
            }
            Message::Archive(id, archived) => {
                let Some(card) = boards.active_mut().and_then(|board| board.card_mut(id)) else {
                    return false;
                };
                card.archived = archived;
                self.editor = None;
                return true;
            }
            message => return self.edit(boards, message),
        }
        false
    }

    fn edit(&mut self, boards: &mut Boards, message: Message) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        match message {
            Message::EditTitle(value) => editor.card.title = value,
            Message::Description(value) => editor.card.description = value,
            Message::Date(value) => editor.date = value,
            Message::Labels(value) => editor.labels = value,
            Message::Item(value) => editor.item = value,
            Message::AddItem if !editor.item.trim().is_empty() => {
                editor.card.checklist.push(ChecklistItem {
                    id: Id::new_v4(),
                    text: editor.item.trim().to_owned(),
                    checked: false,
                });
                editor.item.clear();
            }
            Message::ToggleItem(id, checked) => {
                if let Some(item) = editor.card.checklist.iter_mut().find(|item| item.id == id) {
                    item.checked = checked;
                }
            }
            Message::RemoveItem(id) => editor.card.checklist.retain(|item| item.id != id),
            Message::SaveCard if editor.valid() => {
                let Some(card) = boards
                    .active_mut()
                    .and_then(|board| board.card_mut(editor.card.id))
                else {
                    return false;
                };
                editor.card.title = editor.card.title.trim().into();
                editor.card.due = editor.date.trim().parse().ok();
                editor.card.labels = editor
                    .labels
                    .split(',')
                    .map(str::trim)
                    .filter(|label| !label.is_empty())
                    .fold(Vec::new(), |mut labels, label| {
                        if !labels.iter().any(|existing| existing == label) {
                            labels.push(label.to_owned());
                        }
                        labels
                    });
                *card = editor.card.clone();
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn dialog(&self) -> Option<Element<'_, Message>> {
        let (target, value) = self.name.as_ref()?;
        let title = match target {
            NameTarget::Board => fl!("new-board"),
            NameTarget::RenameBoard => fl!("rename-board"),
            NameTarget::Column => fl!("new-column"),
            NameTarget::RenameColumn(_) => fl!("rename-column"),
        };
        let save = button::suggested(fl!("save"));
        let mut input = text_input(fl!("name"), value).on_input(Message::NameChanged);
        let save = if value.trim().is_empty() {
            save
        } else {
            input = input.on_submit(|_| Message::SaveName);
            save.on_press(Message::SaveName)
        };
        Some(
            widget::dialog()
                .title(title)
                .control(input)
                .primary_action(save)
                .secondary_action(button::standard(fl!("cancel")).on_press(Message::CancelName))
                .into(),
        )
    }

    pub fn view<'a>(&'a self, board: Option<&'a Board>) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();
        let mut content = widget::column::with_capacity(3)
            .spacing(spacing.space_m)
            .padding(spacing.space_m);
        if let Some(error) = &self.error {
            let mut status = widget::row::with_capacity(3)
                .spacing(spacing.space_xs)
                .push(text::body(fl!("storage-error", error = error.as_str())));
            if self.pending.is_some() {
                status = status
                    .push(button::standard(fl!("retry")).on_press(Message::Retry))
                    .push(button::text(fl!("discard")).on_press(Message::Discard));
            } else {
                status = status.push(button::standard(fl!("retry")).on_press(Message::Reload));
            }
            content = content.push(status);
        }
        let Some(board) = board else {
            return content.push(text::title2(fl!("no-boards"))).into();
        };
        let actions = widget::row::with_capacity(4)
            .spacing(spacing.space_xs)
            .push(
                button::text(fl!("rename-board")).on_press(Message::Name(NameTarget::RenameBoard)),
            )
            .push(button::text(fl!("new-column")).on_press(Message::Name(NameTarget::Column)))
            .push(button::text(fl!("archive")).on_press(Message::ShowArchive))
            .push(text_input(fl!("search"), &self.search).on_input(Message::Search));
        let today = Zoned::now().date();
        let columns = board.columns.iter().fold(
            widget::row::with_capacity(board.columns.len())
                .spacing(spacing.space_s)
                .height(Length::Fill),
            |row, column| row.push(self.column(board, column, today)),
        );
        content
            .push(text::title1(&board.name))
            .push(actions)
            .push(
                widget::scrollable(columns)
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default(),
                    ))
                    .height(Length::Fill),
            )
            .height(Length::Fill)
            .into()
    }

    fn column<'a>(
        &'a self,
        board: &'a Board,
        column: &'a Column,
        today: Date,
    ) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();
        let index = board
            .columns
            .iter()
            .position(|entry| entry.id == column.id)
            .unwrap();
        let header = widget::row::with_capacity(3)
            .spacing(spacing.space_xxs)
            .align_y(Alignment::Center)
            .push(
                button::text(&column.name)
                    .on_press(Message::Name(NameTarget::RenameColumn(column.id))),
            )
            .push(
                button::text("←")
                    .on_press_maybe((index > 0).then_some(Message::ShiftColumn(column.id, false))),
            )
            .push(button::text("→").on_press_maybe(
                (index + 1 < board.columns.len()).then_some(Message::ShiftColumn(column.id, true)),
            ));
        let query = self.search.trim().to_lowercase();
        let cards = column
            .cards
            .iter()
            .filter(|card| {
                !card.archived
                    && (query.is_empty()
                        || card.title.to_lowercase().contains(&query)
                        || card.description.to_lowercase().contains(&query)
                        || card
                            .labels
                            .iter()
                            .any(|label| label.to_lowercase().contains(&query)))
            })
            .fold(
                widget::column::with_capacity(column.cards.len()).spacing(spacing.space_xxs),
                |list, card| {
                    let mut details = Vec::new();
                    if !card.checklist.is_empty() {
                        let done = card.checklist.iter().filter(|item| item.checked).count();
                        details.push(format!("{done}/{}", card.checklist.len()));
                    }
                    if let Some(due) = card.due {
                        details.push(if due < today && !column.complete {
                            fl!("overdue", date = due.to_string())
                        } else {
                            due.to_string()
                        });
                    }
                    if !card.labels.is_empty() {
                        details.push(card.labels.join(", "));
                    }
                    let label = widget::column::with_capacity(2).push(text::body(&card.title));
                    let label = if details.is_empty() {
                        label
                    } else {
                        label.push(text::caption(details.join(" · ")))
                    };
                    let handle = drag::handle(
                        card.id,
                        self.drag_widgets.sources[&card.id].clone(),
                        widget::container(
                            widget::icon::from_name("list-drag-handle-symbolic")
                                .size(16)
                                .icon(),
                        )
                        .padding(spacing.space_xxs),
                    );
                    let row = widget::row::with_capacity(2)
                        .align_y(Alignment::Center)
                        .push(handle)
                        .push(
                            button::custom(label)
                                .width(Length::Fill)
                                .on_press(Message::Open(card.id)),
                        );
                    let target = Target {
                        column: column.id,
                        before: Some(card.id),
                    };
                    list.push(
                        target.view(
                            self.drag_widgets.destinations[&target].clone(),
                            widget::column::with_capacity(2)
                                .push(self.insertion_line(target))
                                .push(row),
                        ),
                    )
                },
            );
        let end = Target {
            column: column.id,
            before: None,
        };
        let cards = cards.push(
            end.view(
                self.drag_widgets.destinations[&end].clone(),
                widget::column::with_capacity(2)
                    .push(self.insertion_line(end))
                    .push(cosmic::iced::widget::space().width(Length::Fill).height(32)),
            ),
        );
        let add: Element<_> = match &self.composer {
            Some((id, value)) if *id == column.id => text_input(fl!("card-title"), value)
                .on_input(Message::Title)
                .on_submit(|_| Message::AddCard)
                .into(),
            _ => button::standard(fl!("add-card"))
                .on_press(Message::Compose(column.id))
                .into(),
        };
        widget::container(
            widget::column::with_capacity(4)
                .spacing(spacing.space_xs)
                .push(header)
                .push(
                    checkbox(column.complete)
                        .label(fl!("completed-column"))
                        .on_toggle(move |complete| Message::CompleteColumn(column.id, complete)),
                )
                .push(widget::scrollable(cards).height(Length::Fill))
                .push(add),
        )
        .padding(spacing.space_s)
        .width(260)
        .height(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
    }

    fn insertion_line(&self, target: Target) -> Element<'_, Message> {
        let active = self.drop_target == Some(target);
        widget::container(cosmic::iced::widget::space())
            .width(Length::Fill)
            .height(3)
            .style(
                move |theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                    background: active.then(|| {
                        cosmic::iced::Background::Color(theme.cosmic().accent_color().into())
                    }),
                    ..Default::default()
                },
            )
            .into()
    }

    pub fn drawer<'a>(&'a self, board: &'a Board) -> Option<ContextDrawer<'a, Message>> {
        let spacing = cosmic::theme::spacing();
        if self.archive {
            let mut content = widget::column::with_capacity(4).spacing(spacing.space_xs);
            for card in board
                .columns
                .iter()
                .flat_map(|column| &column.cards)
                .filter(|card| card.archived)
            {
                content = content.push(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_s)
                        .push(text::body(&card.title))
                        .push(
                            button::text(fl!("restore")).on_press(Message::Archive(card.id, false)),
                        ),
                );
            }
            if !board
                .columns
                .iter()
                .flat_map(|column| &column.cards)
                .any(|card| card.archived)
            {
                content = content.push(text::body(fl!("archive-empty")));
            }
            return Some(
                context_drawer::context_drawer(content, Message::Close).title(fl!("archive")),
            );
        }
        let editor = self.editor.as_ref()?;
        let id = editor.card.id;
        let (column, card) = board.card(id)?;
        let destinations: Vec<_> = board
            .columns
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        let selected = board.columns.iter().position(|entry| entry.id == column.id);
        let column_ids: Vec<_> = board.columns.iter().map(|entry| entry.id).collect();
        let items = editor.card.checklist.iter().fold(
            widget::column::with_capacity(editor.card.checklist.len()).spacing(spacing.space_xxs),
            |list, item| {
                let item_id = item.id;
                list.push(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_xxs)
                        .push(
                            checkbox(item.checked)
                                .label(&item.text)
                                .on_toggle(move |checked| Message::ToggleItem(item_id, checked)),
                        )
                        .push(button::text(fl!("remove")).on_press(Message::RemoveItem(item_id))),
                )
            },
        );
        let content = widget::column::with_capacity(13)
            .spacing(spacing.space_xs)
            .push(text::body(fl!("status")))
            .push(widget::dropdown(destinations, selected, move |index| {
                Message::Move(id, column_ids[index], None)
            }))
            .push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_xs)
                    .push(
                        button::text(fl!("move-up")).on_press(Message::Shift(id, column.id, false)),
                    )
                    .push(
                        button::text(fl!("move-down"))
                            .on_press(Message::Shift(id, column.id, true)),
                    ),
            )
            .push(text::body(fl!("card-title")))
            .push(text_input(fl!("card-title"), &editor.card.title).on_input(Message::EditTitle))
            .push(text::body(fl!("description")))
            .push(
                text_input(fl!("description"), &editor.card.description)
                    .on_input(Message::Description),
            )
            .push(text::body(fl!("due-date")))
            .push(text_input("YYYY-MM-DD", &editor.date).on_input(Message::Date))
            .push(text::body(fl!("labels")))
            .push(text_input(fl!("labels"), &editor.labels).on_input(Message::Labels))
            .push(text::title4(fl!("checklist")))
            .push(items)
            .push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(
                        text_input(fl!("checklist-item"), &editor.item)
                            .on_input(Message::Item)
                            .on_submit(|_| Message::AddItem),
                    )
                    .push(button::standard(fl!("add")).on_press(Message::AddItem)),
            );
        let footer = widget::row::with_capacity(2)
            .spacing(spacing.space_xs)
            .push(
                button::suggested(fl!("save"))
                    .on_press_maybe(editor.valid().then_some(Message::SaveCard)),
            )
            .push(button::text(fl!("archive")).on_press(Message::Archive(id, true)));
        Some(
            context_drawer::context_drawer(content, Message::Close)
                .title(&card.title)
                .footer(footer),
        )
    }
}

impl Editor {
    fn valid(&self) -> bool {
        !self.card.title.trim().is_empty()
            && (self.date.trim().is_empty() || self.date.trim().parse::<Date>().is_ok())
    }
}
