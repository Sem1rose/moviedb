use std::{
    collections::VecDeque,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use anyhow::{anyhow, bail};
use itertools::Either;
use log::{error, info};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Rect, Size},
    macros::constraint,
    style::{Style, Stylize, palette::tailwind},
    widgets::{Fill, StatefulWidget, Widget},
};
use ratatui_image::{Resize, picker::Picker, sliced::*};
use rustc_hash::{FxHashMap, FxHashSet};
use strum::{EnumDiscriminants, IntoDiscriminant};
use throbber_widgets_tui::{BRAILLE_SIX_DOUBLE, Throbber, ThrobberState};

use crate::helpers;

#[derive(PartialEq, Eq, Clone, Debug, EnumDiscriminants)]
#[strum_discriminants(derive(Hash))]
pub enum ImageID {
    Movie(u32, Option<String>, bool),
    Collection(u32, bool),
    Person(u32),
    Custom(String, bool),
}
impl Hash for ImageID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ImageID::Movie(id, _, backdrop) => (id, backdrop).hash(state),
            _ => core::mem::discriminant(self).hash(state),
        }
    }
}

type LoadResult = (ImageID, anyhow::Result<Either<SlicedProtocol, bool>>);

#[derive(Debug)]
enum Actions {
    DownLoad(ImageID),
    Load(ImageID),
    Resize(ImageIDDiscriminants, [Size; 2]),
    UpdateTokens(String),
}

fn default_sizes() -> FxHashMap<ImageIDDiscriminants, [Size; 2]> {
    FxHashMap::from_iter([
        (ImageIDDiscriminants::Movie, Default::default()),
        (ImageIDDiscriminants::Collection, Default::default()),
        (ImageIDDiscriminants::Person, Default::default()),
        (ImageIDDiscriminants::Custom, Default::default()),
    ])
}

const CALCULATE_OBSTRUCTION: bool = true;
const MAX_CONCURRENT_LOADS: usize = 20;

pub struct RatatuiImage {
    sizes:           FxHashMap<ImageIDDiscriminants, [Size; 2]>,
    hashed_images:   FxHashMap<ImageID, SlicedProtocol>,
    load_queue:      VecDeque<ImageID>,
    loading_ids:     FxHashSet<ImageID>,
    downloading_ids: FxHashSet<ImageID>,
    download_errors: FxHashSet<ImageID>,

    draw_queue:    Vec<(ImageID, Rect, Option<SignedPosition>)>,
    overlay_areas: Vec<Rect>,

    tx_load: Sender<Actions>,
    rx_main: Receiver<LoadResult>,

    cache_dir: PathBuf,

    tick:             u64,
    throbber_state:   ThrobberState,
    pub images_drawn: bool,
}
impl RatatuiImage {
    pub fn new(cache_dir: &Path) -> Self {
        let (rx_main, tx_load) = Self::start_load_thread(cache_dir);

        Self {
            sizes: default_sizes(),
            hashed_images: FxHashMap::default(),
            load_queue: Default::default(),
            loading_ids: FxHashSet::with_capacity_and_hasher(
                MAX_CONCURRENT_LOADS,
                rustc_hash::FxBuildHasher,
            ),
            downloading_ids: FxHashSet::default(),
            download_errors: Default::default(),

            draw_queue: vec![],
            overlay_areas: vec![],

            tx_load,
            rx_main,

            cache_dir: cache_dir.to_path_buf(),

            tick: 0,
            throbber_state: Default::default(),
            images_drawn: false,
        }
    }

    fn start_load_thread(cache_dir: &Path) -> (Receiver<LoadResult>, Sender<Actions>) {
        let (tx_load, rx_load) = mpsc::channel::<Actions>();
        let (tx_main, rx_main) = mpsc::channel::<LoadResult>();

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

                        let size = sizes[&image_id.discriminant()][matches!(
                            image_id,
                            ImageID::Movie(_, _, true)
                                | ImageID::Collection(_, true)
                                | ImageID::Custom(_, true)
                        )
                            as usize];

                        if size.width == 0 || size.height == 0 {
                            _ = tx_main.send((image_id, Err(anyhow!("Size not initialized."))));
                            continue;
                        }

                        let path = Self::path_from_image_id(&image_id, &cache_dir);
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

                                Ok(Either::Left(protocol))
                            })();

                            tx_main.send((image_id, result))
                        });
                    }
                    Actions::DownLoad(image_id) => {
                        let tx_main = tx_main.clone();

                        let cache_dir = cache_dir.clone();
                        let tmdb_access_token = tmdb_access_token.as_ref().unwrap().clone();
                        thread::spawn(move || {
                            let result = match &image_id {
                                ImageID::Movie(id, Some(path), backdrop) =>
                                    tmdb::movie::get_custom_artwork(
                                        &cache_dir,
                                        tmdb_access_token.as_str(),
                                        Some(*id),
                                        path,
                                        *backdrop,
                                    ),
                                &ImageID::Movie(id, None, backdrop) =>
                                    tmdb::movie::get_movie_artworks(
                                        &cache_dir,
                                        tmdb_access_token.as_str(),
                                        None,
                                        id,
                                        Some(backdrop),
                                    ),
                                &ImageID::Collection(id, false) =>
                                    tmdb::collection::get_collection_artwork(
                                        &cache_dir,
                                        tmdb_access_token.as_str(),
                                        id,
                                    ),
                                &ImageID::Person(id) => tmdb::movie::get_person_artwork(
                                    &cache_dir,
                                    tmdb_access_token.as_str(),
                                    id,
                                ),
                                ImageID::Custom(path, backdrop) => tmdb::movie::get_custom_artwork(
                                    &cache_dir,
                                    tmdb_access_token.as_str(),
                                    None,
                                    path,
                                    *backdrop,
                                ),
                                _ => Ok(false),
                            };

                            tx_main.send((image_id, result.map(|x| Either::Right(x))))
                        });
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

        (rx_main, tx_load)
    }

    fn path_from_image_id(image_id: &ImageID, cache_dir: &Path) -> PathBuf {
        match image_id {
            &ImageID::Movie(id, _, backdrop) => if backdrop {
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
            ImageID::Custom(path, _) => cache_dir
                .join("custom")
                .join(path.strip_prefix('/').unwrap_or(path))
                .with_extension("jpg"),
        }
    }

    pub fn delete_image_file(&mut self, image_id: ImageID) {
        let path = Self::path_from_image_id(&image_id, &self.cache_dir);
        if let Err(error) = fs::remove_file(path) {
            error!("Error while trying to delete {image_id:?}: {error:#?}");
        }

        self.download_errors.remove(&image_id);
        self.loading_ids.remove(&image_id);
        self.hashed_images.remove(&image_id);
    }

    pub fn hash_image(&mut self, image_id: ImageID) {
        if Self::path_from_image_id(&image_id, &self.cache_dir).is_file() {
            self.load_queue.push_back(image_id);
        } else {
            if !self.downloading_ids.contains(&image_id)
                && !self.download_errors.contains(&image_id)
            {
                self.downloading_ids.insert(image_id.clone());
                _ = self.tx_load.send(Actions::DownLoad(image_id));
            }
        }
    }

    pub fn update(&mut self) {
        self.draw_queue.clear();
        self.overlay_areas.clear();
        self.images_drawn = true;

        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        for (image_id, result) in self.rx_main.try_iter() {
            match result {
                Ok(protocol) => match protocol {
                    Either::Left(protocol) => {
                        self.loading_ids.remove(&image_id);
                        _ = self.hashed_images.insert(image_id, protocol)
                    }
                    Either::Right(downloaded_successfully) =>
                        if downloaded_successfully {
                            self.downloading_ids.remove(&image_id);
                            self.load_queue.push_back(image_id);
                        } else {
                            error!("Unable to download {image_id:?}");

                            if !self.download_errors.contains(&image_id) {
                                info!("retrying...");
                                self.download_errors.insert(image_id.clone());

                                let tx_load_cloned = self.tx_load.clone();
                                thread::spawn(move || {
                                    thread::sleep(Duration::from_secs(2));
                                    _ = tx_load_cloned.send(Actions::DownLoad(image_id));
                                });
                            } else {
                                self.downloading_ids.remove(&image_id);
                            }
                        },
                },
                Err(error) => {
                    error!("error loading image {image_id:?}: {error:#?}");

                    self.loading_ids.remove(&image_id);
                }
            }
        }

        while (MAX_CONCURRENT_LOADS - self.loading_ids.len()).min(self.load_queue.len()) > 0 {
            let image_id = self.load_queue.pop_front().unwrap();
            if self.tx_load.send(Actions::Load(image_id.clone())).is_ok() {
                self.loading_ids.insert(image_id);
            }
        }
        self.load_queue.clear();
    }

    pub fn draw_image(
        &mut self,
        image_id: ImageID,
        unobstructed: bool,
        sliced_pos: Option<SignedPosition>,
        buffer: &mut Buffer,
    ) {
        let buffer_area = *buffer.area();
        let size_index = matches!(
            image_id,
            ImageID::Movie(_, _, true) | ImageID::Collection(_, true) | ImageID::Custom(_, true)
        ) as usize;

        if sliced_pos.is_none() {
            let size = self.sizes.get_mut(&image_id.discriminant()).unwrap();
            if size[size_index] != buffer_area.as_size() {
                size[size_index] = buffer_area.as_size();
                _ = self
                    .tx_load
                    .send(Actions::Resize(image_id.discriminant(), *size));

                self.hashed_images.retain(|k, _| {
                    ImageIDDiscriminants::from(k) != image_id.discriminant()
                        || match k {
                            ImageID::Movie(_, _, backdrop)
                            | ImageID::Collection(_, backdrop)
                            | ImageID::Custom(_, backdrop) => *backdrop as usize != size_index,
                            ImageID::Person(_) => false,
                        }
                });
            }
        }

        if let Some(protocol) = self.hashed_images.get(&image_id) {
            if !unobstructed && CALCULATE_OBSTRUCTION {
                self.draw_queue.push((image_id, buffer_area, sliced_pos));
            } else {
                let Size { width, height } = protocol.size();

                let centered_area =
                    buffer_area.centered(constraint!(== width), constraint!(== height));
                SlicedImage::new(
                    protocol,
                    sliced_pos.unwrap_or(SignedPosition { x: 0, y: 0 }),
                )
                .render(centered_area, buffer);
            }
        } else {
            if !self.loading_ids.contains(&image_id) && !self.downloading_ids.contains(&image_id) {
                self.hash_image(image_id.clone());
            }

            Fill::new(" ")
                .bg(tailwind::GRAY.c950)
                .render(buffer_area, buffer);

            if !self.download_errors.contains(&image_id) || self.downloading_ids.contains(&image_id)
            {
                StatefulWidget::render(
                    Throbber::default()
                        .throbber_set(BRAILLE_SIX_DOUBLE)
                        .style(Style::new().fg(tailwind::CYAN.c600).bold()),
                    buffer_area.centered(constraint!(==1), constraint!(==1)),
                    buffer,
                    &mut self.throbber_state,
                );
            }

            self.images_drawn = false;
        }
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
                if let Some(protocol) = self.hashed_images.get(image_id) {
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
        } else {
            let (obsructed, unobstructed): (
                Vec<&(ImageID, Rect, Option<SignedPosition>)>,
                Vec<&(ImageID, Rect, Option<SignedPosition>)>,
            ) = self
                .draw_queue
                .iter()
                .partition(|x| self.overlay_areas.iter().any(|y| y.intersects(x.1)));

            for (image_id, area, sliced_pos) in unobstructed {
                if let Some(protocol) = self.hashed_images.get(image_id) {
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

            for (image_id, big_area, sliced_pos) in obsructed {
                let obstructions = self
                    .overlay_areas
                    .iter()
                    .filter(|x| x.intersects(*big_area));
                let mut areas = vec![*big_area];
                for obstruction in obstructions {
                    let mut new_areas = vec![];
                    for area in areas {
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

                if let Some(protocol) = self.hashed_images.get(image_id) {
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

    pub fn update_access_token(&self, access_token: &str) {
        _ = self
            .tx_load
            .send(Actions::UpdateTokens(access_token.to_string()))
    }
}
