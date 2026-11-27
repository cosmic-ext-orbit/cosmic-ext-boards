use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, calendar::CalendarModel, popover};
use jiff::{
    civil::{Date, Weekday},
    fmt::strtime,
};

use crate::fl;

enum Parse<'a> {
    Iso,
    Strftime(&'a str),
    Custom(Box<dyn Fn(&str) -> Option<Date> + 'a>),
}

impl Parse<'_> {
    fn parse(&self, value: &str) -> Option<Date> {
        match self {
            Self::Iso => value.parse().ok(),
            Self::Strftime(format) => strtime::parse(format, value).ok()?.to_date().ok(),
            Self::Custom(parse) => parse(value),
        }
    }
}

/// A date format accepted by [`date_input`].
pub struct Format<'a> {
    placeholder: &'a str,
    parser: Parse<'a>,
}

impl<'a> Format<'a> {
    /// The ISO 8601 calendar date format, `YYYY-MM-DD`.
    pub const ISO: Self = Self {
        placeholder: "YYYY-MM-DD",
        parser: Parse::Iso,
    };

    /// A `strftime`-style format, such as `%d/%m/%Y`.
    pub fn strftime(format: &'a str, placeholder: &'a str) -> Self {
        Self {
            placeholder,
            parser: Parse::Strftime(format),
        }
    }

    /// A caller-provided parser for formats that `strftime` cannot express.
    pub fn custom(placeholder: &'a str, parse: impl Fn(&str) -> Option<Date> + 'a) -> Self {
        Self {
            placeholder,
            parser: Parse::Custom(Box::new(parse)),
        }
    }

    fn is_iso(&self) -> bool {
        matches!(self.parser, Parse::Iso)
    }
}

/// Configuration for [`date_input`].
pub struct Options<'a> {
    formats: &'a [Format<'a>],
    validate: Option<Box<dyn Fn(Date) -> Result<(), String> + 'a>>,
}

impl<'a> Options<'a> {
    /// Accepts the given formats, in any order.
    pub fn new(formats: &'a [Format<'a>]) -> Self {
        Self {
            formats,
            validate: None,
        }
    }

    /// Validates parsed dates, using the returned message for invalid dates.
    pub fn validate(mut self, validate: impl Fn(Date) -> Result<(), String> + 'a) -> Self {
        self.validate = Some(Box::new(validate));
        self
    }

    fn placeholder(&self) -> String {
        if self.formats.is_empty() {
            "YYYY-MM-DD".to_owned()
        } else {
            self.formats
                .iter()
                .map(|format| format.placeholder)
                .collect::<Vec<_>>()
                .join(" / ")
        }
    }

    /// Returns the validation error for a completed date, if any.
    pub fn validation_message(&self, value: &str) -> Option<String> {
        if value.is_empty() || value.chars().count() < min_placeholder_len(self.formats) {
            return None;
        }
        self.validate_result(value).err()
    }

    fn parse(&self, value: &str) -> Option<Date> {
        self.validate_result(value).ok()
    }

    fn validate_result(&self, value: &str) -> Result<Date, String> {
        for format in self.formats {
            if let Some(date) = format.parser.parse(value) {
                return match &self.validate {
                    Some(validate) => validate(date).map(|()| date),
                    None => Ok(date),
                };
            }
        }
        Err(format_error(self.formats))
    }
}

impl Default for Options<'static> {
    fn default() -> Self {
        Self::new(&[Format::ISO])
    }
}

fn format_date(value: &str) -> String {
    let digits: String = value
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(8)
        .collect();

    let mut formatted = String::with_capacity(10);
    for (index, digit) in digits.chars().enumerate() {
        if index == 4 || index == 6 {
            formatted.push('-');
        }
        formatted.push(digit);
    }
    formatted
}

fn sanitize(value: &str, formats: &[Format<'_>]) -> String {
    if matches!(formats, [format] if format.is_iso()) {
        return format_date(value);
    }

    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-/. ,".contains(*c))
        .take(max_placeholder_len(formats))
        .collect()
}

fn format_error(formats: &[Format<'_>]) -> String {
    let formats = formats
        .iter()
        .map(|format| format.placeholder)
        .collect::<Vec<_>>()
        .join(", ");
    fl!("invalid-date", formats = formats)
}

fn min_placeholder_len(formats: &[Format<'_>]) -> usize {
    formats
        .iter()
        .map(|format| format.placeholder.chars().count())
        .min()
        .unwrap_or(10)
        .max(1)
}

fn max_placeholder_len(formats: &[Format<'_>]) -> usize {
    formats
        .iter()
        .map(|format| format.placeholder.chars().count())
        .max()
        .unwrap_or(10)
        .max(1)
}

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

    let Options { formats, .. } = options;
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
