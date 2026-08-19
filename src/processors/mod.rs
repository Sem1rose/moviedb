mod movies_fetcher;

use itertools::Itertools;
use movies_fetcher::MoviesFetcherProcessor;
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoEnumIterator};

use crate::key_event_handler::KeyEventHandler;

#[derive(EnumDiscriminants, EnumCount, EnumIter)]
#[strum_discriminants(derive(Hash))]
pub enum Processor {
    DetailsFetcher(Box<MoviesFetcherProcessor>),
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
            Processor::DetailsFetcher(movies_fetcher_processsor) => &**movies_fetcher_processsor,
        }
    }

    fn as_trait_mut(&mut self) -> &mut dyn ProcessorTrait {
        match self {
            Processor::DetailsFetcher(movies_fetcher_processsor) =>
                &mut **movies_fetcher_processsor,
        }
    }

    pub fn update(&mut self, key_event_handler: &mut KeyEventHandler) {
        self.as_trait_mut().update(key_event_handler)
    }
}

pub trait ProcessorTrait {
    fn update(&mut self, key_event_handler: &mut KeyEventHandler);
}
