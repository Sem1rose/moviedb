mod history_syncer;
mod tokens_refresher;

use history_syncer::HistorySyncerProcessor;
use itertools::Itertools;
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoEnumIterator};
use tokens_refresher::TokensRefresherProcessor;

use crate::{event_handler::EventHandler, image_backend::RatatuiImage};

#[derive(EnumDiscriminants, EnumCount, EnumIter)]
#[strum_discriminants(derive(Hash))]
pub enum Processor {
    HistorySyncer(Box<HistorySyncerProcessor>),
    TokensRefresher(Box<TokensRefresherProcessor>),
}

impl Processor {
    pub fn default_all() -> [Processor; Processor::COUNT] {
        Processor::iter().collect_array().unwrap()
    }

    fn as_trait(&self) -> &dyn ProcessorTrait {
        match self {
            Processor::HistorySyncer(history_syncer_processsor) => &**history_syncer_processsor,
            Processor::TokensRefresher(tokens_refresher_processor) => &**tokens_refresher_processor,
        }
    }

    fn as_trait_mut(&mut self) -> &mut dyn ProcessorTrait {
        match self {
            Processor::HistorySyncer(history_syncer_processsor) => &mut **history_syncer_processsor,
            Processor::TokensRefresher(tokens_refresher_processor) =>
                &mut **tokens_refresher_processor,
        }
    }

    pub fn update(&mut self, key_event_handler: &mut EventHandler) {
        self.as_trait_mut().update(key_event_handler)
    }

    pub fn needs_render(&self) -> bool {
        self.as_trait().needs_render()
    }

    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        self.as_trait().get_state()
    }

    pub fn render(
        &self,
        frame: &mut ratatui::Frame,
        key_event_handler: &mut EventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        self.as_trait()
            .render(frame, key_event_handler, image_renderer)
    }
}

pub trait ProcessorTrait {
    fn update(&mut self, key_event_handler: &mut EventHandler);
    fn needs_render(&self) -> bool {
        false
    }
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, None)
    }
    fn render(
        &self,
        frame: &mut ratatui::Frame,
        key_event_handler: &mut EventHandler,
        image_renderer: &mut RatatuiImage,
    );
}
