use std::borrow::Cow;

use cosmic::Element;
use cosmic::iced::Alignment;
use cosmic::widget::{self, text, text_input};

use super::action;

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

    pub fn view<'a, Message: Clone + 'static>(
        &'a self,
        placeholder: String,
        add_label: String,
        remove_label: impl Fn(&str) -> String,
        on_event: fn(Event) -> Message,
    ) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();
        let input = text_input(placeholder, &self.draft)
            .on_input(move |value| on_event(Event::Input(value)));
        let input = if self.triggers.enter {
            input.on_submit(move |value| on_event(Event::Submit(value)))
        } else {
            input
        };
        let tags = self.values.iter().enumerate().fold(
            widget::row::with_capacity(self.values.len()).spacing(spacing.space_xxxs),
            |row, (index, label)| {
                row.push(surface(
                    widget::row::with_capacity(2)
                        .align_y(Alignment::Center)
                        .push(text::caption(label))
                        .push(
                            action("window-close-symbolic", remove_label(label))
                                .extra_small()
                                .on_press(on_event(Event::Remove(index))),
                        ),
                ))
            },
        );
        widget::column::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push_maybe((!self.values.is_empty()).then(|| tags.wrap()))
            .push(widget::settings::item_row(vec![
                input.into(),
                action("list-add-symbolic", add_label)
                    .on_press_maybe(
                        (!self.draft.trim().is_empty())
                            .then(|| on_event(Event::Submit(self.draft.clone()))),
                    )
                    .into(),
            ]))
            .into()
    }
}

pub fn chip<'a, Message: 'a>(label: impl Into<Cow<'a, str>> + 'a) -> Element<'a, Message> {
    surface(text::caption(label))
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
