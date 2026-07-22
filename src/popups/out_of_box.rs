use std::{cell::RefCell, rc::Rc};

use ratatui::{Frame, macros::vertical, style::palette::material};

use crate::{
    config::Config,
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    widgets::{self, ActionTypes},
};

pub struct OutOfBoxPopup {
    item:   usize,
    config: Rc<RefCell<Config>>,
}

impl OutOfBoxPopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        Self { item: 0, config }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
    }
}
