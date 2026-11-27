mod drawer;
mod view;

use std::sync::LazyLock;

use cosmic::iced::Task;
use cosmic::widget::date_input;
use cosmic::widget::{Id as WidgetId, text_editor, text_input};
use jiff::civil::Date;

use super::{
    Board, Boards, Card, ChecklistItem, Column, Id,
    drag::{self, Target},
};
use crate::fl;
use crate::widgets::chips::{self, Chips, Triggers};

static NAME_INPUT: LazyLock<WidgetId> = LazyLock::new(WidgetId::unique);
static CARD_INPUT: LazyLock<WidgetId> = LazyLock::new(WidgetId::unique);

fn date_options() -> date_input::Options<'static> {
    date_input::Options::auto()
}

fn parse_date(value: &str) -> Option<Date> {
    date_options().parse(value)
}

fn format_date(date: Date) -> String {
    date_options()
        .format(date)
        .unwrap_or_else(|| date.to_string())
}

#[derive(Default)]
pub struct Page {
    composer: Option<(Id, String)>,
    editor: Option<Editor>,
    name: Option<(NameTarget, String)>,
    search: String,
    show_filters: bool,
    label: Option<String>,
    due: DueFilter,
    archive: bool,
    drop_target: Option<Target>,
    drag_widgets: drag::Widgets,
    pub error: Option<String>,
    pub pending: Option<Boards>,
}

struct Editor {
    card: Card,
    date: String,
    date_state: date_input::DateInputState,
    description: text_editor::Content<cosmic::Renderer>,
    item: String,
    labels: Chips,
    editing_title: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DueFilter {
    #[default]
    All,
    Upcoming,
    Overdue,
    None,
}

impl DueFilter {
    const OPTIONS: [Self; 4] = [Self::All, Self::Upcoming, Self::Overdue, Self::None];

    fn label(self) -> String {
        match self {
            Self::All => fl!("any-date"),
            Self::Upcoming => fl!("upcoming"),
            Self::Overdue => fl!("past-due"),
            Self::None => fl!("no-date"),
        }
    }

    fn matches(self, card: &Card, complete: bool, today: Date) -> bool {
        match self {
            Self::All => true,
            Self::Upcoming => {
                !complete
                    && card.due.is_some_and(|date| {
                        date >= today && date.since(today).is_ok_and(|span| span.get_days() <= 7)
                    })
            }
            Self::Overdue => !complete && card.due.is_some_and(|date| date < today),
            Self::None => card.due.is_none(),
        }
    }
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
    CancelCard,
    Open(Id),
    Close,
    EditTitle(String),
    EditTitleActive(bool),
    Description(text_editor::Action),
    Calendar(date_input::Event),
    Labels(chips::Event),
    Item(String),
    AddItem,
    ToggleItem(Id, bool),
    RemoveItem(Id),
    SaveCard,
    MoveToColumn(Id, Id),
    DragEntered(Target),
    DragLeft(Target),
    DragEnded,
    Drop(Id, Target),
    Shift(Id, Id, bool),
    ShiftColumn(Id, bool),
    CompleteColumn(Id, bool),
    Archive(Id, bool),
    Search(String),
    ToggleFilters,
    FilterLabel(Option<String>),
    FilterDue(DueFilter),
    ShowArchive,
    Reload,
    Retry,
    Discard,
}

impl Page {
    pub fn update_drag_widgets(&mut self, board: Option<&Board>) {
        self.drag_widgets.update(board);
    }

    pub fn focus(&self, message: &Message) -> Task<Message> {
        match message {
            Message::NewBoard | Message::Name(_) if self.name.is_some() => {
                text_input::focus(NAME_INPUT.clone())
            }
            Message::Compose(id)
                if self
                    .composer
                    .as_ref()
                    .is_some_and(|(column, _)| column == id) =>
            {
                text_input::focus(CARD_INPUT.clone())
            }
            _ => Task::none(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn dismiss_dialog(&mut self) -> bool {
        self.name.take().is_some()
    }

    pub fn dismiss_drawer(&mut self) {
        if self
            .editor
            .as_mut()
            .is_some_and(|editor| editor.date_state.close_calendar())
        {
            return;
        }
        self.close_drawer();
    }

    fn close_drawer(&mut self) {
        self.editor = None;
        self.archive = false;
    }

    pub fn show_drawer(&self) -> bool {
        self.archive || self.editor.is_some()
    }

    pub fn saved(&mut self) {
        self.error = None;
        self.pending = None;
    }

    pub fn update(&mut self, boards: &mut Boards, message: Message) -> bool {
        if matches!(message, Message::Close) {
            self.close_drawer();
            return false;
        }

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
            Message::CancelCard => self.composer = None,
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
                        date: card.due.map_or_else(String::new, format_date),
                        date_state: date_input::DateInputState::default(),
                        description: text_editor::Content::with_text(&card.description),
                        item: String::new(),
                        labels: Chips::new(
                            card.labels.clone(),
                            Triggers {
                                separators: &[','],
                                enter: true,
                            },
                        ),
                        editing_title: false,
                    });
                }
            }
            Message::ShowArchive => {
                self.editor = None;
                self.archive = true;
            }
            Message::Search(value) => self.search = value,
            Message::ToggleFilters => self.show_filters = !self.show_filters,
            Message::FilterLabel(value) => self.label = value,
            Message::FilterDue(value) => self.due = value,
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
            Message::MoveToColumn(id, column) => {
                let Some(board) = boards.active_mut() else {
                    return false;
                };
                return board
                    .card(id)
                    .is_some_and(|(source, _)| source.id != column)
                    && board.move_card(id, column, None);
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
            Message::EditTitleActive(active) => editor.editing_title = active,
            Message::Description(action) => editor.description.perform(action),
            Message::Calendar(event) => editor.update_calendar(event),
            Message::Labels(event) => editor.labels.update(event),
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
                editor.card.description = editor.description.text();
                editor.card.due = parse_date(editor.date.trim());
                editor.labels.commit_draft();
                editor.card.labels = editor.labels.values().to_vec();
                *card = editor.card.clone();
                return true;
            }
            _ => {}
        }
        false
    }
}

impl Editor {
    fn update_calendar(&mut self, event: date_input::Event) {
        let options = date_options();
        self.date_state.update(event, &mut self.date, &options);
    }

    fn valid(&self) -> bool {
        !self.card.title.trim().is_empty()
            && (self.date.trim().is_empty() || parse_date(self.date.trim()).is_some())
    }
}
