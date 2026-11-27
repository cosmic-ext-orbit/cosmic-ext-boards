pub mod chips;

use cosmic::widget::{self, button, icon, text_editor, text_input};

pub fn action<'a, Message: Clone + 'static>(
    name: &'static str,
    label: String,
) -> button::IconButton<'a, Message> {
    button::icon(icon::from_name(name))
        .name(label.clone())
        .tooltip(label)
}

pub fn editor_style(theme: &cosmic::Theme, status: text_editor::Status) -> text_editor::Style {
    use text_input::StyleSheet;

    let class = cosmic::theme::TextInput::EditableText;
    let appearance = match status {
        text_editor::Status::Active => theme.active(&class),
        text_editor::Status::Hovered => theme.hovered(&class),
        text_editor::Status::Focused { .. } => theme.focused(&class),
        text_editor::Status::Disabled => theme.disabled(&class),
    };
    text_editor::Style {
        background: appearance.background,
        border: cosmic::iced::Border {
            radius: appearance.border_radius,
            width: appearance.border_width,
            color: appearance.border_color,
        },
        placeholder: appearance.placeholder_color,
        value: appearance
            .text_color
            .unwrap_or_else(|| theme.current_container().on.into()),
        selection: appearance.selected_fill,
    }
}

pub fn symbolic_svg(bytes: &'static [u8]) -> widget::icon::Handle {
    icon::from_svg_bytes(bytes).symbolic(true)
}
