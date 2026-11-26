use std::collections::BTreeSet;

use cosmic::Element;
use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::iced::{Alignment, Length, widget::scrollable};
use cosmic::widget::{self, TextEditor, button, checkbox, menu, text, text_input};
use jiff::{
    ZonedDateTime,
    civil::{Date, Weekday},
};

use super::{
    Board, CARD_INPUT, Column, DueFilter, Message, NAME_INPUT, NameTarget, Page, Target, drag,
};
use crate::fl;

impl Page {
    pub fn dialog(&self) -> Option<Element<'_, Message>> {
        let (target, value) = self.name.as_ref()?;
        let title = match target {
            NameTarget::Board => fl!("new-board"),
            NameTarget::RenameBoard => fl!("rename-board"),
            NameTarget::Column => fl!("new-column"),
            NameTarget::RenameColumn(_) => fl!("rename-column"),
        };
        let save = button::suggested(fl!("save"));
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
                .secondary_action(button::standard(fl!("cancel")).on_press(Message::CancelName))
                .into(),
        )
    }

    pub fn view<'a>(&'a self, board: Option<&'a Board>) -> Element<'a, Message> {
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
                    .push(button::standard(fl!("retry")).on_press(Message::Retry))
                    .push(button::text(fl!("discard")).on_press(Message::Discard));
            } else {
                status = status.push(button::standard(fl!("retry")).on_press(Message::Reload));
            }
            content = content.push(status);
        }
        let Some(board) = board else {
            return content.push(text::title2(fl!("no-boards"))).into();
        };
        let today = ZonedDateTime::now().date();
        let columns = board.columns.iter().fold(
            widget::row::with_capacity(board.columns.len()).spacing(spacing.space_xs),
            |row, column| row.push(self.column(board, column, today)),
        );
        let toolbar = widget::row::with_capacity(3)
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center)
            .push(widget::container(text::title2(&board.name)).width(Length::FillPortion(2)))
            .push(
                widget::container(
                    text_input(fl!("search"), &self.search)
                        .width(Length::Fill)
                        .on_input(Message::Search),
                )
                .width(Length::FillPortion(1))
                .max_width(218),
            )
            .push(
                button::standard(fl!("filter"))
                    .class(card_style(
                        self.show_filters || self.label.is_some() || self.due != DueFilter::All,
                    ))
                    .on_press(Message::ToggleFilters),
            );
        content
            .push(toolbar)
            .push_maybe(self.show_filters.then(|| self.filters(board)))
            .push(
                widget::scrollable(columns)
                    .direction(scrollable::Direction::Both {
                        horizontal: scrollable::Scrollbar::default(),
                        vertical: scrollable::Scrollbar::default(),
                    })
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
        let cards = cards.into_iter().fold(
            widget::column::with_capacity(count).spacing(spacing.space_xxs),
            |list, card| {
                let selected = self
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.card.id == card.id);
                let title = text::heading(&card.title);
                let mut content = widget::column::with_capacity(3)
                    .spacing(spacing.space_xxxs)
                    .width(Length::Fill)
                    .push(title);
                if !card.labels.is_empty() {
                    let labels = card.labels.iter().fold(
                        widget::row::with_capacity(card.labels.len()).spacing(spacing.space_xxxs),
                        |row, label| row.push(chip(label)),
                    );
                    content = content.push(labels.wrap());
                }
                let mut metadata = widget::row::with_capacity(2).spacing(spacing.space_xs);
                if !card.checklist.is_empty() {
                    let done = card.checklist.iter().filter(|item| item.checked).count();
                    metadata = metadata.push(detail(
                        "object-select-symbolic",
                        format!("{done}/{}", card.checklist.len()),
                    ));
                }
                if let Some(due) = card.due {
                    let date = if due.year() == today.year() {
                        due.strftime("%b %-d").to_string()
                    } else {
                        due.to_string()
                    };
                    metadata = metadata.push(detail(
                        "x-office-calendar-symbolic",
                        if due < today && !column.complete {
                            fl!("overdue", date = date)
                        } else {
                            date
                        },
                    ));
                }
                if !card.checklist.is_empty() || card.due.is_some() {
                    content = content.push(metadata.wrap());
                }
                let handle = drag::handle(
                    card.id,
                    self.drag_widgets.sources[&card.id].clone(),
                    widget::container(
                        widget::icon::from_name("view-grid-symbolic")
                            .size(14)
                            .icon(),
                    )
                    .padding([6, 3]),
                );
                let row = widget::container(
                    widget::row::with_capacity(2)
                        .align_y(Alignment::Center)
                        .push(
                            button::custom(content)
                                .class(card_style(selected))
                                .padding([8, 10])
                                .width(Length::Fill)
                                .on_press(Message::Open(card.id)),
                        )
                        .push(handle),
                )
                .class(cosmic::theme::Container::Card);
                let target = Target {
                    column: column.id,
                    before: Some(card.id),
                };
                list.push(
                    target.view(
                        self.drag_widgets.destinations[&target].clone(),
                        widget::column::with_capacity(2)
                            .push(self.insertion_line(target))
                            .push(row),
                    ),
                )
            },
        );
        let end = Target {
            column: column.id,
            before: None,
        };
        let cards = cards.push(
            end.view(
                self.drag_widgets.destinations[&end].clone(),
                widget::column::with_capacity(2)
                    .push(self.insertion_line(end))
                    .push(cosmic::iced::widget::space().width(Length::Fill).height(12)),
            ),
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
                .push(
                    button::icon(widget::icon::from_name("window-close-symbolic"))
                        .tooltip(fl!("cancel"))
                        .on_press(Message::CancelCard),
                )
                .into(),
            _ => button::custom(
                widget::container(detail("list-add-symbolic", fl!("add-card")))
                    .center_x(Length::Fill),
            )
            .class(card_style(false))
            .width(Length::Fill)
            .padding([7, 8])
            .on_press(Message::Compose(column.id))
            .into(),
        };
        let id = column.id;
        let mut actions = vec![menu::Item::Button(
            fl!("rename-column"),
            None,
            ColumnAction::Rename(id),
        )];
        if index > 0 {
            actions.push(menu::Item::Button(
                fl!("move-left"),
                None,
                ColumnAction::Shift(id, false),
            ));
        }
        if index + 1 < board.columns.len() {
            actions.push(menu::Item::Button(
                fl!("move-right"),
                None,
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
            Element::from(menu::root("…")),
            menu::items(&Default::default(), actions),
        )]);
        let header = widget::row::with_capacity(4)
            .spacing(spacing.space_xxs)
            .align_y(Alignment::Center)
            .push(text::heading(&column.name))
            .push(chip(count.to_string()))
            .push(cosmic::iced::widget::space().width(Length::Fill))
            .push(options);
        widget::container(
            widget::column::with_capacity(3)
                .spacing(spacing.space_xxs)
                .push(header)
                .push(widget::scrollable(cards).height(Length::Fill))
                .push(add),
        )
        .padding(spacing.space_xs)
        .width(248)
        .height(276)
        .class(cosmic::theme::Container::Card)
        .into()
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

    pub fn drawer<'a>(&'a self, board: &'a Board) -> Option<ContextDrawer<'a, Message>> {
        let spacing = cosmic::theme::spacing();
        if self.archive {
            let mut content = widget::column::with_capacity(4).spacing(spacing.space_xs);
            for card in board
                .columns
                .iter()
                .flat_map(|column| &column.cards)
                .filter(|card| card.archived)
            {
                content = content.push(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_s)
                        .push(text::body(&card.title))
                        .push(
                            button::text(fl!("restore")).on_press(Message::Archive(card.id, false)),
                        ),
                );
            }
            if !board
                .columns
                .iter()
                .flat_map(|column| &column.cards)
                .any(|card| card.archived)
            {
                content = content.push(text::body(fl!("archive-empty")));
            }
            return Some(
                context_drawer::context_drawer(content, Message::Close).title(fl!("archive")),
            );
        }
        let editor = self.editor.as_ref()?;
        let id = editor.card.id;
        let (column, card) = board.card(id)?;
        let destinations: Vec<_> = board
            .columns
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        let selected = board.columns.iter().position(|entry| entry.id == column.id);
        let column_ids: Vec<_> = board.columns.iter().map(|entry| entry.id).collect();
        let items = editor.card.checklist.iter().fold(
            widget::column::with_capacity(editor.card.checklist.len()).spacing(spacing.space_xxs),
            |list, item| {
                let item_id = item.id;
                list.push(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_xxs)
                        .push(
                            checkbox(item.checked)
                                .label(&item.text)
                                .on_toggle(move |checked| Message::ToggleItem(item_id, checked)),
                        )
                        .push(button::text(fl!("remove")).on_press(Message::RemoveItem(item_id))),
                )
            },
        );
        let content = widget::column::with_capacity(13)
            .spacing(spacing.space_xs)
            .push(
                widget::row::with_capacity(2)
                    .align_y(Alignment::Center)
                    .spacing(spacing.space_xs)
                    .push(widget::container(text::body(fl!("status"))).width(92))
                    .push(widget::dropdown(destinations, selected, move |index| {
                        Message::MoveToColumn(id, column_ids[index])
                    })),
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_xs)
                    .push(
                        button::text(fl!("move-up")).on_press(Message::Shift(id, column.id, false)),
                    )
                    .push(
                        button::text(fl!("move-down"))
                            .on_press(Message::Shift(id, column.id, true)),
                    ),
            )
            .push(text::body(fl!("card-title")))
            .push(text_input(fl!("card-title"), &editor.card.title).on_input(Message::EditTitle))
            .push(text::body(fl!("description")))
            .push(
                TextEditor::new(&editor.description)
                    .placeholder(fl!("description"))
                    .height(100)
                    .on_action(Message::Description),
            )
            .push(
                widget::row::with_capacity(3)
                    .align_y(Alignment::Center)
                    .spacing(spacing.space_xxs)
                    .push(widget::container(text::body(fl!("due-date"))).width(92))
                    .push(text_input("YYYY-MM-DD", &editor.date).on_input(Message::Date))
                    .push(
                        button::icon(widget::icon::from_name("x-office-calendar-symbolic"))
                            .tooltip(fl!("choose-date"))
                            .on_press(Message::ToggleCalendar),
                    ),
            )
            .push_maybe(editor.calendar.as_ref().map(|calendar| {
                widget::calendar(
                    calendar,
                    Message::SelectDate,
                    || Message::CalendarMonth(false),
                    || Message::CalendarMonth(true),
                    Weekday::Monday,
                )
            }))
            .push(text::body(fl!("labels")))
            .push(text_input(fl!("labels"), &editor.labels).on_input(Message::Labels))
            .push(text::title4(fl!("checklist")))
            .push(items)
            .push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(
                        text_input(fl!("checklist-item"), &editor.item)
                            .on_input(Message::Item)
                            .on_submit(|_| Message::AddItem),
                    )
                    .push(button::standard(fl!("add")).on_press(Message::AddItem)),
            );
        let footer = widget::row::with_capacity(2)
            .spacing(spacing.space_xs)
            .push(
                button::suggested(fl!("save"))
                    .on_press_maybe(editor.valid().then_some(Message::SaveCard)),
            )
            .push(button::text(fl!("archive")).on_press(Message::Archive(id, true)));
        Some(
            context_drawer::context_drawer(content, Message::Close)
                .title(&card.title)
                .footer(footer),
        )
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

fn detail<'a>(icon: &'static str, label: String) -> Element<'a, Message> {
    widget::row::with_capacity(2)
        .align_y(Alignment::Center)
        .spacing(4)
        .push(widget::icon::from_name(icon).size(14).icon())
        .push(text::caption(label))
        .into()
}

fn chip<'a>(label: impl Into<std::borrow::Cow<'a, str>> + 'a) -> Element<'a, Message> {
    widget::container(text::caption(label))
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

fn card_style(selected: bool) -> cosmic::theme::Button {
    use button::Catalog;
    use cosmic::theme::Button;

    fn rectangular(
        style: button::Style,
        highlighted: bool,
        theme: &cosmic::Theme,
    ) -> button::Style {
        button::Style {
            border_radius: 7.into(),
            border_width: 1.0,
            border_color: if highlighted {
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
                theme,
            )
        }),
        hovered: Box::new(move |focused, theme| {
            rectangular(
                theme.hovered(focused, false, &Button::Standard),
                selected || focused,
                theme,
            )
        }),
        pressed: Box::new(move |focused, theme| {
            rectangular(
                theme.pressed(focused, false, &Button::Standard),
                selected || focused,
                theme,
            )
        }),
        disabled: Box::new(|theme| rectangular(theme.disabled(&Button::Standard), false, theme)),
    }
}
