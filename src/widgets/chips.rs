use std::{borrow::Cow, sync::LazyLock};

use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, Id as WidgetId, text, text_input::StyleSheet};

use super::action;

pub static LABELS_INPUT: LazyLock<WidgetId> = LazyLock::new(WidgetId::unique);

#[derive(Clone, Debug)]
pub enum Event {
    Input(String),
    Submit(String),
    Remove(usize),
}

#[derive(Clone, Copy)]
pub struct Triggers {
    pub separators: &'static [char],
    pub enter: bool,
}

pub struct Chips {
    values: Vec<String>,
    draft: String,
    triggers: Triggers,
}

impl Chips {
    pub fn new(values: Vec<String>, triggers: Triggers) -> Self {
        Self {
            values,
            draft: String::new(),
            triggers,
        }
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Input(value) => {
                // The text input publishes an empty value when Backspace is
                // pressed on an empty field. Treat that as removing the last
                // chip, like a web chip input does.
                if value.is_empty() && self.draft.is_empty() {
                    self.values.pop();
                    return;
                }

                if let Some((committed, draft)) = value.rsplit_once(self.triggers.separators) {
                    for label in committed.split(self.triggers.separators) {
                        self.insert(label);
                    }
                    self.draft = draft.to_owned();
                } else {
                    self.draft = value;
                }
            }
            Event::Submit(value) => {
                self.insert(&value);
                self.draft.clear();
            }
            Event::Remove(index) if index < self.values.len() => {
                self.values.remove(index);
            }
            Event::Remove(_) => {}
        }
    }

    pub fn commit_draft(&mut self) {
        let draft = std::mem::take(&mut self.draft);
        self.insert(&draft);
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }

    fn insert(&mut self, value: &str) {
        let value = value.trim();
        if !value.is_empty() && !self.values.iter().any(|label| label == value) {
            self.values.push(value.to_owned());
        }
    }

    pub fn view<Message: Clone + 'static>(
        &self,
        placeholder: String,
        remove_label: impl Fn(&str) -> String,
        on_event: fn(Event) -> Message,
    ) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let input = widget::text_input(placeholder, &self.draft)
            .id(LABELS_INPUT.clone())
            .style(cosmic::theme::TextInput::EditableText)
            .width(if self.values.is_empty() {
                Length::Fill
            } else {
                // A `Fill` child in a wrapping row would always take the
                // whole line, so keep a typing area that follows the chips.
                Length::Fixed(120.0)
            })
            .padding([0, spacing.space_xxs])
            .on_input(move |value| on_event(Event::Input(value)));
        let input = if self.triggers.enter {
            input.on_submit(move |value| on_event(Event::Submit(value)))
        } else {
            input
        };

        let row = self
            .values
            .iter()
            .enumerate()
            .fold(
                widget::row::with_capacity(self.values.len() + 1)
                    .width(Length::Fill)
                    .spacing(spacing.space_xxs)
                    .align_y(Alignment::Center),
                |row, (index, label)| {
                    row.push(surface(
                        widget::row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .push(text::caption(label))
                            .push(
                                action("window-close-symbolic", remove_label(label))
                                    .extra_small()
                                    .padding(2)
                                    .on_press(on_event(Event::Remove(index))),
                            ),
                    ))
                },
            )
            .push(input);

        widget::container(row.wrap())
            .width(Length::Fill)
            .padding([spacing.space_xxs, spacing.space_xxs])
            .style(field_style)
            .into()
    }
}

pub fn chip<'a, Message: 'a>(label: impl Into<Cow<'a, str>> + 'a) -> Element<'a, Message> {
    surface(text::caption(label))
}

fn field_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let appearance = theme.active(&cosmic::theme::TextInput::Default);
    cosmic::iced::widget::container::Style {
        background: Some(appearance.background),
        border: cosmic::iced::Border {
            radius: appearance.border_radius,
            width: appearance.border_width,
            color: appearance.border_color,
        },
        ..Default::default()
    }
}

fn surface<'a, Message: 'a>(child: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::container(child)
        .padding([2, 6])
        .style(
            |theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                background: Some(
                    cosmic::iced::Color::from(theme.cosmic().accent_color())
                        .scale_alpha(0.18)
                        .into(),
                ),
                border: cosmic::iced::border::rounded(5),
                ..Default::default()
            },
        )
        .into()
}
