use cosmic::Element;
use cosmic::iced::{Alignment, Length, widget::stack};
use cosmic::widget::{self, calendar::CalendarModel, popover, text_input};
use jiff::civil::{Date, Weekday};

use super::action;
use crate::fl;

#[derive(Clone, Debug)]
pub enum Event {
    Input(String),
    Clear,
    Toggle,
    Close,
    PreviousMonth,
    NextMonth,
    Select(Date),
}

pub fn date_input<'a, Message: Clone + 'static>(
    value: &'a str,
    model: Option<&'a CalendarModel>,
    on_event: fn(Event) -> Message,
) -> Element<'a, Message> {
    let mut calendar = popover(
        action("x-office-calendar-symbolic", fl!("choose-date"))
            .extra_small()
            .on_press(on_event(Event::Toggle)),
    )
    .position(popover::Position::Bottom)
    .on_close(on_event(Event::Close));
    if let Some(model) = model {
        calendar = calendar.popup(
            widget::container(widget::calendar(
                model,
                move |date| on_event(Event::Select(date)),
                move || on_event(Event::PreviousMonth),
                move || on_event(Event::NextMonth),
                Weekday::Monday,
            ))
            .padding(cosmic::theme::spacing().space_xs)
            .class(cosmic::theme::Container::Card),
        );
    }

    let mut input = text_input("YYYY-MM-DD", value)
        .width(Length::Fill)
        .trailing_icon(calendar.into())
        .on_input(move |value| on_event(Event::Input(value)));

    if !value.is_empty() {
        input = input.trailing_icon(
            action("edit-clear-symbolic", fl!("clear-date"))
                .extra_small()
                .on_press(on_event(Event::Clear))
                .into(),
        );
    }

    input.into()
}
