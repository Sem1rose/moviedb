use std::{cell::RefCell, rc::Rc};

use ratatui::{Frame, macros::vertical, style::palette::material};

use crate::{
    config::Config,
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::{PopupTrait, Popups},
    widgets::{self, ActionTypes},
};

#[derive(Default, Debug)]
pub enum Phase {
    #[default]
    Initializing,
    Done,
}

pub struct OutOfBoxPopup {
    pub phase: Phase,
    item:      usize,
    config:    Rc<RefCell<Config>>,
}

impl OutOfBoxPopup {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        Self {
            item: 0,
            config,
            phase: Phase::default(),
        }
    }
}

impl PopupTrait for OutOfBoxPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
    }
}
