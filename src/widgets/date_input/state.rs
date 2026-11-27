use cosmic::widget::calendar::CalendarModel;
use jiff::{ToSpan, ZonedDateTime};

use super::{Event, Options};

/// Edit and calendar state for a date input.
#[derive(Default)]
pub struct DateInputState {
    editing: bool,
    calendar: Option<CalendarModel>,
}

impl DateInputState {
    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn calendar(&self) -> Option<&CalendarModel> {
        self.calendar.as_ref()
    }

    pub fn close_calendar(&mut self) -> bool {
        self.calendar.take().is_some()
    }

    pub fn update(&mut self, event: Event, value: &mut String, options: &Options<'_>) {
        match event {
            Event::Input(input) => {
                *value = input;
                if value.is_empty() {
                    self.calendar = None;
                }
            }
            Event::Edit(editing) => {
                self.editing = editing;
                if editing {
                    self.open(value, options);
                } else {
                    self.calendar = None;
                }
            }
            Event::Focus => self.open(value, options),
            Event::Unfocus | Event::Close => self.calendar = None,
            Event::PreviousMonth => self.shift_month(-1),
            Event::NextMonth => self.shift_month(1),
            Event::Select(date) => {
                *value = options.format(date).unwrap_or_else(|| date.to_string());
                self.calendar = None;
            }
        }
    }

    fn open(&mut self, value: &str, options: &Options<'_>) {
        if self.calendar.is_none() {
            let date = options
                .parse(value.trim())
                .unwrap_or_else(|| ZonedDateTime::now().date());
            self.calendar = Some(CalendarModel::new(date, date));
        }
    }

    fn shift_month(&mut self, months: i32) {
        if let Some(calendar) = &mut self.calendar
            && let Ok(date) = calendar.visible.checked_add(months.months())
        {
            calendar.visible = date;
        }
    }
}
