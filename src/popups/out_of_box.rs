use ratatui::{Frame, macros::vertical, style::palette::material};

use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    widgets::{self, ActionTypes},
};

#[derive(Default)]
pub struct OutOfBoxPopup {
    item: usize,
}

impl OutOfBoxPopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new() -> Self {
        Self { item: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
    }
}
