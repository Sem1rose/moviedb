use crate::{app::App, config::Config, image_backends::ImageBackend, types::*};
use ratatui::{prelude::Rect, Frame};
use ratatui_image::thread::ThreadImage;
use ratatui_image::{
    picker::Picker,
    protocol::StatefulProtocol,
    thread::{ResizeRequest, ResizeResponse, ThreadProtocol},
};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct ArtworkID {
    pub tmdb_id: u32,
    pub backdrop: bool,
}

pub enum ImageEvents {
    DrawImage(ArtworkID, Result<ResizeResponse>),
    LoadImage(ArtworkID, Result<StatefulProtocol>),
}

pub struct RatatuiImage {
    pub hashed_images: HashMap<ArtworkID, ThreadProtocol>,

    pub rx_main: Receiver<ImageEvents>,
    pub tx_worker_collector: Sender<(ArtworkID, Receiver<ResizeRequest>)>,
    pub tx_load_decode: Sender<(ArtworkID, String)>,
}

impl RatatuiImage {
    fn start_workers_thread(
        tx_main: &Sender<ImageEvents>,
    ) -> Sender<(ArtworkID, Receiver<ResizeRequest>)> {
        let (tx_worker_collector, rx_worker_collector) = mpsc::channel();

        let tx_main_sender = tx_main.clone();
        let mut rx_workers: Vec<(ArtworkID, Receiver<ResizeRequest>)> = vec![]; // index 0 is always the fanart image
        thread::spawn(move || loop {
            for rx_worker in rx_worker_collector.try_iter() {
                rx_workers.push(rx_worker);
            }

            let mut dropped = vec![];
            for (i, (tmdb_id, rx_worker)) in rx_workers.iter_mut().enumerate() {
                let message = rx_worker.try_recv();

                if let Ok(request) = message {
                    let _ = tx_main_sender.send(ImageEvents::DrawImage(
                        *tmdb_id,
                        request
                            .resize_encode()
                            .map_or_else(|error| Err(error.into()), Result::Ok),
                    ));
                } else if let Err(mpsc::TryRecvError::Disconnected) = message {
                    dropped.push(i);
                }
            }

            for x in dropped {
                if rx_workers.len() > x {
                    rx_workers.remove(x);
                }
            }
        });

        tx_worker_collector
    }

    fn start_load_thread(tx_main: &Sender<ImageEvents>) -> Sender<(ArtworkID, String)> {
        let (tx_load_decode, rx_load_decode) = mpsc::channel::<(ArtworkID, String)>();

        let tx_main_sender = tx_main.clone();
        let picker =
            Picker::from_query_stdio().expect("error querying graphics capabilities: {error}");
        thread::spawn(move || {
            // let poster_pool = ThreadPool::with_name("poster load decode".into(), 16);
            // let fanart_pool = ThreadPool::with_name("fanart load decode".into(), 16);

            for (tmdb_id, path) in rx_load_decode.iter() {
                let tx_main = tx_main_sender.clone();

                // if movie_id.backdrop {
                thread::spawn(move || {
                    let open_result = image::ImageReader::open(path);

                    if let Err(error) = open_result {
                        let _ =
                            tx_main.send(ImageEvents::LoadImage(tmdb_id, Err(Errors::Io(error))));
                        // std::fs::File::create(format!("/home/semirose/{}backdrop", movie_id.0))
                        //     .unwrap()
                        //     .write_all(path.as_bytes());
                        // std::process::exit(1);
                    } else if let Ok(reader) = open_result {
                        let decode_result = reader.decode();

                        if let Err(error) = decode_result {
                            let _ = tx_main
                                .send(ImageEvents::LoadImage(tmdb_id, Err(Errors::Image(error))));
                        } else if let Ok(decoded) = decode_result {
                            let _ = tx_main.send(ImageEvents::LoadImage(
                                tmdb_id,
                                Ok(picker.new_resize_protocol(decoded)),
                            ));
                        }
                    }
                });
                // } else {
                //     thread::spawn(move || {
                //         let open_result = image::ImageReader::open(path);

                //         if let Err(error) = open_result {
                //             let _ = tx_main.send(ImageEvents::LoadImage(
                //                 movie_id,
                //                 // Err(Errors::Other(error.to_string() + &path)),
                //                 Err(Errors::Io(error)),
                //             ));
                //             // std::fs::File::create(format!("/home/semirose/{}poster", movie_id.0))
                //             //     .unwrap()
                //             //     .write_all(path.as_bytes());
                //             // std::process::exit(1);
                //         } else if let Ok(reader) = open_result {
                //             let decode_result = reader.decode();

                //             if let Err(error) = decode_result {
                //                 let _ = tx_main.send(ImageEvents::LoadImage(
                //                     movie_id,
                //                     Err(Errors::Image(error)),
                //                 ));
                //             } else if let Ok(decoded) = decode_result {
                //                 let _ = tx_main.send(ImageEvents::LoadImage(
                //                     movie_id,
                //                     Ok(picker.new_resize_protocol(decoded)),
                //                 ));
                //             }
                //         }
                //     });
                // }
            }
        });

        tx_load_decode
    }

    fn hash_image(&mut self, artwork_id: ArtworkID, config: &Config) {
        let (tx_worker, rx_worker) = mpsc::channel();

        let new_protocol = ThreadProtocol::new(tx_worker, None);
        self.hashed_images.insert(artwork_id, new_protocol);
        let _ = self.tx_worker_collector.send((artwork_id, rx_worker));

        let path = format!(
            "{}",
            if artwork_id.backdrop {
                &config.dirs.backdrop_cache
            } else {
                &config.dirs.poster_cache
            }
            .join(format!("{}.jpg", artwork_id.tmdb_id))
            .display()
        );

        let _ = self.tx_load_decode.send((artwork_id, path));
    }

    // pub fn rehash_visible_images(&mut self, app: &App) {
    //     let start_index = self.movies_list.scroll_pos;
    //     let movie_ids = app.movies
    //         [start_index..(start_index + self.movies_list.num_visible_movies)]
    //         .iter()
    //         .map(|x| x.id.tmdb)
    //         .collect::<Vec<_>>();

    //     for (i, id) in movie_ids.iter().enumerate() {
    //         let index = start_index + i;

    //         let key = ArtworkID {
    //             id: index,
    //             backdrop: false,
    //         };
    //         if !self.hashed_images.contains_key(&key) {
    //             self.hash_image(key, app);
    //         } else {
    //             let poster_path = format!(
    //                 "{}",
    //                 &app.config
    //                     .dirs
    //                     .poster_cache
    //                     .join(format!("{}.jpg", id))
    //                     .display()
    //             );

    //             let _ = self.tx_load_decode.send((key, poster_path));
    //         }

    //         if index == self.movies_list.current_movie_index() {
    //             let key = ArtworkID {
    //                 id: index,
    //                 backdrop: true,
    //             };
    //             if !self.hashed_images.contains_key(&key) {
    //                 self.hash_image(key, app);
    //             } else {
    //                 let fanart_path = format!(
    //                     "{}",
    //                     &app.config
    //                         .dirs
    //                         .backdrop_cache
    //                         .join(format!("{}.jpg", id))
    //                         .display()
    //                 );

    //                 let _ = self.tx_load_decode.send((key, fanart_path));
    //             }
    //         }
    //     }
    // }

    // pub fn rehash_image(&mut self, movie_index: usize, backdrop: bool, app: &App) {
    //     let path = format!(
    //         "{}",
    //         if backdrop {
    //             &app.config.dirs.backdrop_cache
    //         } else {
    //             &app.config.dirs.poster_cache
    //         }
    //         .join(format!("{}.jpg", app.movies[movie_index].id.tmdb))
    //         .display()
    //     );

    //     let _ = self.tx_load_decode.send((
    //         ArtworkID {
    //             id: movie_index,
    //             backdrop,
    //         },
    //         path.clone(),
    //     ));
    // }
}

impl ImageBackend for RatatuiImage {
    fn new() -> Self {
        let (tx_main, rx_main) = mpsc::channel();

        let tx_load_decode = Self::start_load_thread(&tx_main);
        let tx_worker_collector = Self::start_workers_thread(&tx_main);

        Self {
            hashed_images: HashMap::new(),
            rx_main,
            tx_worker_collector,
            tx_load_decode,
        }
    }

    fn update(&mut self) {
        let mut errored_ids = vec![];
        for image_event in self.rx_main.try_iter() {
            match image_event {
                ImageEvents::LoadImage(key, result) => {
                    if let Ok(protocol) = result {
                        if self.hashed_images.contains_key(&key) {
                            self.hashed_images
                                .get_mut(&key)
                                .unwrap()
                                .replace_protocol(protocol);
                        }
                    } else if result.is_err() {
                        // } else if let Err(_) = result {
                        // error!("Error while loading: {}", error);
                        errored_ids.push(key.tmdb_id);
                    }
                }
                ImageEvents::DrawImage(key, result) => {
                    if let Ok(response) = result {
                        if self.hashed_images.contains_key(&key) {
                            self.hashed_images
                                .get_mut(&key)
                                .unwrap()
                                .update_resized_protocol(response);
                        }
                    } else if result.is_err() {
                        // } else if let Err(_) = result {
                        // error!("Error while loading: {}", error);
                        errored_ids.push(key.tmdb_id);
                    }
                }
            }
        }
        for id in errored_ids {
            self.remove_cached_image(id);
        }
    }

    fn draw_image(
        &mut self,
        app: &App,
        tmdb_id: u32,
        backdrop: bool,
        area: Rect,
        frame: &mut Frame,
    ) {
        let key = ArtworkID { tmdb_id, backdrop };
        if !self.hashed_images.contains_key(&key) {
            self.hash_image(key, &app.config);
        }
        frame.render_stateful_widget(
            ThreadImage::new().resize(ratatui_image::Resize::Scale(
                Some(ratatui_image::FilterType::Triangle), // None,
            )),
            area,
            self.hashed_images.get_mut(&key).unwrap(),
        );
    }

    fn reload_images(&mut self, app: &App, start_index: usize, count: Option<usize>) {
        let stop_index = count.map(|x| x + start_index).unwrap_or(app.movies.len());
        let movie_ids = app.movies[start_index..stop_index]
            .iter()
            .map(|x| x.id.tmdb)
            .collect::<Vec<_>>();

        for tmdb_id in movie_ids.iter() {
            let key = ArtworkID {
                tmdb_id: *tmdb_id,
                backdrop: false,
            };
            if !self.hashed_images.contains_key(&key) {
                self.hash_image(key, &app.config);
            } else {
                let poster_path = format!(
                    "{}",
                    &app.config
                        .dirs
                        .poster_cache
                        .join(format!("{}.jpg", tmdb_id))
                        .display()
                );

                let _ = self.tx_load_decode.send((key, poster_path));
            }

            let key = ArtworkID {
                tmdb_id: *tmdb_id,
                backdrop: true,
            };
            if !self.hashed_images.contains_key(&key) {
                self.hash_image(key, &app.config);
            } else {
                let fanart_path = format!(
                    "{}",
                    &app.config
                        .dirs
                        .backdrop_cache
                        .join(format!("{}.jpg", tmdb_id))
                        .display()
                );

                let _ = self.tx_load_decode.send((key, fanart_path));
            }
        }
    }

    fn remove_cached_image(&mut self, tmdb_id: u32) {
        self.hashed_images.remove(&ArtworkID {
            tmdb_id,
            backdrop: true,
        });
        self.hashed_images.remove(&ArtworkID {
            tmdb_id,
            backdrop: false,
        });
    }
}
