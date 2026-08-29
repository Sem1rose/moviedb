mod history_syncer;

use history_syncer::HistorySyncerProcessor;
use itertools::Itertools;
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoEnumIterator};

use crate::key_event_handler::KeyEventHandler;

#[derive(EnumDiscriminants, EnumCount, EnumIter)]
#[strum_discriminants(derive(Hash))]
pub enum Processor {
    HistorySyncer(Box<HistorySyncerProcessor>),
}

#[macro_export]
macro_rules! new_processor {
    ($processor_enum:ident, $T:expr) => {
        Processor::$processor_enum(Box::new($T))
    };
    ($processor_enum:ident) => {
        Processor::$processor_enum(Box::default())
    };
}

impl Processor {
    pub fn default_all() -> [Processor; Processor::COUNT] {
        Processor::iter().collect_array().unwrap()
    }

    fn as_trait(&self) -> &dyn ProcessorTrait {
        match self {
            Processor::HistorySyncer(history_syncer_processsor) => &**history_syncer_processsor,
        }
    }

    fn as_trait_mut(&mut self) -> &mut dyn ProcessorTrait {
        match self {
            Processor::HistorySyncer(history_syncer_processsor) => &mut **history_syncer_processsor,
        }
    }

    pub fn update(&mut self, key_event_handler: &mut KeyEventHandler) {
        self.as_trait_mut().update(key_event_handler)
    }

    pub fn needs_render(&self) -> bool {
        self.as_trait().needs_render()
    }

    pub fn render(&self, frame: &mut ratatui::Frame, key_event_handler: &mut KeyEventHandler) {
        self.as_trait().render(frame, key_event_handler)
    }
}

pub trait ProcessorTrait {
    fn update(&mut self, key_event_handler: &mut KeyEventHandler);
    fn needs_render(&self) -> bool;
    fn render(&self, frame: &mut ratatui::Frame, key_event_handler: &mut KeyEventHandler);
}
