use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::widget::{self, settings, text, toggler};

use crate::config::{Density, Preferences};
use crate::fl;

#[derive(Clone, Copy, Debug)]
pub enum Change {
    Density(Density),
    Labels(bool),
    DueDates(bool),
    ChecklistProgress(bool),
    ColumnCounts(bool),
    ReopenLastBoard(bool),
}

impl Change {
    pub fn apply(self, preferences: &mut Preferences) {
        match self {
            Self::Density(value) => preferences.density = value,
            Self::Labels(value) => preferences.show_labels = value,
            Self::DueDates(value) => preferences.show_due_dates = value,
            Self::ChecklistProgress(value) => preferences.show_checklist_progress = value,
            Self::ColumnCounts(value) => preferences.show_column_counts = value,
            Self::ReopenLastBoard(value) => preferences.reopen_last_board = value,
        }
    }
}

pub fn drawer<Message: Clone + 'static>(
    preferences: &Preferences,
    error: Option<&str>,
    on_change: fn(Change) -> Message,
    on_close: Message,
) -> ContextDrawer<'static, Message> {
    const DENSITIES: [Density; 3] = [Density::Comfortable, Density::Compact, Density::TitleOnly];

    let density_names = vec![fl!("comfortable"), fl!("compact"), fl!("title-only")];
    let selected = DENSITIES
        .iter()
        .position(|density| *density == preferences.density);
    let cards = settings::section()
        .title(fl!("cards"))
        .add(settings::item(
            fl!("card-density"),
            widget::dropdown(density_names, selected, move |index| {
                on_change(Change::Density(DENSITIES[index]))
            }),
        ))
        .add(settings::item(
            fl!("show-labels"),
            toggler(preferences.show_labels).on_toggle_maybe(
                (preferences.density != Density::TitleOnly)
                    .then_some(move |value| on_change(Change::Labels(value))),
            ),
        ))
        .add(settings::item(
            fl!("show-due-dates"),
            toggler(preferences.show_due_dates).on_toggle_maybe(
                (preferences.density != Density::TitleOnly)
                    .then_some(move |value| on_change(Change::DueDates(value))),
            ),
        ))
        .add(settings::item(
            fl!("show-checklist-progress"),
            toggler(preferences.show_checklist_progress).on_toggle_maybe(
                (preferences.density != Density::TitleOnly)
                    .then_some(move |value| on_change(Change::ChecklistProgress(value))),
            ),
        ));
    let boards = settings::section()
        .title(fl!("boards"))
        .add(settings::item(
            fl!("show-column-counts"),
            toggler(preferences.show_column_counts)
                .on_toggle(move |value| on_change(Change::ColumnCounts(value))),
        ))
        .add(settings::item(
            fl!("reopen-last-board"),
            toggler(preferences.reopen_last_board)
                .on_toggle(move |value| on_change(Change::ReopenLastBoard(value))),
        ));
    let content = settings::view_column(vec![cards.into(), boards.into()])
        .push_maybe(error.map(|error| text::body(fl!("settings-error", error = error))));
    context_drawer::context_drawer(content, on_close).title(fl!("settings"))
}
