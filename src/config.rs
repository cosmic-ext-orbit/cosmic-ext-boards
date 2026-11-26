// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub density: Density,
    pub show_labels: bool,
    pub show_due_dates: bool,
    pub show_checklist_progress: bool,
    pub show_column_counts: bool,
    pub reopen_last_board: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            density: Density::Comfortable,
            show_labels: true,
            show_due_dates: true,
            show_checklist_progress: true,
            show_column_counts: true,
            reopen_last_board: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
    TitleOnly,
}
