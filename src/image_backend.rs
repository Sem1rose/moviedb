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

use crate::{helpers, types::FxIndexMap};

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

const CALCULATE_OBSTRUCTION: bool = false;

pub struct RatatuiImage {
    sizes:         FxHashMap<ImageIDDiscriminants, [Size; 2]>,
    hashed_images: FxIndexMap<ImageID, Option<SlicedProtocol>>,

    draw_queue:    Vec<(ImageID, Rect, Option<SignedPosition>)>,
    overlay_areas: Vec<Rect>,

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

            draw_queue: vec![],
            overlay_areas: vec![],

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

    pub fn hash_image(&mut self, image_id: ImageID) {
        self.hashed_images.insert(image_id, None);

        _ = self.tx_load.send(Actions::Load(image_id));
    }

    pub fn update(&mut self) {
        self.draw_queue.clear();
        self.overlay_areas.clear();

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
        let size_index = matches!(
            image_id,
            ImageID::Movie(_, true) | ImageID::Collection(_, true)
        ) as usize;

        if sliced_pos.is_none() {
            let size = self.sizes.get_mut(&image_id.into()).unwrap();
            if size[size_index] != area.as_size() {
                size[size_index] = area.as_size();
                _ = self.tx_load.send(Actions::Resize(image_id.into(), *size));

                self.hashed_images.retain(|k, _| {
                    ImageIDDiscriminants::from(k) != image_id.into()
                        || match k {
                            ImageID::Movie(_, backdrop) => *backdrop as usize != size_index,
                            ImageID::Collection(_, backdrop) => *backdrop as usize != size_index,
                            ImageID::Person(_) => false,
                        }
                });

                return false;
            }
        }

        let mut drawn = false;
        if let Some(value) = self.hashed_images.get(&image_id) {
            if let Some(protocol) = value {
                if CALCULATE_OBSTRUCTION {
                    self.draw_queue.push((image_id, area, sliced_pos));
                } else {
                    let Size { width, height } = protocol.size();

                    let centered_area =
                        area.centered(constraint!(== width), constraint!(== height));
                    frame.render_widget(
                        SlicedImage::new(
                            protocol,
                            sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                        ),
                        centered_area,
                    );
                }

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

        drawn
    }

    pub fn add_overlay(&mut self, area: Rect) {
        self.overlay_areas.push(area);
    }

    pub fn render(&self, frame: &mut Frame) {
        if !CALCULATE_OBSTRUCTION {
            return;
        }

        if self.overlay_areas.is_empty() {
            for (image_id, area, sliced_pos) in &self.draw_queue {
                if let Some(value) = self.hashed_images.get(image_id) {
                    if let Some(protocol) = value {
                        let Size { width, height } = protocol.size();

                        let centered_area =
                            area.centered(constraint!(== width), constraint!(== height));
                        frame.render_widget(
                            SlicedImage::new(
                                protocol,
                                sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                            ),
                            centered_area,
                        );
                    }
                }
            }
        } else {
            let (obsructed, unobstructed): (
                Vec<&(ImageID, Rect, Option<SignedPosition>)>,
                Vec<&(ImageID, Rect, Option<SignedPosition>)>,
            ) = self
                .draw_queue
                .iter()
                .partition(|x| self.overlay_areas.iter().any(|y| y.intersects(x.1)));

            for (image_id, area, sliced_pos) in unobstructed {
                if let Some(value) = self.hashed_images.get(image_id) {
                    if let Some(protocol) = value {
                        let Size { width, height } = protocol.size();

                        let centered_area =
                            area.centered(constraint!(== width), constraint!(== height));
                        frame.render_widget(
                            SlicedImage::new(
                                protocol,
                                sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                            ),
                            centered_area,
                        );
                    }
                }
            }

            for (image_id, big_area, sliced_pos) in obsructed {
                let obstructions = self
                    .overlay_areas
                    .iter()
                    .filter(|x| x.intersects(*big_area));
                let mut areas = vec![*big_area];
                for obstruction in obstructions {
                    let mut new_areas = vec![];
                    for area in areas {
                        //     match (
                        //         obstruction.contains(area.as_position()),
                        //         obstruction.contains(
                        //             area.offset(Offset::new(area.width as i32, 0)).as_position(),
                        //         ),
                        //         obstruction.contains(
                        //             area.offset(Offset::new(0, area.height as i32))
                        //                 .as_position(),
                        //         ),
                        //         obstruction.contains(
                        //             area.offset(Offset::new(area.width as i32, area.height as i32))
                        //                 .as_position(),
                        //         ),
                        //     ) {
                        //         (false, false, false, true) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 area.y,
                        //                 area.width,
                        //                 obstruction.y - area.y,
                        //             ));
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 obstruction.y,
                        //                 obstruction.x - area.x,
                        //                 area.bottom() - obstruction.y,
                        //             ));
                        //         }
                        //         (false, false, true, false) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 area.y,
                        //                 area.width,
                        //                 obstruction.y - area.y,
                        //             ));
                        //             new_areas.push(Rect::new(
                        //                 obstruction.right(),
                        //                 obstruction.y,
                        //                 area.right() - obstruction.right(),
                        //                 area.bottom() - obstruction.y,
                        //             ));
                        //         }
                        //         (false, true, false, false) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 area.y,
                        //                 obstruction.x - area.x,
                        //                 obstruction.bottom() - area.y,
                        //             ));
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 obstruction.bottom(),
                        //                 area.width,
                        //                 area.bottom() - obstruction.bottom(),
                        //             ));
                        //         }
                        //         (true, false, false, false) => {
                        //             new_areas.push(Rect::new(
                        //                 obstruction.right(),
                        //                 area.y,
                        //                 area.right() - obstruction.right(),
                        //                 obstruction.bottom() - area.y,
                        //             ));
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 obstruction.bottom(),
                        //                 area.width,
                        //                 area.bottom() - obstruction.bottom(),
                        //             ));
                        //         }
                        //         (true, true, false, false) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 obstruction.bottom(),
                        //                 area.width,
                        //                 area.bottom() - obstruction.bottom(),
                        //             ));
                        //         }
                        //         (false, false, true, true) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 area.y,
                        //                 area.width,
                        //                 obstruction.y - area.y,
                        //             ));
                        //         }
                        //         (false, true, false, true) => {
                        //             new_areas.push(Rect::new(
                        //                 area.x,
                        //                 area.y,
                        //                 obstruction.x - area.x,
                        //                 area.height,
                        //             ));
                        //         }
                        //         (true, false, true, false) => {
                        //             new_areas.push(Rect::new(
                        //                 obstruction.right(),
                        //                 area.y,
                        //                 area.right() - obstruction.right(),
                        //                 area.height,
                        //             ));
                        //         }

                        //         (false, true, true, true) => (),
                        //         (true, false, true, true) => (),
                        //         (true, true, false, true) => (),
                        //         (true, true, true, false) => (),
                        //         (false, true, true, false) => (),
                        //         (true, false, false, true) => (),
                        //         (false, false, false, false) => (),
                        //         (true, true, true, true) => (),
                        //     }
                        let intersection = obstruction.intersection(area);
                        if intersection.x > area.x {
                            new_areas.push(Rect::new(
                                area.x,
                                intersection.y,
                                intersection.x - area.x,
                                intersection.height,
                            ));
                        }
                        if intersection.right() < area.right() {
                            new_areas.push(Rect::new(
                                intersection.right(),
                                intersection.y,
                                area.right() - intersection.right(),
                                intersection.height,
                            ));
                        }
                        if intersection.y > area.y {
                            new_areas.push(Rect::new(
                                area.x,
                                area.y,
                                area.width,
                                intersection.y - area.y,
                            ));
                        }
                        if intersection.bottom() < area.bottom() {
                            new_areas.push(Rect::new(
                                area.x,
                                intersection.bottom(),
                                area.width,
                                area.bottom() - intersection.bottom(),
                            ));
                        }
                    }

                    areas = new_areas;
                }

                if let Some(value) = self.hashed_images.get(image_id) {
                    if let Some(protocol) = value {
                        let Size { width, height } = protocol.size();
                        let centered_big_area =
                            big_area.centered(constraint!(== width), constraint!(== height));

                        for area in areas.into_iter().filter(|x| x.height > 0 && x.width > 1) {
                            frame.render_widget(
                                SlicedImage::new(
                                    protocol,
                                    helpers::signed_pos_add(
                                        sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                                        helpers::signed_subtract_pos(
                                            centered_big_area.as_position(),
                                            area.as_position(),
                                        ),
                                    ),
                                ),
                                area,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn update_access_token(&self, access_token: &str) {
        _ = self
            .tx_load
            .send(Actions::UpdateTokens(access_token.to_string()))
    }
}
