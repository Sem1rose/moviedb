mod file_writer;
mod history_syncer;
mod tokens_refresher;

use file_writer::FileWriterProcessor;
use history_syncer::HistorySyncerProcessor;
use itertools::Itertools;
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoEnumIterator};
use tokens_refresher::TokensRefresherProcessor;

use crate::{event_handler::EventHandler, image_backend::RatatuiImage};

#[derive(EnumDiscriminants, EnumCount, EnumIter)]
#[strum_discriminants(derive(Hash))]
pub enum Processor {
    FileWriter(Box<FileWriterProcessor>),
    HistorySyncer(Box<HistorySyncerProcessor>),
    TokensRefresher(Box<TokensRefresherProcessor>),
}

impl Processor {
    pub fn default_all() -> [Processor; Processor::COUNT] {
        Processor::iter().collect_array().unwrap()
    }

    fn as_trait(&self) -> &dyn ProcessorTrait {
        match self {
            Processor::FileWriter(file_writer_processor) => &**file_writer_processor,
            Processor::HistorySyncer(history_syncer_processsor) => &**history_syncer_processsor,
            Processor::TokensRefresher(tokens_refresher_processor) => &**tokens_refresher_processor,
        }
    }

    fn as_trait_mut(&mut self) -> &mut dyn ProcessorTrait {
        match self {
            Processor::FileWriter(file_writer_processor) => &mut **file_writer_processor,
            Processor::HistorySyncer(history_syncer_processsor) => &mut **history_syncer_processsor,
            Processor::TokensRefresher(tokens_refresher_processor) =>
                &mut **tokens_refresher_processor,
        }
    }

    pub fn needs_render(&self) -> bool {
        self.as_trait().needs_render()
    }

    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        self.as_trait().get_state()
    }

    pub fn active_operation(&self) -> Option<[String; 2]> {
        self.as_trait().active_operation()
    }

    pub fn update(&mut self, event_handler: &mut EventHandler) {
        self.as_trait_mut().update(event_handler)
    }

    pub fn render(
        &self,
        frame: &mut ratatui::Frame,
        image_renderer: &mut RatatuiImage,
        event_handler: &mut EventHandler,
    ) {
        self.as_trait().render(frame, image_renderer, event_handler)
    }
}

pub trait ProcessorTrait {
    fn needs_render(&self) -> bool {
        false
    }
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, None)
    }
    fn active_operation(&self) -> Option<[String; 2]> {
        None
    }

    fn update(&mut self, event_handler: &mut EventHandler);
    fn render(
        &self,
        frame: &mut ratatui::Frame,
        image_renderer: &mut RatatuiImage,
        event_handler: &mut EventHandler,
    );
}
