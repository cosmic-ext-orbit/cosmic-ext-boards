use jiff::{civil::Date, fmt::strtime};

use crate::fl;

enum Parse<'a> {
    Iso,
    Strftime(&'a str),
    #[allow(dead_code)]
    Custom(Box<dyn Fn(&str) -> Option<Date> + Send + Sync + 'a>),
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

/// A date format accepted by the date input widget.
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
    pub const fn strftime(format: &'a str, placeholder: &'a str) -> Self {
        Self {
            placeholder,
            parser: Parse::Strftime(format),
        }
    }

    /// A caller-provided parser for formats that `strftime` cannot express.
    #[allow(dead_code)]
    pub fn custom(
        placeholder: &'a str,
        parse: impl Fn(&str) -> Option<Date> + Send + Sync + 'a,
    ) -> Self {
        Self {
            placeholder,
            parser: Parse::Custom(Box::new(parse)),
        }
    }

    pub(super) fn placeholder(&self) -> &'a str {
        self.placeholder
    }

    pub(super) fn parse(&self, value: &str) -> Option<Date> {
        self.parser.parse(value)
    }

    pub(super) fn format(&self, date: Date) -> Option<String> {
        match &self.parser {
            Parse::Iso => Some(date.to_string()),
            Parse::Strftime(format) => Some(date.strftime(format).to_string()),
            Parse::Custom(_) => None,
        }
    }
}

#[derive(Debug)]
struct Mask {
    segments: Vec<usize>,
    separators: Vec<String>,
    digits_only: bool,
}

impl Mask {
    fn from_placeholder(placeholder: &str) -> Option<Self> {
        let mut chars = placeholder.chars().peekable();
        let mut segments = Vec::new();
        let mut separators = Vec::new();

        while chars.peek().is_some() {
            let mut segment = String::new();
            while let Some(char) = chars.peek().copied() {
                if !char.is_ascii_alphanumeric() {
                    break;
                }
                segment.push(char);
                chars.next();
            }
            if segment.is_empty() {
                return None;
            }
            segments.push(segment.chars().count());

            let mut separator = String::new();
            while let Some(char) = chars.peek().copied() {
                if char.is_ascii_alphanumeric() {
                    break;
                }
                separator.push(char);
                chars.next();
            }
            if separator.is_empty() {
                break;
            }
            separators.push(separator);
        }

        (segments.len() == separators.len() + 1).then_some(Self {
            segments,
            separators,
            digits_only: placeholder
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .all(|c| matches!(c, 'Y' | 'M' | 'D')),
        })
    }

    fn apply(&self, value: &str) -> String {
        let total: usize = self.segments.iter().sum();
        let values: Vec<char> = value
            .chars()
            .filter(|c| {
                if self.digits_only {
                    c.is_ascii_digit()
                } else {
                    c.is_ascii_alphanumeric()
                }
            })
            .take(total)
            .collect();

        let mut formatted = String::new();
        let mut index = 0;
        for (segment, len) in self.segments.iter().enumerate() {
            if index == values.len() {
                break;
            }
            if segment > 0
                && let Some(separator) = self.separators.get(segment - 1)
            {
                formatted.push_str(separator);
            }
            for char in &values[index..(index + *len).min(values.len())] {
                formatted.push(*char);
            }
            index = (index + *len).min(values.len());
        }

        let mut filled = 0;
        for (index, len) in self
            .segments
            .iter()
            .enumerate()
            .take(self.segments.len() - 1)
        {
            filled += *len;
            if values.len() == filled && value.ends_with(self.separators[index].as_str()) {
                formatted.push_str(&self.separators[index]);
            }
        }

        formatted
    }
}

pub(super) fn sanitize(value: &str, formats: &[Format<'_>]) -> String {
    if let [format] = formats
        && let Some(mask) = Mask::from_placeholder(format.placeholder())
    {
        return mask.apply(value);
    }

    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-/. ,".contains(*c))
        .take(max_placeholder_len(formats))
        .collect()
}

pub(super) fn format_error(formats: &[Format<'_>]) -> String {
    let formats = formats
        .iter()
        .map(Format::placeholder)
        .collect::<Vec<_>>()
        .join(", ");
    fl!("invalid-date", formats = formats)
}

pub(super) fn min_placeholder_len(formats: &[Format<'_>]) -> usize {
    formats
        .iter()
        .map(Format::placeholder)
        .map(|placeholder| placeholder.chars().count())
        .min()
        .unwrap_or(10)
        .max(1)
}

fn max_placeholder_len(formats: &[Format<'_>]) -> usize {
    formats
        .iter()
        .map(Format::placeholder)
        .map(|placeholder| placeholder.chars().count())
        .max()
        .unwrap_or(10)
        .max(1)
}
