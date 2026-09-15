use std::{
    fs,
    path::Path,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use anyhow::anyhow;
use log::error;

use crate::{
    event_handler::EventHandler,
    processors::ProcessorTrait,
    types::{Collection, Entry, Movie, Person},
};

enum Operation {
    SaveDBs(
        Option<Vec<Movie>>,
        Option<Vec<Entry>>,
        Option<Vec<Person>>,
        Option<Vec<Collection>>,
    ),
}

#[derive(Default)]
pub struct FileWriterProcessor {
    initialized: bool,

    tx_operation_request:  Option<Sender<Operation>>,
    rx_operation_response: Option<Receiver<anyhow::Result<()>>>,
}

impl FileWriterProcessor {
    pub fn initialize(&mut self, home_dir: &Path) {
        if self.initialized {
            return;
        }

        let (tx_operation_request, rx_operation_request) = channel::<Operation>();
        let (tx_operation_response, rx_operation_response) = channel::<anyhow::Result<()>>();

        let home_dir = home_dir.to_path_buf();
        thread::spawn(move || {
            for operation in rx_operation_request.iter() {
                match operation {
                    Operation::SaveDBs(movies, watched, persons, collections) => {
                        macro_rules! try_save {
                            ($obj:expr) => {
                                if let Some(data) = $obj {
                                    let name = stringify!($obj);
                                    let path = &home_dir.join(format!("{name}.json"));
                                    match serde_json::to_string(&data) {
                                        Err(error) =>
                                            _ = tx_operation_response.send(Err(anyhow!(
                                                "Error while trying to serialize {name}: {error}"
                                            ))),
                                        Ok(serialized) => {
                                            _ = fs::rename(
                                                path,
                                                home_dir.join(name).with_extension("json.bak"),
                                            );
                                            if let Err(error) = fs::write(path, serialized) {
                                                _ = tx_operation_response.send(Err(anyhow!(
                                                    "Error while trying to save {name}: {error}"
                                                )));
                                            }
                                        }
                                    }
                                }
                            };
                        }

                        try_save!(movies);
                        try_save!(watched);
                        try_save!(persons);
                        try_save!(collections);
                    }
                }
            }
        });

        *self = Self {
            initialized: true,

            tx_operation_request: Some(tx_operation_request),
            rx_operation_response: Some(rx_operation_response),

            ..Default::default()
        }
    }

    pub fn save_data(
        &self,
        movies: Option<Vec<Movie>>,
        watched: Option<Vec<Entry>>,
        persons: Option<Vec<Person>>,
        collections: Option<Vec<Collection>>,
    ) {
        if let Err(error) = self
            .tx_operation_request
            .as_ref()
            .unwrap()
            .send(Operation::SaveDBs(movies, watched, persons, collections))
        {
            error!("Error: {error:?}")
        }
    }
}

impl ProcessorTrait for FileWriterProcessor {
    fn update(&mut self, _: &mut EventHandler) {
        if !self.initialized {
            return;
        }

        for result in self.rx_operation_response.as_ref().unwrap().try_iter() {
            match result {
                Ok(_) => (),
                Err(error) => error!("Error while executing operation: {error:?}"),
            }
        }
    }
}
