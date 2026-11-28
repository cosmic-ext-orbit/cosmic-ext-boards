use cosmic::Element;
use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::iced::Length;
use cosmic::widget::{self, TextEditor, button, checkbox, date_input, icon, settings, text};

use super::{Board, Editor, Message, Page};
use crate::fl;
use crate::widgets::{action, editor_style};

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
        let mut active = column.cards.iter().filter(|card| !card.archived);
        let can_move_up = active.clone().next().is_some_and(|card| card.id != id);
        let can_move_down = active.next_back().is_some_and(|card| card.id != id);
        let title = widget::editable_input(
            fl!("card-title"),
            &editor.card.title,
            editor.editing_title,
            Message::EditTitleActive,
        )
        .font(cosmic::font::bold())
        .size(20)
        .on_input(Message::EditTitle)
        .on_submit(|_| Message::EditTitleActive(false));

        let fields = widget::list_column::with_capacity(2)
            .add(
                settings::item::builder(fl!("status"))
                    .icon(
                        icon::from_svg_bytes(include_bytes!(
                            "../../../resources/icons/board-symbolic.svg"
                        ))
                        .symbolic(true)
                        .icon()
                        .size(16),
                    )
                    .control(widget::dropdown(destinations, selected, move |index| {
                        Message::MoveToColumn(id, ids[index])
                    })),
            )
            .list_item_padding([0, spacing.space_m])
            .add(due_date(editor));
        let description = settings::section()
            .title(fl!("description"))
            .add(settings::item_row(vec![
                TextEditor::new(&editor.description)
                    .placeholder(fl!("description"))
                    .height(90)
                    .style(editor_style)
                    .on_action(Message::Description)
                    .into(),
            ]));
        let labels = settings::section()
            .title(fl!("labels"))
            .add(settings::item_row(vec![editor.labels.view(
                fl!("add-label"),
                |label| fl!("remove-label", label = label),
                Message::Labels,
            )]));
        let content = settings::view_column(vec![
            fields.into(),
            description.into(),
            labels.into(),
            checklist(editor),
        ]);

        let actions = widget::row::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push(
                action("go-up-symbolic", fl!("move-up"))
                    .on_press_maybe(can_move_up.then_some(Message::Shift(id, column.id, false))),
            )
            .push(
                action("go-down-symbolic", fl!("move-down"))
                    .on_press_maybe(can_move_down.then_some(Message::Shift(id, column.id, true))),
            );

        let footer = widget::row::with_capacity(3)
            .padding([spacing.space_xxxs, 0])
            .push(button::standard(fl!("archive")).on_press(Message::Archive(id, true)))
            .push(widget::space::horizontal())
            .push(
                button::suggested(fl!("save"))
                    .on_press_maybe(editor.valid().then_some(Message::SaveCard)),
            );

        Some(
            context_drawer::context_drawer(content, Message::Close)
                .actions(actions)
                .header(title)
                .footer(footer),
        )
    }
}

fn due_date(editor: &Editor) -> widget::Row<'_, Message, cosmic::Theme> {
    let options = date_input::Options::auto();
    let item = settings::item::builder(fl!("due-date"));
    let item = if let Some(message) = options.validation_message(&editor.date) {
        item.description(message)
    } else {
        item
    };

    item.icon(
        icon::from_name("x-office-calendar-symbolic")
            .size(16)
            .icon(),
    )
    .control(date_input::date_input(
        &editor.date,
        &editor.date_state,
        options,
        Message::Calendar,
    ))
}

fn checklist(editor: &Editor) -> Element<'_, Message> {
    let spacing = cosmic::theme::spacing();
    let items = editor.card.checklist.iter().map(|item| {
        let id = item.id;
        let draft = editor
            .checklist_edit
            .as_ref()
            .filter(|(active, _)| *active == id)
            .map(|(_, draft)| draft.as_str());
        settings::item_row(vec![
            checkbox(item.checked)
                .name(item.text.as_str())
                .on_toggle(move |checked| Message::ToggleItem(id, checked))
                .into(),
            widget::editable_input(
                fl!("checklist-item"),
                draft.unwrap_or(&item.text),
                draft.is_some(),
                move |active| Message::EditChecklistItem(id, active),
            )
            .padding(0)
            .width(Length::Fill)
            .on_input(move |text| Message::ChecklistItemText(id, text))
            .on_unfocus(Message::EditChecklistItem(id, false))
            .on_submit(move |_| Message::SubmitChecklistItem(id))
            .into(),
            action("edit-delete-symbolic", fl!("remove"))
                .on_press(Message::RemoveItem(id))
                .into(),
        ])
    });
    settings::section()
        .title(fl!("checklist"))
        .extend(items)
        .add(settings::item_row(vec![
            widget::editable_input(
                fl!("checklist-item"),
                &editor.item,
                editor.editing_item,
                Message::EditItemActive,
            )
            .padding([0, spacing.space_xxs])
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
