use std::{borrow::Cow, collections::HashMap};

use cosmic::Element;
use cosmic::iced::clipboard::{dnd::DndAction, mime::AsMimeTypes};
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
    pub destinations: HashMap<Target, widget::Id>,
}

impl Widgets {
    pub fn update(&mut self, board: Option<&Board>) {
        let mut next = Self::default();
        for column in board.into_iter().flat_map(|board| &board.columns) {
            let end = Target {
                column: column.id,
                before: None,
            };
            next.destinations.insert(
                end,
                self.destinations
                    .remove(&end)
                    .unwrap_or_else(widget::Id::unique),
            );
            for card in column.cards.iter().filter(|card| !card.archived) {
                let target = Target {
                    column: column.id,
                    before: Some(card.id),
                };
                next.sources.insert(
                    card.id,
                    self.sources
                        .remove(&card.id)
                        .unwrap_or_else(widget::Id::unique),
                );
                next.destinations.insert(
                    target,
                    self.destinations
                        .remove(&target)
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
        .on_cancel(Some(Message::DragEnded))
        .on_finish(Some(Message::DragEnded))
        .into()
}

impl Target {
    pub fn view<'a>(
        self,
        id: widget::Id,
        child: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        DndDestination::with_id(child, id, vec![MIME.into()])
            .action(DndAction::Move)
            .preferred_action(DndAction::Move)
            .on_enter(move |_, _, _| Message::DragEntered(self))
            .on_leave(move || Message::DragLeft(self))
            .on_finish(move |mime, data, action, _, _| {
                if mime == MIME
                    && action == DndAction::Move
                    && let Ok(card) = Id::from_slice(&data)
                {
                    return Message::Drop(card, self);
                }
                Message::DragEnded
            })
            .into()
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
