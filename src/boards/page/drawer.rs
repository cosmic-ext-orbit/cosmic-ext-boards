use cosmic::Element;
use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::iced::Length;
use cosmic::widget::{
    self, TextEditor, button, checkbox, divider, icon, popover, settings, text, text_input,
};
use jiff::civil::Weekday;

use super::{Board, Editor, Message, Page};
use crate::fl;
use crate::widgets::{action, editor_style, symbolic_svg};

impl Page {
    pub fn drawer<'a>(&'a self, board: &'a Board) -> Option<ContextDrawer<'a, Message>> {
        if self.archive {
            return Some(archive(board));
        }

        let editor = self.editor.as_ref()?;
        let id = editor.card.id;
        let (column, _) = board.card(id)?;
        let spacing = cosmic::theme::spacing();
        let destinations: Vec<_> = board
            .columns
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        let selected = board.columns.iter().position(|entry| entry.id == column.id);
        let ids: Vec<_> = board.columns.iter().map(|entry| entry.id).collect();
        let ordering = widget::row::with_capacity(2)
            .push(
                action("go-up-symbolic", fl!("move-up"))
                    .on_press(Message::Shift(id, column.id, false)),
            )
            .push(
                action("go-down-symbolic", fl!("move-down"))
                    .on_press(Message::Shift(id, column.id, true)),
            );
        let title = if editor.card.title.trim().is_empty() {
            fl!("card-title")
        } else {
            editor.card.title.clone()
        };

        let content = settings::view_column(vec![
            widget::column::with_capacity(3)
                .spacing(spacing.space_xs)
                .push_maybe(editor.editing_title.then(|| {
                    settings::item_row(vec![
                        text::body(fl!("card-title")).into(),
                        text_input(fl!("card-title"), &editor.card.title)
                            .on_input(Message::EditTitle)
                            .on_submit(|_| Message::ToggleTitle)
                            .into(),
                    ])
                }))
                .push(settings::item_row(vec![
                    field_label(
                        symbolic_svg(include_bytes!(
                            "../../../resources/icons/board-symbolic.svg"
                        )),
                        fl!("status"),
                    ),
                    widget::space::horizontal().into(),
                    widget::dropdown(destinations, selected, move |index| {
                        Message::MoveToColumn(id, ids[index])
                    })
                    .into(),
                    ordering.into(),
                ]))
                .push(due_date(editor))
                .into(),
            divider::horizontal::light().into(),
            widget::column::with_capacity(2)
                .spacing(spacing.space_xs)
                .push(text::heading(fl!("description")))
                .push(
                    TextEditor::new(&editor.description)
                        .placeholder(fl!("description"))
                        .height(90)
                        .style(editor_style)
                        .on_action(Message::Description),
                )
                .into(),
            widget::column::with_capacity(2)
                .spacing(spacing.space_xs)
                .push(text::heading(fl!("labels")))
                .push(editor.labels.view(
                    fl!("add-label"),
                    fl!("add-label"),
                    |label| fl!("remove-label", label = label),
                    Message::Labels,
                ))
                .into(),
            divider::horizontal::light().into(),
            checklist(editor),
        ])
        .spacing(spacing.space_s);

        let footer = widget::row::with_capacity(2)
            .spacing(spacing.space_xs)
            .push(
                button::suggested(fl!("save"))
                    .leading_icon(icon::from_name("document-save-symbolic"))
                    .on_press_maybe(editor.valid().then_some(Message::SaveCard)),
            )
            .push(
                button::standard(fl!("archive"))
                    .leading_icon(icon::from_name("mail-archive-symbolic"))
                    .on_press(Message::Archive(id, true)),
            );

        Some(
            context_drawer::context_drawer(content, Message::Close)
                .title(title)
                .actions(action("edit-symbolic", fl!("rename-card")).on_press(Message::ToggleTitle))
                .footer(footer),
        )
    }
}

fn due_date(editor: &Editor) -> Element<'_, Message> {
    let button =
        action("x-office-calendar-symbolic", fl!("choose-date")).on_press(Message::ToggleCalendar);
    let mut date = popover(button)
        .position(popover::Position::Bottom)
        .on_close(Message::CloseCalendar);
    if let Some(model) = &editor.calendar {
        date = date.popup(
            widget::container(widget::calendar(
                model,
                Message::SelectDate,
                || Message::CalendarMonth(false),
                || Message::CalendarMonth(true),
                Weekday::Monday,
            ))
            .padding(cosmic::theme::spacing().space_xs)
            .class(cosmic::theme::Container::Card),
        );
    }
    settings::item_row(vec![
        field_label(
            icon::from_name("x-office-calendar-symbolic"),
            fl!("due-date"),
        ),
        text_input("YYYY-MM-DD", &editor.date)
            .on_input(Message::Date)
            .into(),
        date.into(),
        action("edit-clear-symbolic", fl!("clear-date"))
            .on_press_maybe((!editor.date.is_empty()).then(|| Message::Date(String::new())))
            .into(),
    ])
    .into()
}

fn checklist(editor: &Editor) -> Element<'_, Message> {
    let spacing = cosmic::theme::spacing();
    let mut items = widget::column::with_capacity(editor.card.checklist.len() + 2)
        .spacing(spacing.space_xxs)
        .push(field_label(
            icon::from_name("object-select-symbolic"),
            fl!("checklist"),
        ));
    for item in &editor.card.checklist {
        let id = item.id;
        items = items.push(settings::item_row(vec![
            checkbox(item.checked)
                .label(&item.text)
                .width(Length::Fill)
                .on_toggle(move |checked| Message::ToggleItem(id, checked))
                .into(),
            action("edit-delete-symbolic", fl!("remove"))
                .on_press(Message::RemoveItem(id))
                .into(),
        ]));
    }
    items
        .push(settings::item_row(vec![
            text_input(fl!("checklist-item"), &editor.item)
                .on_input(Message::Item)
                .on_submit(|_| Message::AddItem)
                .into(),
            action("list-add-symbolic", fl!("add"))
                .on_press_maybe((!editor.item.trim().is_empty()).then_some(Message::AddItem))
                .into(),
        ]))
        .into()
}

fn archive(board: &Board) -> ContextDrawer<'_, Message> {
    let cards = board
        .columns
        .iter()
        .flat_map(|column| &column.cards)
        .filter(|card| card.archived);
    let mut content = widget::column::with_capacity(4).spacing(cosmic::theme::spacing().space_xs);
    let mut empty = true;
    for card in cards {
        empty = false;
        content = content.push(settings::item(
            card.title.as_str(),
            button::text(fl!("restore"))
                .leading_icon(icon::from_name("edit-undo-symbolic"))
                .on_press(Message::Archive(card.id, false)),
        ));
    }
    context_drawer::context_drawer(
        content.push_maybe(empty.then(|| text::body(fl!("archive-empty")))),
        Message::Close,
    )
    .title(fl!("archive"))
}

fn field_label(icon: impl Into<icon::Handle>, label: String) -> Element<'static, Message> {
    settings::item_row(vec![
        widget::icon(icon.into()).size(16).into(),
        text::body(label).into(),
    ])
    .width(Length::Shrink)
    .into()
}
