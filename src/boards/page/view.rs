use std::collections::BTreeSet;

use cosmic::Element;
use cosmic::iced::core::widget::tree;
use cosmic::iced::{Alignment, Color, Length, Shadow, Vector, widget::scrollable};
use cosmic::widget::{self, button, icon, menu, text, text_input};
use jiff::{ZonedDateTime, civil::Date};

use super::{
    Board, CARD_INPUT, Card, Column, DueFilter, Message, NAME_INPUT, NameTarget, Page, Target, drag,
};
use crate::config::{Density, Preferences};
use crate::fl;
use crate::widgets::{action, chips::chip, symbolic_svg};

impl Page {
    pub fn dialog(&self) -> Option<Element<'_, Message>> {
        let (target, value) = self.name.as_ref()?;
        let title = match target {
            NameTarget::Board => fl!("new-board"),
            NameTarget::RenameBoard => fl!("rename-board"),
            NameTarget::Column => fl!("new-column"),
            NameTarget::RenameColumn(_) => fl!("rename-column"),
        };
        let save =
            button::suggested(fl!("save")).leading_icon(icon::from_name("document-save-symbolic"));
        let mut input = text_input(fl!("name"), value)
            .id(NAME_INPUT.clone())
            .on_input(Message::NameChanged);
        let save = if value.trim().is_empty() {
            save
        } else {
            input = input.on_submit(|_| Message::SaveName);
            save.on_press(Message::SaveName)
        };
        Some(
            widget::dialog()
                .title(title)
                .control(input)
                .primary_action(save)
                .secondary_action(
                    button::standard(fl!("cancel"))
                        .leading_icon(icon::from_name("window-close-symbolic"))
                        .on_press(Message::CancelName),
                )
                .into(),
        )
    }

    pub fn view<'a>(
        &'a self,
        board: Option<&'a Board>,
        preferences: &Preferences,
    ) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();
        let mut content = widget::column::with_capacity(3)
            .spacing(spacing.space_s)
            .padding(spacing.space_s);
        if let Some(error) = &self.error {
            let mut status = widget::row::with_capacity(3)
                .spacing(spacing.space_xs)
                .push(text::body(fl!("storage-error", error = error.as_str())));
            if self.pending.is_some() {
                status = status
                    .push(
                        button::standard(fl!("retry"))
                            .leading_icon(icon::from_name("view-refresh-symbolic"))
                            .on_press(Message::Retry),
                    )
                    .push(
                        button::text(fl!("discard"))
                            .leading_icon(icon::from_name("edit-undo-symbolic"))
                            .on_press(Message::Discard),
                    );
            } else {
                status = status.push(
                    button::standard(fl!("retry"))
                        .leading_icon(icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::Reload),
                );
            }
            content = content.push(status);
        }
        let Some(board) = board else {
            return content.push(text::title2(fl!("no-boards"))).into();
        };
        let today = ZonedDateTime::now().date();
        let columns = board.columns.iter().fold(
            widget::row::with_capacity(board.columns.len()).spacing(spacing.space_xs),
            |row, column| row.push(self.column(board, column, today, preferences)),
        );
        let toolbar = widget::row::with_capacity(3)
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center)
            .push(
                widget::container(text::title2(&board.name).font(cosmic::font::bold()))
                    .width(Length::FillPortion(2)),
            )
            .push(
                widget::container(
                    widget::search_input(fl!("search"), &self.search)
                        .width(Length::Fill)
                        .on_input(Message::Search),
                )
                .width(Length::FillPortion(1))
                .max_width(218),
            )
            .push(
                button::standard(fl!("filter"))
                    .leading_icon(symbolic_svg(include_bytes!(
                        "../../../resources/icons/filter-symbolic.svg"
                    )))
                    .class(card_style(
                        self.show_filters || self.label.is_some() || self.due != DueFilter::All,
                        false,
                    ))
                    .on_press(Message::ToggleFilters),
            );
        content
            .push(toolbar)
            .push_maybe(self.show_filters.then(|| self.filters(board)))
            .push(
                widget::scrollable(columns)
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default(),
                    ))
                    .height(Length::Fill),
            )
            .height(Length::Fill)
            .into()
    }

    fn filters(&self, board: &Board) -> Element<'_, Message> {
        let available: BTreeSet<_> = board
            .columns
            .iter()
            .flat_map(|column| &column.cards)
            .filter(|card| !card.archived)
            .flat_map(|card| card.labels.iter().cloned())
            .chain(self.label.iter().cloned())
            .collect();
        let mut labels = vec![None];
        labels.extend(available.into_iter().map(Some));
        let selected = labels.iter().position(|label| *label == self.label);
        let names = labels
            .iter()
            .map(|label| label.clone().unwrap_or_else(|| fl!("any-label")))
            .collect::<Vec<_>>();
        let dates = DueFilter::OPTIONS.map(DueFilter::label);
        let selected_date = DueFilter::OPTIONS
            .iter()
            .position(|filter| *filter == self.due);
        widget::row::with_capacity(2)
            .spacing(cosmic::theme::spacing().space_xs)
            .push(widget::dropdown(names, selected, move |index| {
                Message::FilterLabel(labels[index].clone())
            }))
            .push(widget::dropdown(dates.to_vec(), selected_date, |index| {
                Message::FilterDue(DueFilter::OPTIONS[index])
            }))
            .wrap()
            .into()
    }

    fn column<'a>(
        &'a self,
        board: &'a Board,
        column: &'a Column,
        today: Date,
        preferences: &Preferences,
    ) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();
        let index = board
            .columns
            .iter()
            .position(|entry| entry.id == column.id)
            .unwrap();
        let query = self.search.trim().to_lowercase();
        let cards = column
            .cards
            .iter()
            .filter(|card| {
                !card.archived
                    && self
                        .label
                        .as_ref()
                        .is_none_or(|label| card.labels.contains(label))
                    && self.due.matches(card, column.complete, today)
                    && (query.is_empty()
                        || card.title.to_lowercase().contains(&query)
                        || card.description.to_lowercase().contains(&query)
                        || card
                            .labels
                            .iter()
                            .any(|label| label.to_lowercase().contains(&query)))
            })
            .collect::<Vec<_>>();
        let count = cards.len();
        let padding = match preferences.density {
            Density::Comfortable => [8, 10],
            Density::Compact | Density::TitleOnly => [4, 8],
        };
        let cards = cards.into_iter().fold(
            widget::column::with_capacity(count).spacing(spacing.space_xxs),
            |list, card| {
                let selected = self
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.card.id == card.id);
                let content: Element<'a, Message> = if self.dragging == Some(card.id)
                    && let Some(bounds) = self.card_bounds.get(&card.id)
                {
                    widget::container(cosmic::iced::widget::space())
                        .width(Length::Fill)
                        .height(Length::Fixed(bounds.height))
                        .into()
                } else {
                    button::custom(card_content(card, column.complete, today, preferences))
                        .class(card_style(selected, false))
                        .padding(padding)
                        .width(Length::Fill)
                        .on_press(Message::Open(card.id))
                        .into()
                };
                let surface = drag::source(
                    card.id,
                    self.drag_widgets.sources[&card.id].clone(),
                    content,
                    drag_icon(card, column.complete, today, preferences, selected, padding),
                );
                let target = Target {
                    column: column.id,
                    before: Some(card.id),
                };
                list.push(
                    widget::column::with_capacity(2)
                        .push(self.insertion_line(target))
                        .push(surface),
                )
            },
        );
        let end = Target {
            column: column.id,
            before: None,
        };
        let cards = cards.push(
            widget::column::with_capacity(2)
                .push(self.insertion_line(end))
                .push(cosmic::iced::widget::space().width(Length::Fill).height(12)),
        );
        let add: Element<_> = match &self.composer {
            Some((id, value)) if *id == column.id => widget::row::with_capacity(2)
                .spacing(spacing.space_xxs)
                .push(
                    text_input(fl!("card-title"), value)
                        .id(CARD_INPUT.clone())
                        .on_input(Message::Title)
                        .on_submit(|_| Message::AddCard),
                )
                .push(action("window-close-symbolic", fl!("cancel")).on_press(Message::CancelCard))
                .into(),
            _ => button::custom(
                widget::container(detail("list-add-symbolic", fl!("add-card")))
                    .center_x(Length::Fill),
            )
            .class(card_style(false, false))
            .width(Length::Fill)
            .padding([7, 8])
            .on_press(Message::Compose(column.id))
            .into(),
        };
        let id = column.id;
        let mut actions = vec![menu::Item::Button(
            fl!("rename-column"),
            Some(icon::from_name("edit-symbolic").handle()),
            ColumnAction::Rename(id),
        )];
        if index > 0 {
            actions.push(menu::Item::Button(
                fl!("move-left"),
                Some(icon::from_name("go-previous-symbolic").handle()),
                ColumnAction::Shift(id, false),
            ));
        }
        if index + 1 < board.columns.len() {
            actions.push(menu::Item::Button(
                fl!("move-right"),
                Some(icon::from_name("go-next-symbolic").handle()),
                ColumnAction::Shift(id, true),
            ));
        }
        actions.push(menu::Item::CheckBox(
            fl!("completed-column"),
            None,
            column.complete,
            ColumnAction::Complete(id, !column.complete),
        ));
        let options = menu::bar(vec![menu::Tree::with_children(
            Element::from(action(
                "view-more-horizontal-symbolic",
                fl!("column-options"),
            )),
            menu::items(&Default::default(), actions),
        )]);
        let header = widget::row::with_capacity(4)
            .spacing(spacing.space_xxs)
            .align_y(Alignment::Center)
            .push(text::heading(&column.name))
            .push_maybe(
                preferences
                    .show_column_counts
                    .then(|| chip(count.to_string())),
            )
            .push(cosmic::iced::widget::space().width(Length::Fill))
            .push(options);
        let header = widget::container(header)
            .padding([spacing.space_xxs, spacing.space_xs])
            .width(Length::Fill);
        let body = widget::container(
            widget::column::with_capacity(2)
                .spacing(spacing.space_xxs)
                .push(widget::scrollable(cards).height(Length::Fill))
                .push(add),
        )
        .padding(spacing.space_xs)
        .height(Length::Fill);
        drag::column(
            self.drag_widgets.destinations[&column.id].clone(),
            column.id,
            widget::container(
                widget::column::with_capacity(3)
                    .push(header)
                    .push(widget::divider::horizontal::light())
                    .push(body),
            )
            .width(248)
            .height(Length::Fill)
            .class(cosmic::theme::Container::Card),
        )
    }

    fn insertion_line(&self, target: Target) -> Element<'_, Message> {
        let active = self.drop_target == Some(target);
        widget::container(cosmic::iced::widget::space())
            .width(Length::Fill)
            .height(3)
            .style(
                move |theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                    background: active.then(|| {
                        cosmic::iced::Background::Color(theme.cosmic().accent_color().into())
                    }),
                    ..Default::default()
                },
            )
            .into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnAction {
    Rename(super::Id),
    Shift(super::Id, bool),
    Complete(super::Id, bool),
}

impl menu::action::MenuAction for ColumnAction {
    type Message = Message;

    fn message(&self) -> Message {
        match *self {
            Self::Rename(id) => Message::Name(NameTarget::RenameColumn(id)),
            Self::Shift(id, direction) => Message::ShiftColumn(id, direction),
            Self::Complete(id, complete) => Message::CompleteColumn(id, complete),
        }
    }
}

fn card_content(
    card: &Card,
    complete: bool,
    today: Date,
    preferences: &Preferences,
) -> Element<'static, Message> {
    let spacing = cosmic::theme::spacing();
    let show_info = preferences.density != Density::TitleOnly;
    let mut content = widget::column::with_capacity(3)
        .spacing(spacing.space_xxxs)
        .width(Length::Fill)
        .push(text::heading(card.title.clone()));
    if show_info && preferences.show_labels && !card.labels.is_empty() {
        let labels = card.labels.iter().cloned().fold(
            widget::row::with_capacity(card.labels.len()).spacing(spacing.space_xxxs),
            |row, label| row.push(chip(label)),
        );
        content = content.push(labels.wrap());
    }
    let mut metadata = widget::row::with_capacity(2).spacing(spacing.space_xs);
    let checklist = show_info && preferences.show_checklist_progress && !card.checklist.is_empty();
    let due = card.due.filter(|_| show_info && preferences.show_due_dates);
    if checklist {
        let done = card.checklist.iter().filter(|item| item.checked).count();
        metadata = metadata.push(detail(
            "object-select-symbolic",
            format!("{done}/{}", card.checklist.len()),
        ));
    }
    if let Some(due) = due {
        let date = if due.year() == today.year() {
            due.strftime("%b %-d").to_string()
        } else {
            due.to_string()
        };
        metadata = metadata.push(detail(
            "x-office-calendar-symbolic",
            if due < today && !complete {
                fl!("overdue", date = date)
            } else {
                date
            },
        ));
    }
    if checklist || due.is_some() {
        content = content.push(metadata.wrap());
    }
    content.into()
}

fn drag_icon(
    card: &Card,
    complete: bool,
    today: Date,
    preferences: &Preferences,
    selected: bool,
    padding: [u16; 2],
) -> impl Fn(Vector) -> (Element<'static, ()>, tree::State, Vector) + use<> {
    let card = card.clone();
    let preferences = Preferences {
        density: preferences.density,
        show_labels: preferences.show_labels,
        show_due_dates: preferences.show_due_dates,
        show_checklist_progress: preferences.show_checklist_progress,
        ..Preferences::default()
    };
    move |offset| {
        let content: Element<'static, ()> =
            card_content(&card, complete, today, &preferences).map(|_| ());
        let icon: Element<'static, ()> = widget::container(
            widget::container(
                button::custom(content)
                    .class(card_style(selected, true))
                    .padding(padding)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .on_press(()),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme| {
                let card = cosmic::iced::widget::container::Catalog::style(
                    theme,
                    &cosmic::theme::Container::Card,
                );
                cosmic::iced::widget::container::Style {
                    background: card.background,
                    border: cosmic::iced::Border {
                        radius: 7.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
        )
        .padding([0.0, 4.0, 4.0, 0.0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| cosmic::iced::widget::container::Style {
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into();
        let state = tree::Tree::new(&icon).state;
        // libcosmic provides the point inside the card where the drag began. The
        // drag surface starts with its top-left corner at the cursor, so negate
        // that point to keep the card under the cursor where it was grabbed.
        let icon_offset = Vector::new(-offset.x, -offset.y);
        (icon, state, icon_offset)
    }
}

fn detail<'a>(icon: &'static str, label: String) -> Element<'a, Message> {
    widget::row::with_capacity(2)
        .align_y(Alignment::Center)
        .spacing(4)
        .push(widget::icon::from_name(icon).size(14).icon())
        .push(text::caption(label))
        .into()
}

fn card_style(selected: bool, lifted: bool) -> cosmic::theme::Button {
    use button::Catalog;
    use cosmic::theme::Button;

    fn rectangular(
        style: button::Style,
        highlighted: bool,
        lifted: bool,
        theme: &cosmic::Theme,
    ) -> button::Style {
        button::Style {
            border_radius: 7.into(),
            border_width: 1.0,
            border_color: if highlighted || lifted {
                theme.cosmic().accent_color().into()
            } else {
                style.border_color
            },
            ..style
        }
    }

    Button::Custom {
        active: Box::new(move |focused, theme| {
            rectangular(
                theme.active(focused, false, &Button::Standard),
                selected || focused,
                lifted,
                theme,
            )
        }),
        hovered: Box::new(move |focused, theme| {
            rectangular(
                theme.hovered(focused, false, &Button::Standard),
                selected || focused,
                lifted,
                theme,
            )
        }),
        pressed: Box::new(move |focused, theme| {
            rectangular(
                theme.pressed(focused, false, &Button::Standard),
                selected || focused,
                lifted,
                theme,
            )
        }),
        disabled: Box::new(move |theme| {
            rectangular(theme.disabled(&Button::Standard), false, lifted, theme)
        }),
    }
}
