use ratatui::{Frame, macros::vertical, style::palette::material};

use crate::{
    helpers::{add_padding, create_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::{PopupTrait, Popups},
    screens::main_screen::FilterCriterion,
    types::Movie,
    widgets::{self, ActionTypes},
};

#[derive(Default)]
pub struct AdvancedFilterPopup {
    tab:             usize,
    item:            usize,
    filter_criteria: Vec<FilterCriterion>,
    movies:          Vec<Movie>,
}

impl AdvancedFilterPopup {
    pub fn new(filter_criteria: &[FilterCriterion]) -> Self {
        Self {
            tab:             0,
            item:            0,
            filter_criteria: filter_criteria.to_vec(),
            movies:          vec![],
        }
    }

    pub fn initialize(&mut self, movies: &[Movie]) {
        self.movies = movies.to_vec();
    }
}

impl PopupTrait for AdvancedFilterPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
    }
}
