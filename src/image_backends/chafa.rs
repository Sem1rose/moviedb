use crate::{app::App, config::Config, image_backends::ImageBackend, types::*};
use ratatui::{
    crossterm::ExecutableCommand,
    prelude::{Rect, Size},
    Frame,
};
use std::{
    collections::HashMap,
    io::stdout,
    process::{Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct ArtworkID {
    pub tmdb_id: u32,
    pub backdrop: bool,
}

pub struct Chafa {
    pub hashed_images: HashMap<ArtworkID, Option<String>>,

    pub rx_cached: Receiver<(ArtworkID, Result<String>)>,
    pub tx_cache_request: Sender<(ArtworkID, String, Size)>,

    poster_size: Size,
    backdrop_size: Size,
}

impl Chafa {
    fn hash_image(&mut self, artwork_id: ArtworkID, config: &Config) {
        self.hashed_images.insert(artwork_id, None);

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

        _ = self.tx_cache_request.send((
            artwork_id,
            path,
            if artwork_id.backdrop {
                self.backdrop_size
            } else {
                self.poster_size
            },
        ));
    }
}

impl ImageBackend for Chafa {
    fn new() -> Self {
        let (tx_cache_request, rx_cache_request) = channel::<(ArtworkID, String, Size)>();
        let (tx_cached, rx_cached) = channel();

        thread::spawn(move || loop {
            for (id, path, area) in rx_cache_request.iter() {
                let tx_cached = tx_cached.clone();
                thread::spawn(move || {
                    let cache_result = Command::new("chafa")
                        .args([
                            "--polite",
                            "on",
                            "--relative",
                            "on",
                            "-s",
                            &format!("{}x{}", area.width, area.height),
                            &path,
                        ])
                        .stdout(Stdio::piped())
                        .output();
                    if let Ok(output) = cache_result {
                        let data = String::from_utf8_lossy(&output.stdout).to_string();
                        _ = tx_cached.send((id, Ok(data)));
                    } else if let Err(error) = cache_result {
                        _ = tx_cached.send((id, Err(Errors::Io(error))));
                    }
                });
            }
        });

        Self {
            hashed_images: HashMap::new(),
            rx_cached,
            tx_cache_request,
            poster_size: Size::ZERO,
            backdrop_size: Size::ZERO,
        }
    }

    fn update(&mut self) {
        let mut errored_ids = vec![];
        for (key, result) in self.rx_cached.try_iter() {
            if let Ok(cached) = result {
                self.hashed_images.insert(key, Some(cached));
            } else {
                errored_ids.push(key.tmdb_id);
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
        _frame: &mut Frame,
    ) {
        if backdrop && area.as_size() != self.backdrop_size {
            self.backdrop_size = area.as_size();
            self.reload_images(app, 0, None, Some(true));
            return;
        } else if !backdrop && area.as_size() != self.poster_size {
            self.poster_size = area.as_size();
            self.reload_images(app, 0, None, Some(false));
            return;
        }

        let key = ArtworkID { tmdb_id, backdrop };
        if !self.hashed_images.contains_key(&key) {
            self.hash_image(key, &app.config);
            return;
        }

        if let Some(poster) = self.hashed_images.get(&key).unwrap() {
            let _ = stdout().execute(ratatui::crossterm::cursor::MoveTo(area.x, area.y));
            print!("{}", poster);
        }
    }

    fn reload_images(
        &mut self,
        app: &App,
        start_index: usize,
        count: Option<usize>,
        backdrop: Option<bool>,
    ) {
        let stop_index = count.map(|x| x + start_index).unwrap_or(app.movies.len());
        let movie_ids = app.movies[start_index..stop_index]
            .iter()
            .map(|x| x.id.tmdb)
            .collect::<Vec<_>>();
        let bd = if let Some(true) = backdrop {
            vec![true]
        } else if let Some(false) = backdrop {
            vec![false]
        } else {
            vec![true, false]
        };

        for tmdb_id in movie_ids.iter() {
            for _backdrop in &bd {
                let key = ArtworkID {
                    tmdb_id: *tmdb_id,
                    backdrop: *_backdrop,
                };

                self.hash_image(key, &app.config);
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
