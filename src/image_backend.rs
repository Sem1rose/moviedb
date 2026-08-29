use std::{
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use anyhow::bail;
use log::error;
use ratatui::{
    Frame,
    layout::{Rect, Size},
    macros::constraint,
    style::{Style, Stylize, palette::tailwind},
    widgets::Block,
};
use ratatui_image::{Resize, picker::Picker, sliced::*};
use rustc_hash::FxHashMap;
use strum::EnumDiscriminants;
use throbber_widgets_tui::{BRAILLE_SIX_DOUBLE, Throbber, ThrobberState};

use crate::types::FxIndexMap;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, EnumDiscriminants)]
#[strum_discriminants(derive(Hash))]
pub enum ImageID {
    Movie(u32, bool),
    Collection(u32, bool),
    Person(u32),
}

type LoadResult = (ImageID, anyhow::Result<Result<SlicedProtocol, bool>>);

enum Actions {
    Load(ImageID),
    Resize(ImageIDDiscriminants, [Size; 2]),
    UpdateTokens(String),
}

fn default_sizes() -> FxHashMap<ImageIDDiscriminants, [Size; 2]> {
    FxHashMap::from_iter([
        (ImageIDDiscriminants::Movie, Default::default()),
        (ImageIDDiscriminants::Collection, Default::default()),
        (ImageIDDiscriminants::Person, Default::default()),
    ])
}

pub struct RatatuiImage {
    sizes:         FxHashMap<ImageIDDiscriminants, [Size; 2]>,
    hashed_images: FxIndexMap<ImageID, Option<SlicedProtocol>>,

    tx_load: Sender<Actions>,
    rx_main: Receiver<LoadResult>,
}
impl RatatuiImage {
    pub fn new(cache_dir: &Path) -> Self {
        let (tx_main, rx_main) = mpsc::channel();
        let tx_load = Self::start_load_thread(&tx_main, cache_dir);

        Self {
            sizes: default_sizes(),
            hashed_images: FxIndexMap::with_capacity_and_hasher(100, rustc_hash::FxBuildHasher),

            tx_load,
            rx_main,
        }
    }

    fn start_load_thread(tx_main: &Sender<LoadResult>, cache_dir: &Path) -> Sender<Actions> {
        let (tx_load, rx_load) = mpsc::channel::<Actions>();

        let tx_main = tx_main.clone();
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| {
            error!("error querying graphics capabilities");
            Picker::halfblocks()
        });
        // let picker = Picker::halfblocks();

        let cache_dir = cache_dir.to_path_buf();
        let mut tmdb_access_token: Option<String> = None;
        let mut sizes = default_sizes();
        thread::spawn(move || {
            for action in rx_load.iter() {
                match action {
                    Actions::Load(image_id) => {
                        let tx_main = tx_main.clone();

                        let path = match image_id {
                            ImageID::Movie(id, backdrop) => if backdrop {
                                cache_dir.join("backdrops")
                            } else {
                                cache_dir.join("posters")
                            }
                            .join(id.to_string())
                            .with_extension("jpg"),
                            ImageID::Collection(id, _backdrop) => cache_dir
                                .join("collections")
                                .join(id.to_string())
                                .with_extension("jpg"),
                            ImageID::Person(id) => cache_dir
                                .join("persons")
                                .join(id.to_string())
                                .with_extension("jpg"),
                        };

                        if path.is_file() {
                            let size = sizes[&image_id.into()][matches!(
                                image_id,
                                ImageID::Movie(_, true) | ImageID::Collection(_, true)
                            )
                                as usize];

                            let picker = picker.clone();
                            thread::spawn(move || {
                                let result = (|| -> anyhow::Result<_> {
                                    let result = image::ImageReader::open(&path);
                                    if let Err(err) = result {
                                        bail!("Failed to open {:?}: {}", image_id, err);
                                    }

                                    let result = result.unwrap().decode();
                                    if let Err(err) = result {
                                        bail!("Failed to decode {:?}: {}", image_id, err);
                                    }

                                    let protocol = SlicedProtocol::new_with_resize(
                                        &picker,
                                        result.unwrap(),
                                        size,
                                        Resize::Scale(Some(ratatui_image::FilterType::Triangle)),
                                    )?;

                                    Ok(Ok(protocol))
                                })();

                                tx_main.send((image_id, result))
                            });
                        } else {
                            let cache_dir = cache_dir.clone();
                            let tmdb_access_token = tmdb_access_token.as_ref().unwrap().clone();
                            thread::spawn(move || {
                                let result = {
                                    let result = match image_id {
                                        ImageID::Movie(id, false) =>
                                            tmdb::movie::get_movie_artworks(
                                                &cache_dir,
                                                tmdb_access_token.as_str(),
                                                None,
                                                id,
                                            ),
                                        ImageID::Collection(id, false) =>
                                            tmdb::collection::get_collection_artwork(
                                                &cache_dir,
                                                tmdb_access_token.as_str(),
                                                id,
                                            ),
                                        ImageID::Person(id) => tmdb::movie::get_person_artwork(
                                            &cache_dir,
                                            tmdb_access_token.as_str(),
                                            id,
                                        ),
                                        _ => Ok(false),
                                    }
                                    .ok()
                                    .unwrap_or(false);

                                    Ok(Err(result))
                                };

                                tx_main.send((image_id, result))
                            });
                        }
                    }
                    Actions::Resize(id, new_sizes) => {
                        *sizes.get_mut(&id).unwrap() = new_sizes;
                    }
                    Actions::UpdateTokens(access_token) => {
                        tmdb_access_token = Some(access_token);
                    }
                }
            }
        });

        tx_load
    }

    fn hash_image(&mut self, image_id: ImageID) {
        self.hashed_images.insert(image_id, None);

        _ = self.tx_load.send(Actions::Load(image_id));
    }

    pub fn update(&mut self) {
        for (image_id, result) in self.rx_main.try_iter() {
            if let Ok(protocol) = result {
                if self.hashed_images.contains_key(&image_id) {
                    if let Ok(protocol) = protocol {
                        _ = self
                            .hashed_images
                            .get_mut(&image_id)
                            .unwrap()
                            .insert(protocol);
                    } else if let Err(true) = protocol {
                        // downloaded successfully
                        match image_id {
                            ImageID::Movie(id, _) => {
                                _ = self.tx_load.send(Actions::Load(ImageID::Movie(id, true)));
                                _ = self.tx_load.send(Actions::Load(ImageID::Movie(id, false)));
                            }
                            ImageID::Collection(id, _) => {
                                // _ = self.tx_load.send(Actions::Load(ImageID::Collection(id, true)));
                                _ = self
                                    .tx_load
                                    .send(Actions::Load(ImageID::Collection(id, false)));
                            }
                            ImageID::Person(id) => {
                                _ = self.tx_load.send(Actions::Load(ImageID::Person(id)));
                            }
                        }
                    }
                }
            } else if let Err(error) = result {
                error!("error loading image {image_id:?}: {error:#?}");
                _ = self.tx_load.send(Actions::Load(image_id));
            }
        }

        while self.hashed_images.len() > 100 {
            _ = self.hashed_images.shift_remove_index(0).unwrap();
        }
    }

    pub fn draw_image(
        &mut self,
        image_id: ImageID,
        area: Rect,
        sliced_pos: Option<SignedPosition>,
        throbber_state: &mut ThrobberState,
        frame: &mut Frame,
    ) -> bool {
        // macro_rules! pop_then_hash {
        //     ($collection:expr, $filter_map:expr, $retain:expr) => {
        //         let hash = $collection.iter().filter_map($filter_map).collect_vec();
        //         $collection.retain($retain);
        //         for artwork_id in hash {
        //             if self.hashed_images.get(&artwork_id).is_none() {
        //                 self.hash_image(artwork_id);
        //             }
        //         }
        //     };
        // }

        let index = matches!(
            image_id,
            ImageID::Movie(_, true) | ImageID::Collection(_, true)
        ) as usize;
        if sliced_pos.is_none() {
            let size = self.sizes.get_mut(&image_id.into()).unwrap();
            if size[index] != area.as_size() {
                size[index] = area.as_size();
                _ = self.tx_load.send(Actions::Resize(image_id.into(), *size));

                self.hashed_images.retain(|k, _| {
                    ImageIDDiscriminants::from(k) != image_id.into()
                        || match k {
                            ImageID::Movie(_, backdrop) => *backdrop as usize != index,
                            ImageID::Collection(_, backdrop) => *backdrop as usize != index,
                            ImageID::Person(_) => false,
                        }
                });
                // pop_then_hash!(
                //     self.hashed_images,
                //     |(k, _)| {
                //         (ImageIDDiscriminants::from(*k) == image_id.into())
                //             .then_some(match k {
                //                 ImageID::Movie(_, backdrop) =>
                //                     (*backdrop as usize == index).then_some(*k),
                //                 ImageID::Collection(_, backdrop) =>
                //                     (*backdrop as usize == index).then_some(*k),
                //                 ImageID::Person(_) => Some(*k),
                //             })
                //             .flatten()
                //     },
                //     |k, _| {
                //         ImageIDDiscriminants::from(k) != image_id.into()
                //             || match k {
                //                 ImageID::Movie(_, backdrop) => *backdrop as usize != index,
                //                 ImageID::Collection(_, backdrop) => *backdrop as usize != index,
                //                 ImageID::Person(_) => false,
                //             }
                //     }
                // );

                // pop_then_hash!(
                //     self.preload_images,
                //     |k| {
                //         (ImageIDDiscriminants::from(*k) == image_id.into())
                //             .then_some(match k {
                //                 ImageID::Movie(_, backdrop) =>
                //                     (*backdrop as usize == index).then_some(*k),
                //                 ImageID::Collection(_, backdrop) =>
                //                     (*backdrop as usize == index).then_some(*k),
                //                 ImageID::Person(_) => Some(*k),
                //             })
                //             .flatten()
                //     },
                //     |k| {
                //         ImageIDDiscriminants::from(k) != image_id.into()
                //             || match k {
                //                 ImageID::Movie(_, backdrop) => *backdrop as usize != index,
                //                 ImageID::Collection(_, backdrop) => *backdrop as usize != index,
                //                 ImageID::Person(_) => false,
                //             }
                //     }
                // );

                return false;
            }
        }

        let mut drawn = false;
        if let Some(value) = self.hashed_images.get(&image_id) {
            if let Some(protocol) = value {
                let Size { width, height } = protocol.size();

                let centered_area = area.centered(constraint!(== width), constraint!(== height));
                frame.render_widget(
                    SlicedImage::new(
                        protocol,
                        sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                    ),
                    centered_area,
                );

                drawn = true;
            } else {
                frame.render_widget(Block::new().bg(tailwind::GRAY.c950), area);
                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(BRAILLE_SIX_DOUBLE)
                        .style(Style::new().fg(tailwind::CYAN.c600).bold()),
                    area.centered(constraint!(==1), constraint!(==1)),
                    throbber_state,
                );
            }
        } else {
            self.hash_image(image_id);
        }
        if drawn {
            self.hashed_images.move_index(
                self.hashed_images.get_index_of(&image_id).unwrap(),
                self.hashed_images.len() - 1,
            );
        }

        // pop_then_hash!(
        //     self.preload_images,
        //     |k| {
        //         (ImageIDDiscriminants::from(*k) == image_id.into())
        //             .then_some(match k {
        //                 ImageID::Movie(_, backdrop) => (*backdrop as usize == index).then_some(*k),
        //                 ImageID::Collection(_, backdrop) =>
        //                     (*backdrop as usize == index).then_some(*k),
        //                 ImageID::Person(_) => Some(*k),
        //             })
        //             .flatten()
        //     },
        //     |k| {
        //         ImageIDDiscriminants::from(k) != image_id.into()
        //             || match k {
        //                 ImageID::Movie(_, backdrop) => *backdrop as usize != index,
        //                 ImageID::Collection(_, backdrop) => *backdrop as usize != index,
        //                 ImageID::Person(_) => false,
        //             }
        //     }
        // );

        drawn
    }

    // pub fn preload_movies(&mut self, movies: Vec<u32>, rule: &str) {
    //     match rule {
    //         "all" => {
    //             self.preload_images = movies.iter().map(|&id| ImageID::Movie(id, false)).collect();
    //             self.preload_images
    //                 .extend(movies.into_iter().map(|id| ImageID::Movie(id, true)));
    //         }
    //         "posters" => {
    //             self.preload_images = movies.iter().map(|&id| ImageID::Movie(id, false)).collect();
    //         }
    //         _ => (),
    //     }
    // }

    pub fn update_access_token(&self, access_token: &str) {
        _ = self
            .tx_load
            .send(Actions::UpdateTokens(access_token.to_string()))
    }
}
