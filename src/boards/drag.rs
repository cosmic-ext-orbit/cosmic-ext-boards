use std::{any::Any, borrow::Cow, collections::HashMap};

use cosmic::Element;
use cosmic::iced::Rectangle;
use cosmic::iced::clipboard::{dnd::DndAction, mime::AsMimeTypes};
use cosmic::iced::core::widget::{Operation, operation::Outcome};
use cosmic::widget::{self, DndDestination, DndSource};

use super::{Board, Id, Message};

const MIME: &str = "application/x-dev.edfloreshz.boards-card";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub column: Id,
    pub before: Option<Id>,
}

#[derive(Default)]
pub(super) struct Widgets {
    pub sources: HashMap<Id, widget::Id>,
    pub destinations: HashMap<Id, widget::Id>,
}

impl Widgets {
    pub fn update(&mut self, board: Option<&Board>) {
        let mut next = Self::default();
        for column in board.into_iter().flat_map(|board| &board.columns) {
            next.destinations.insert(
                column.id,
                self.destinations
                    .remove(&column.id)
                    .unwrap_or_else(widget::Id::unique),
            );
            for card in column.cards.iter().filter(|card| !card.archived) {
                next.sources.insert(
                    card.id,
                    self.sources
                        .remove(&card.id)
                        .unwrap_or_else(widget::Id::unique),
                );
            }
        }
        *self = next;
    }
}

pub(super) fn source<'a>(
    card: Id,
    id: widget::Id,
    child: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    DndSource::with_id(child, id)
        .action(DndAction::Move)
        .drag_content(move || Payload(card))
        .on_start(Some(Message::DragStarted))
        .on_cancel(Some(Message::DragEnded))
        .on_finish(Some(Message::DragEnded))
        .into()
}

pub(super) fn column<'a>(
    id: widget::Id,
    column: Id,
    child: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    DndDestination::with_id(child, id, vec![MIME.into()])
        .action(DndAction::Move)
        .preferred_action(DndAction::Move)
        .on_enter(move |_, y, _| Message::DragAt { column, y })
        .on_motion(move |_, y| Message::DragAt { column, y })
        .on_leave(move || Message::DragLeft(column))
        .on_finish(move |mime, data, action, _, y| {
            if mime == MIME
                && action == DndAction::Move
                && let Ok(card) = Id::from_slice(&data)
            {
                return Message::DropAt(card, column, y);
            }
            Message::DragEnded
        })
        .into()
}

pub(super) fn card_bounds(ids: HashMap<Id, widget::Id>) -> impl Operation<HashMap<Id, Rectangle>> {
    struct Bounds {
        cards: HashMap<widget::Id, Id>,
        bounds: HashMap<Id, Rectangle>,
    }

    impl Operation<HashMap<Id, Rectangle>> for Bounds {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<HashMap<Id, Rectangle>>)) {
            operate(self);
        }

        fn custom(&mut self, id: Option<&widget::Id>, bounds: Rectangle, _: &mut dyn Any) {
            if let Some(card) = id.and_then(|id| self.cards.get(id)) {
                self.bounds.insert(*card, bounds);
            }
        }

        fn finish(&self) -> Outcome<HashMap<Id, Rectangle>> {
            Outcome::Some(self.bounds.clone())
        }
    }

    Bounds {
        cards: ids.into_iter().map(|(card, id)| (id, card)).collect(),
        bounds: HashMap::new(),
    }
}

struct Payload(Id);

impl AsMimeTypes for Payload {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![MIME.into()])
    }

    fn as_bytes(&self, mime: &str) -> Option<Cow<'static, [u8]>> {
        (mime == MIME).then(|| Cow::Owned(self.0.as_bytes().to_vec()))
    }
}
