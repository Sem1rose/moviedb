use anyhow::bail;
use log::error;
use ratatui::{
    layout::{Constraint, Rect, Size},
    Frame,
};
use ratatui_image::{picker::Picker, protocol::Protocol, Image, Resize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::helpers::center_rect;

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct ArtworkID {
    pub tmdb_id: u32,
    pub backdrop: bool,
}

type LoadResult = (ArtworkID, anyhow::Result<Protocol>);

enum Actions {
    Load(ArtworkID, String),
    ResizeArtwork(Size),
    ResizeBackdrop(Size),
}

pub struct RatatuiImage {
    hashed_images: HashMap<ArtworkID, Option<Protocol>>,
    preload_images: Vec<ArtworkID>,

    tx_load: Sender<Actions>,
    rx_main: Receiver<LoadResult>,

    artwork_size: Option<Size>,
    backdrop_size: Option<Size>,
    cache_dir: PathBuf,

    pub loading: u32,
}
impl RatatuiImage {
    pub fn new(cache_dir: &PathBuf) -> Self {
        let (tx_main, rx_main) = mpsc::channel();

        let tx_load = Self::start_load_thread(&tx_main);

        Self {
            hashed_images: HashMap::new(),
            preload_images: vec![],
            rx_main,
            tx_load,
            artwork_size: None,
            backdrop_size: None,
            loading: 0,
            cache_dir: cache_dir.clone(),
        }
    }
    fn start_load_thread(tx_main: &Sender<LoadResult>) -> Sender<Actions> {
        let (tx_load, rx_load) = mpsc::channel::<Actions>();

        let tx_main = tx_main.clone();
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
            error!("error querying graphics capabilities");
            Picker::halfblocks()
        });

        thread::spawn(move || {
            let mut artwork_size: Size = Size::default();
            let mut backdrop_size: Size = Size::default();

            for action in rx_load.iter() {
                match action {
                    Actions::Load(artwork_id, path) => {
                        let tx_main = tx_main.clone();

                        let _picker = picker.clone();
                        thread::spawn(move || {
                            let result = (|| -> anyhow::Result<_> {
                                let decoded;
                                let reader;
                                let result = image::ImageReader::open(&path);
                                if let Err(err) = result {
                                    bail!("Failed to open {:?}: {}", artwork_id, err);
                                } else {
                                    reader = result.unwrap();
                                }

                                let result = reader.decode();
                                if let Err(err) = result {
                                    bail!("Failed to decode {:?}: {}", artwork_id, err);
                                } else {
                                    decoded = result.unwrap();
                                }

                                let protocol = _picker.new_protocol(
                                    decoded,
                                    Rect {
                                        x: 0,
                                        y: 0,
                                        width: if artwork_id.backdrop {
                                            backdrop_size.width
                                        } else {
                                            artwork_size.width
                                        },
                                        height: if artwork_id.backdrop {
                                            backdrop_size.height
                                        } else {
                                            artwork_size.height
                                        },
                                    },
                                    Resize::Scale(Some(ratatui_image::FilterType::Triangle)),
                                )?;

                                Ok(protocol)
                            })();

                            tx_main.send((artwork_id, result))
                        });
                    }
                    Actions::ResizeArtwork(_size) => {
                        artwork_size = _size;
                    }
                    Actions::ResizeBackdrop(_size) => {
                        backdrop_size = _size;
                    }
                }
            }
        });

        tx_load
    }

    fn hash_image(&mut self, artwork_id: ArtworkID) {
        self.hashed_images.insert(artwork_id, None);

        let path = format!(
            "{}",
            if artwork_id.backdrop {
                self.cache_dir.join("backdrops")
            } else {
                self.cache_dir.join("posters")
            }
            .join(format!("{}.jpg", artwork_id.tmdb_id))
            .display()
        );

        _ = self.tx_load.send(Actions::Load(artwork_id, path));
        self.loading += 1;
    }

    pub fn update(&mut self) {
        for (artwork_id, result) in self.rx_main.try_iter() {
            if let Ok(protocol) = result {
                if self.hashed_images.contains_key(&artwork_id) {
                    _ = self
                        .hashed_images
                        .get_mut(&artwork_id)
                        .unwrap()
                        .insert(protocol);
                    self.loading -= 1;
                }
            } else if let Err(e) = result {
                error!("{e}");

                _ = self.tx_load.send(Actions::Load(
                    artwork_id,
                    format!(
                        "{}",
                        if artwork_id.backdrop {
                            self.cache_dir.join("backdrops")
                        } else {
                            self.cache_dir.join("posters")
                        }
                        .join(format!("{}.jpg", artwork_id.tmdb_id))
                        .display()
                    ),
                ));
            }
        }
    }

    pub fn draw_image(
        &mut self,
        tmdb_id: u32,
        backdrop: bool,
        area: Rect,
        frame: &mut Frame,
    ) -> bool {
        let artwork_id = ArtworkID { tmdb_id, backdrop };
        let mut drawn = false;
        if backdrop {
            if self.backdrop_size.is_none() {
                _ = self.tx_load.send(Actions::ResizeBackdrop(area.as_size()));
                self.backdrop_size = Some(area.as_size());
            } else if self.backdrop_size.unwrap() != area.as_size() {
                _ = self.tx_load.send(Actions::ResizeBackdrop(area.as_size()));
                self.backdrop_size = Some(area.as_size());

                let mut cached = self.hashed_images.clone();
                self.hashed_images.retain(|k, _| !k.backdrop);
                for (path, _) in cached.drain().filter(|(k, _)| k.backdrop) {
                    self.hash_image(path);
                }

                return false;
            }
        } else {
            if self.artwork_size.is_none() {
                _ = self.tx_load.send(Actions::ResizeArtwork(area.as_size()));
                self.artwork_size = Some(area.as_size());
            } else if self.artwork_size.unwrap() != area.as_size() {
                _ = self.tx_load.send(Actions::ResizeArtwork(area.as_size()));
                self.artwork_size = Some(area.as_size());

                let mut cached = self.hashed_images.clone();
                self.hashed_images.retain(|k, _| k.backdrop);
                for (path, _) in cached.drain().filter(|(k, _)| !k.backdrop) {
                    self.hash_image(path);
                }

                return false;
            }
        }

        if let Some(value) = self.hashed_images.get(&artwork_id) {
            if let Some(protocol) = value {
                let Size { width, height } = protocol.area().as_size();

                frame.render_widget(
                    Image::new(protocol),
                    center_rect(area, Constraint::Length(width), Constraint::Length(height)),
                );
                drawn = true;
            }
        } else {
            self.hash_image(artwork_id);
        }

        let preload_images = self.preload_images.clone();
        self.preload_images.clear();
        for artwork_id in preload_images {
            if let None = self.hashed_images.get(&artwork_id) {
                self.hash_image(artwork_id);
            }
        }

        return drawn;
    }

    pub fn preload_images(&mut self, items: &[u32]) {
        self.preload_images = items
            .into_iter()
            .map(|&id| ArtworkID {
                tmdb_id: id,
                backdrop: false,
            })
            .collect();
    }
}
