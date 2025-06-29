mod movie_description;
mod movies_list;
use crate::{
    app::{App, Errors, Result},
    draw::Drawer,
    screens::{
        main_screen::{movie_description::MovieDescription, movies_list::MoviesList},
        Screens,
    },
};
use log::error;
use ratatui::style::Stylize;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_image::{
    picker::Picker,
    protocol::StatefulProtocol,
    thread::{ResizeRequest, ResizeResponse, ThreadProtocol},
};
use ratatui_macros::{horizontal, vertical};
use std::io::prelude::*;
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use style::palette::tailwind;
use threadpool::ThreadPool;

//                    id     backdrop/poster
pub type MovieID = (usize, bool);

pub enum ImageEvents {
    DrawImage(MovieID, Result<ResizeResponse>),
    LoadImage(MovieID, Result<StatefulProtocol>),
}

pub struct MainScreen {
    pub movies_list_options: MoviesList,
    pub movie_description_options: MovieDescription,

    pub hashed_images: HashMap<MovieID, ThreadProtocol>,

    pub rx_main: Receiver<ImageEvents>,
    pub tx_worker_collector: Sender<(MovieID, Receiver<ResizeRequest>)>,
    pub tx_load_decode: Sender<(MovieID, String)>,
    // pub hasher_pool: ThreadPool,
}

impl Default for MainScreen {
    fn default() -> Self {
        let (tx_main, rx_main) = mpsc::channel();

        let (tx_load_decode, tx_worker_collector) = MainScreen::start_workers(tx_main);

        Self {
            movies_list_options: MoviesList::default(),
            movie_description_options: MovieDescription::default(),

            hashed_images: HashMap::new(),

            rx_main,
            tx_load_decode,
            tx_worker_collector,
            // hasher_pool: ThreadPool::with_name("poster-hashing".into(), 2),
        }
    }
}

impl MainScreen {
    fn start_workers(
        tx_main: Sender<ImageEvents>,
    ) -> (
        Sender<(MovieID, String)>,
        Sender<(MovieID, Receiver<ResizeRequest>)>,
    ) {
        let (tx_load_decode, rx_load_decode) = mpsc::channel::<(MovieID, String)>();
        let (tx_worker_collector, rx_worker_collector) = mpsc::channel();

        let tx_main_sender = tx_main.clone();
        let mut rx_workers: Vec<(MovieID, Receiver<ResizeRequest>)> = vec![]; // index 0 is always the fanart image
        thread::spawn(move || loop {
            for rx_worker in rx_worker_collector.try_iter() {
                rx_workers.push(rx_worker);
            }

            let mut dropped = vec![];
            for (i, (movie_index, rx_worker)) in rx_workers.iter_mut().enumerate() {
                let message = rx_worker.try_recv();

                if let Ok(request) = message {
                    let _ = tx_main_sender.send(ImageEvents::DrawImage(
                        *movie_index,
                        request
                            .resize_encode()
                            .map_or_else(|error| Result::Err(error.into()), Result::Ok),
                    ));
                } else if let Err(std::sync::mpsc::TryRecvError::Disconnected) = message {
                    dropped.push(i);
                }
            }

            for x in dropped {
                if rx_workers.len() > x {
                    rx_workers.remove(x);
                }
            }
        });

        let tx_main_sender = tx_main.clone();
        let picker =
            Picker::from_query_stdio().expect("error querying graphics capabilities: {error}");
        thread::spawn(move || {
            let poster_pool = ThreadPool::with_name("poster load decode".into(), 16);
            let fanart_pool = ThreadPool::with_name("fanart load decode".into(), 16);

            for (movie_id, path) in rx_load_decode.iter() {
                let tx_main = tx_main_sender.clone();

                if movie_id.1 {
                    fanart_pool.execute(move || {
                        let open_result = image::ImageReader::open(path.clone());

                        if let Err(error) = open_result {
                            let _ = tx_main
                                .send(ImageEvents::LoadImage(movie_id, Err(Errors::Io(error))));
                            // std::fs::File::create(format!("/home/semirose/{}backdrop", movie_id.0))
                            //     .unwrap()
                            //     .write_all(path.as_bytes());
                            // std::process::exit(1);
                        } else if let Ok(reader) = open_result {
                            let decode_result = reader.decode();

                            if let Err(error) = decode_result {
                                let _ = tx_main.send(ImageEvents::LoadImage(
                                    movie_id,
                                    Err(Errors::Image(error)),
                                ));
                            } else if let Ok(decoded) = decode_result {
                                let _ = tx_main.send(ImageEvents::LoadImage(
                                    movie_id,
                                    Ok(picker.new_resize_protocol(decoded)),
                                ));
                            }
                        }
                    });
                } else {
                    poster_pool.execute(move || {
                        let open_result = image::ImageReader::open(path.clone());

                        if let Err(error) = open_result {
                            let _ = tx_main
                                .send(ImageEvents::LoadImage(movie_id, Err(Errors::Io(error))));
                            // std::fs::File::create(format!("/home/semirose/{}poster", movie_id.0))
                            //     .unwrap()
                            //     .write_all(path.as_bytes());
                            // std::process::exit(1);
                        } else if let Ok(reader) = open_result {
                            let decode_result = reader.decode();

                            if let Err(error) = decode_result {
                                let _ = tx_main.send(ImageEvents::LoadImage(
                                    movie_id,
                                    Err(Errors::Image(error)),
                                ));
                            } else if let Ok(decoded) = decode_result {
                                let _ = tx_main.send(ImageEvents::LoadImage(
                                    movie_id,
                                    Ok(picker.new_resize_protocol(decoded)),
                                ));
                            }
                        }
                    });
                }
            }
        });

        (tx_load_decode, tx_worker_collector)
    }

    fn read_channels(&mut self) {
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
                    } else if let Err(error) = result {
                        error!("Error while loading: {}", error);
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
                    } else if let Err(error) = result {
                        error!("Error while drawing: {}", error);
                    }
                }
            }
        }
    }

    pub fn hash_image(&mut self, movie_index: usize, fanart: bool, app: &App) {
        let (tx_worker, rx_worker) = mpsc::channel::<_>();

        let new_protocol = ThreadProtocol::new(tx_worker, None);
        self.hashed_images
            .insert((movie_index, fanart), new_protocol);
        let _ = self
            .tx_worker_collector
            .send(((movie_index, fanart), rx_worker));

        let path = format!(
            "{}",
            if fanart {
                &app.config.dirs.backdrop_cache
            } else {
                &app.config.dirs.poster_cache
            }
            .join(format!("{}.jpg", app.movies[movie_index].tmdb_id))
            .display()
        );

        let _ = self
            .tx_load_decode
            .send(((movie_index, fanart), path.clone()));
    }

    pub fn rehash_images(&mut self, app: &App, start_index: usize) {
        let movie_ids = app.movies[start_index..]
            .iter()
            .map(|x| x.tmdb_id)
            .collect::<Vec<_>>();

        for (i, id) in movie_ids.iter().enumerate() {
            let index = start_index + i;

            if !self.hashed_images.contains_key(&(index, false)) {
                self.hash_image(index, false, app);
            } else {
                let poster_path = format!(
                    "{}",
                    &app.config
                        .dirs
                        .poster_cache
                        .join(format!("{}.jpg", id))
                        .display()
                );

                let _ = self.tx_load_decode.send(((index, false), poster_path));
            }

            if !self.hashed_images.contains_key(&(index, true)) {
                self.hash_image(index, true, app);
            } else {
                let fanart_path = format!(
                    "{}",
                    &app.config
                        .dirs
                        .backdrop_cache
                        .join(format!("{}.jpg", id))
                        .display()
                );

                let _ = self.tx_load_decode.send(((index, true), fanart_path));
            }
        }
    }

    pub fn rehash_visible_images(&mut self, app: &App) {
        let start_index = self.movies_list_options.scroll_pos;
        let movie_ids = app.movies
            [start_index..(start_index + self.movies_list_options.num_visible_movies)]
            .iter()
            .map(|x| x.tmdb_id)
            .collect::<Vec<_>>();

        for (i, id) in movie_ids.iter().enumerate() {
            let index = start_index + i;

            if !self.hashed_images.contains_key(&(index, false)) {
                self.hash_image(index, false, app);
            } else {
                let poster_path = format!(
                    "{}",
                    &app.config
                        .dirs
                        .poster_cache
                        .join(format!("{}.jpg", id))
                        .display()
                );

                let _ = self.tx_load_decode.send(((index, false), poster_path));
            }

            if index == self.movies_list_options.current_movie_index() {
                if !self.hashed_images.contains_key(&(index, true)) {
                    self.hash_image(index, true, app);
                } else {
                    let fanart_path = format!(
                        "{}",
                        &app.config
                            .dirs
                            .backdrop_cache
                            .join(format!("{}.jpg", id))
                            .display()
                    );

                    let _ = self.tx_load_decode.send(((index, true), fanart_path));
                }
            }
        }
    }

    pub fn rehash_image(&mut self, movie_index: usize, fanart: bool, app: &App) {
        let path = format!(
            "{}",
            if fanart {
                &app.config.dirs.backdrop_cache
            } else {
                &app.config.dirs.poster_cache
            }
            .join(format!("{}.jpg", app.movies[movie_index].tmdb_id))
            .display()
        );

        let _ = self
            .tx_load_decode
            .send(((movie_index, fanart), path.clone()));
    }
}

impl Drawer {
    pub fn main_screen_handle_key_events(&mut self, app: &mut App, event: KeyEvent) -> Result<()> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(());
        }

        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('a') => {
                self.open_add_movie_popup();
            }
            KeyCode::Char('e') => {
                self.open_edit_movie_popup(app);
            }
            KeyCode::Char('d') => {
                self.open_remove_movie_popup();
            }
            KeyCode::Delete => {
                self.open_remove_movie_popup();
            }
            KeyCode::Char('G') => {
                self.main_screen_options
                    .movies_list_options
                    .goto_index(app.movies.len(), app.movies.len() - 1);
            }
            KeyCode::Char('g') => {
                self.main_screen_options
                    .movies_list_options
                    .goto_index(app.movies.len(), 0);
            }
            KeyCode::Up => {
                self.main_screen_options
                    .movies_list_options
                    .dec_movie_selection();
            }
            KeyCode::Down => {
                self.main_screen_options
                    .movies_list_options
                    .inc_movie_selection(app.movies.len());
            }
            KeyCode::Esc => {
                self.close_popups();
            }
            _ => (),
        }

        Ok(())
    }

    pub fn open_main_screen(&mut self) {
        self.close_popups();

        self.current_screen = Screens::MainScreen;
    }

    pub fn render_main_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        self.main_screen_options.read_channels();

        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        self.render_movies_list(frame, app, horiz_lay[1], num_movies)?;

        if !app.movies.is_empty() {
            self.draw_movie_description(app, frame, horiz_lay[0]);
        }

        Ok(())
    }
}
