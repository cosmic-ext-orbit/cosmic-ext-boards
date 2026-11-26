use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Id = Uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Boards {
    pub boards: Vec<Board>,
    pub selected: Option<Id>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Board {
    pub id: Id,
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub id: Id,
    pub name: String,
    pub complete: bool,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub due: Option<Date>,
    pub checklist: Vec<ChecklistItem>,
    pub archived: bool,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: Id,
    pub text: String,
    pub checked: bool,
}

impl Boards {
    pub fn active(&self) -> Option<&Board> {
        self.boards
            .iter()
            .find(|board| Some(board.id) == self.selected)
    }

    pub fn active_mut(&mut self) -> Option<&mut Board> {
        self.boards
            .iter_mut()
            .find(|board| Some(board.id) == self.selected)
    }
}

impl Board {
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        Self {
            id: Id::new_v4(),
            name,
            columns,
        }
    }

    pub fn card(&self, id: Id) -> Option<(&Column, &Card)> {
        self.columns.iter().find_map(|column| {
            column
                .cards
                .iter()
                .find(|card| card.id == id)
                .map(|card| (column, card))
        })
    }

    pub fn card_mut(&mut self, id: Id) -> Option<&mut Card> {
        self.columns
            .iter_mut()
            .flat_map(|column| &mut column.cards)
            .find(|card| card.id == id)
    }

    pub fn move_card(&mut self, id: Id, column: Id, before: Option<Id>) -> bool {
        let Some(source) = self.columns.iter().position(|column| {
            column
                .cards
                .iter()
                .any(|card| card.id == id && !card.archived)
        }) else {
            return false;
        };
        let Some(target) = self
            .columns
            .iter()
            .position(|candidate| candidate.id == column)
        else {
            return false;
        };
        let cards = &self.columns[target].cards;
        let insertion = match before {
            Some(anchor) => {
                let Some(index) = cards
                    .iter()
                    .position(|card| card.id == anchor && !card.archived)
                else {
                    return false;
                };
                index
            }
            None => cards.len(),
        };
        let index = self.columns[source]
            .cards
            .iter()
            .position(|card| card.id == id)
            .unwrap();
        let insertion = insertion - usize::from(source == target && index < insertion);
        if source == target && index == insertion {
            return false;
        }
        let card = self.columns[source].cards.remove(index);
        self.columns[target].cards.insert(insertion, card);
        true
    }
}

impl Column {
    pub fn new(name: String, complete: bool) -> Self {
        Self {
            id: Id::new_v4(),
            name,
            complete,
            cards: Vec::new(),
        }
    }
}

impl Card {
    pub fn new(title: String) -> Self {
        Self {
            id: Id::new_v4(),
            title,
            description: String::new(),
            due: None,
            checklist: Vec::new(),
            archived: false,
            labels: Vec::new(),
        }
    }
}
