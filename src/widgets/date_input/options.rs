use jiff::civil::Date;

use super::{
    format::{Format, format_error, min_placeholder_len},
    locale::locale_formats,
};

/// Configuration for the date input widget.
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

    /// Accepts the date format detected from the system locale.
    pub fn auto() -> Self {
        Self::new(locale_formats())
    }

    /// Validates parsed dates, using the returned message for invalid dates.
    #[allow(dead_code)]
    pub fn validate(mut self, validate: impl Fn(Date) -> Result<(), String> + 'a) -> Self {
        self.validate = Some(Box::new(validate));
        self
    }

    pub(super) fn formats(&self) -> &'a [Format<'a>] {
        self.formats
    }

    pub(super) fn placeholder(&self) -> String {
        if self.formats.is_empty() {
            "YYYY-MM-DD".to_owned()
        } else {
            self.formats
                .iter()
                .map(Format::placeholder)
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

    /// Formats a date using the first configured format that can format it.
    pub fn format(&self, date: Date) -> Option<String> {
        self.formats.iter().find_map(|format| format.format(date))
    }

    /// Parses a date using the configured formats and validation.
    pub fn parse(&self, value: &str) -> Option<Date> {
        self.validate_result(value).ok()
    }

    fn validate_result(&self, value: &str) -> Result<Date, String> {
        for format in self.formats {
            if let Some(date) = format.parse(value) {
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
