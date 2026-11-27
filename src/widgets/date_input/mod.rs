mod format;
mod locale;
mod options;
mod state;

#[allow(unused_imports)]
pub use format::Format;
pub use options::Options;
pub use state::DateInputState;

use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, calendar::CalendarModel, popover};
use jiff::civil::{Date, Weekday};

use self::format::sanitize;

#[derive(Clone, Debug)]
pub enum Event {
    Input(String),
    Edit(bool),
    Focus,
    Unfocus,
    Close,
    PreviousMonth,
    NextMonth,
    Select(Date),
}

pub fn date_input<'a, Message: Clone + 'static>(
    value: &'a str,
    editing: bool,
    model: Option<&'a CalendarModel>,
    options: Options<'a>,
    on_event: fn(Event) -> Message,
) -> Element<'a, Message> {
    let placeholder = options.placeholder();
    let message = options.validation_message(value);
    let formats = options.formats();

    let mut input = widget::editable_input(placeholder, value, editing, move |editing| {
        on_event(Event::Edit(editing))
    })
    .width(Length::Fill)
    .on_focus(on_event(Event::Focus))
    .on_unfocus(on_event(Event::Unfocus))
    .on_input(move |input| on_event(Event::Input(sanitize(&input, formats))))
    .on_submit(move |_| on_event(Event::Edit(false)));

    if let Some(message) = message {
        input = input.error(message);
    }

    let mut input = popover(input)
        .position(popover::Position::Bottom)
        .on_close(on_event(Event::Close));
    if let Some(model) = model {
        input = input.popup(
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

    input.into()
}
