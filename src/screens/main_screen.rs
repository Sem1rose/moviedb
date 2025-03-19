use crate::{
    app::{App, Errors, Rating, Result},
    draw::Drawer,
    helpers::ellipsize_string,
    screens::Screens,
};
use log::error;
use ratatui::style::{Color, Style, Stylize};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_image::{
    picker::Picker,
    protocol::StatefulProtocol,
    thread::{ResizeRequest, ResizeResponse, ThreadImage, ThreadProtocol},
};
use ratatui_macros::{horizontal, line, text, vertical};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use style::palette::tailwind;
use threadpool::ThreadPool;

type MovieID = (usize, bool);

pub enum ImageEvents {
    DrawImage(MovieID, Result<ResizeResponse>),
    LoadImage(MovieID, Result<StatefulProtocol>),
}

pub struct MainScreen {
    pub num_visible_movies: usize,
    pub scroll_pos: usize,
    pub selected: usize,

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
            scroll_pos: 0,
            selected: 0,
            num_visible_movies: 0,

            hashed_images: HashMap::new(),

            rx_main,
            tx_load_decode,
            tx_worker_collector,
            // hasher_pool: ThreadPool::with_name("poster-hashing".into(), 2),
        }
    }
}

impl MainScreen {
    pub fn inc_movie_selection(&mut self, num_movies: usize) -> bool {
        if num_movies == 0 {
            return false;
        }

        if self.current_movie_index() < num_movies - 1 {
            if self.selected < self.num_visible_movies - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }

            return true;
        }

        false
    }

    pub fn dec_movie_selection(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;

            return true;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;

            return true;
        }

        false
    }
}

impl MainScreen {
    pub fn current_movie_index(&self) -> usize {
        self.scroll_pos + self.selected
    }

    fn start_workers(
        tx_main: Sender<ImageEvents>,
    ) -> (
        Sender<(MovieID, String)>,
        Sender<(MovieID, Receiver<ResizeRequest>)>,
    ) {
        let (tx_load_decode, rx_load_decode) = mpsc::channel::<_>();
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
            let pool = ThreadPool::with_name("image load decode".into(), 16);

            for (movie_id, path) in rx_load_decode.iter() {
                let tx_main = tx_main_sender.clone();

                pool.execute(move || {
                    let open_result = image::ImageReader::open(path);

                    if let Err(error) = open_result {
                        let _ =
                            tx_main.send(ImageEvents::LoadImage(movie_id, Err(Errors::Io(error))));
                    } else if let Ok(reader) = open_result {
                        let decode_result = reader.decode();

                        if let Err(error) = decode_result {
                            let _ = tx_main
                                .send(ImageEvents::LoadImage(movie_id, Err(Errors::Image(error))));
                        } else if let Ok(decoded) = decode_result {
                            let _ = tx_main.send(ImageEvents::LoadImage(
                                movie_id,
                                Ok(picker.new_resize_protocol(decoded)),
                            ));
                        }
                    }
                });
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

    pub fn set_num_movies_visible(&mut self, num_movies_visible: usize) {
        if self.num_visible_movies == 0 {
            self.num_visible_movies = num_movies_visible;
        } else if self.num_visible_movies != num_movies_visible {
            self.num_visible_movies = num_movies_visible;

            if self.selected >= num_movies_visible {
                self.selected = num_movies_visible - 1;
            }

            self.scroll_pos = self.current_movie_index() - self.selected;
        }
    }

    pub fn goto_index(&mut self, num_movies: usize, index: usize) -> bool {
        if index >= num_movies {
            return false;
        }

        if self.num_visible_movies >= num_movies {
            self.scroll_pos = 0;
            self.selected = index;
        } else if num_movies - index < self.num_visible_movies {
            self.scroll_pos = num_movies - self.num_visible_movies;
            self.selected = index - self.scroll_pos;
        } else {
            self.selected = index % self.num_visible_movies;
            self.scroll_pos = (index / self.num_visible_movies) * self.num_visible_movies;
        }

        true
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
                app.should_quit = true;
            }
            KeyCode::Char('a') => {
                self.open_add_movie_popup();
            }
            KeyCode::Char('e') => {
                self.open_edit_movie_popup();
            }
            KeyCode::Char('d') => {
                self.open_remove_movie_popup();
            }
            KeyCode::Delete => {
                self.open_remove_movie_popup();
            }
            KeyCode::Char('G') => {
                self.main_screen_options
                    .goto_index(app.movies.len(), app.movies.len() - 1);
            }
            KeyCode::Char('g') => {
                self.main_screen_options.goto_index(app.movies.len(), 0);
            }
            KeyCode::Up => {
                self.main_screen_options.dec_movie_selection();
            }
            KeyCode::Down => {
                self.main_screen_options
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

    pub fn render_movies_list(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        self.main_screen_options.read_channels();

        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        let movies_lay = Layout::vertical(vec![Constraint::Min(8); num_movies]).split(horiz_lay[1]);

        self.main_screen_options.set_num_movies_visible(num_movies);

        for (i, area) in movies_lay.iter().enumerate() {
            if !app.movies.is_empty()
                && (i + self.main_screen_options.scroll_pos) < app.movies.len()
            {
                self.draw_movie_widget(i, app, frame, *area);
            } else {
                frame.render_widget(
                    Block::new().bg(if i % 2 == 0 {
                        tailwind::NEUTRAL.c900
                    } else {
                        tailwind::STONE.c900
                    }),
                    *area,
                );
            }
        }

        if !app.movies.is_empty() {
            self.draw_movie_description(app, frame, horiz_lay[0]);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("🢑"))
                .end_symbol(Some("🢓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▉")
                .thumb_style(Style::new().fg(Color::White))
                .track_style(Style::new().fg(Color::DarkGray).bold())
                .begin_style(Style::new().fg(Color::DarkGray).bold())
                .end_style(Style::new().fg(Color::DarkGray).bold());

            let mut scrollbar_state = ScrollbarState::new(app.movies.len() - num_movies)
                .position(self.main_screen_options.scroll_pos);

            frame.render_stateful_widget(scrollbar, horiz_lay[1], &mut scrollbar_state);
        }

        Ok(())
    }

    fn draw_movie_widget(&mut self, id: usize, app: &mut App, frame: &mut Frame, area: Rect) {
        let selected = self.main_screen_options.selected == id;
        let alt = (self.main_screen_options.scroll_pos + id) % 2 == 0;
        let movie_id = id + self.main_screen_options.scroll_pos;
        let movie = app.movies[movie_id].clone();

        // TODO: create a themes framework, maybe in the config
        let (background, text, border, selection_highlight) = if selected {
            (
                Color::Rgb(16, 48, 16),
                Color::Rgb(48, 144, 48),
                Color::Rgb(64, 192, 64),
                Color::Rgb(32, 96, 32),
            )
        } else if alt {
            (
                Color::Rgb(48, 16, 16),
                Color::Rgb(144, 48, 48),
                Color::Rgb(192, 64, 64),
                Color::Rgb(96, 32, 32),
            )
        } else {
            (
                Color::Rgb(16, 24, 48),
                Color::Rgb(48, 72, 144),
                Color::Rgb(64, 96, 192),
                Color::Rgb(32, 48, 96),
            )
        };

        let vert_lay = vertical![==1, >=0, ==1].split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [highlight_area, poster_area, description_area, _] =
            horizontal![==2, ==movie_width, >=0, ==2].areas(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let name = ellipsize_string(movie.name.as_str(), description_area.width as usize - 11);

        let text = text![
            (name.bold() + " ".into() + movie.year.italic()),
            format!("{:.1}", movie.user_rating),
            "",
            movie.tagline,
        ];

        frame.render_widget(text, description_area);

        if selected {
            frame.render_widget(
                text![line!["▐"]; highlight_area.height as usize].fg(selection_highlight),
                highlight_area,
            );
        }

        let key = (self.main_screen_options.scroll_pos + id, false);
        if !self.main_screen_options.hashed_images.contains_key(&key) {
            self.main_screen_options.hash_image(key.0, key.1, app);
        }

        frame.render_stateful_widget(
            ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
                ratatui_image::FilterType::Triangle,
            ))),
            poster_area,
            self.main_screen_options
                .hashed_images
                .get_mut(&key)
                .unwrap(),
        );
    }

    fn draw_movie_description(&mut self, app: &mut App, frame: &mut Frame, area: Rect) {
        let movie = &app.movies[self.main_screen_options.current_movie_index()];

        let [_, vert, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [_, horiz, _] = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .areas(vert);

        let backdrop_height = ((vert.width - 4) as f32 * 9.0 / 32.0).ceil() as u16;
        let [poster_area, title_area, description_area] = Layout::vertical(vec![
            Constraint::Length(backdrop_height),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .areas(horiz);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        // if !self.main_screen_options.image_drawn[0] {
        //     self.main_screen_options.draw_image(app, 0, true);
        // }

        // frame.render_stateful_widget(
        //     ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
        //         ratatui_image::FilterType::Triangle,
        //     ))),
        //     poster_area,
        //     &mut self.main_screen_options.images[0].1,
        // );

        let subtitle = Line::from_iter([
            "released: ".italic(),
            movie.year.as_str().bold().italic(),
            " ".repeat((title_area.width - 11 - 14).into()).into(),
            "rating: ".italic(),
            if let Rating::TMDB(rating, count) = movie.ratings[1] {
                format!("{:.1}", rating).italic().bold()
            } else if let Rating::Trakt(rating, count) = movie.ratings[1] {
                format!("{:.1}", rating).italic().bold()
            } else {
                "nan".into()
            },
        ]);
        let mut name = movie.name.clone();
        if name.len() > (title_area.width as usize - 5) {
            name.truncate(title_area.width as usize - 8);
            name += "...";
        }

        let lines = vec![
            Line::from(name.as_str().bold()).centered(),
            subtitle,
            Line::from("─".repeat(title_area.width as usize)).dim(),
        ];

        let description = Paragraph::new(movie.overview.as_str()).wrap(Wrap { trim: true });

        frame.render_widget(Text::from(lines), title_area);
        frame.render_widget(description, description_area);

        let key = (self.main_screen_options.current_movie_index(), true);
        if !self.main_screen_options.hashed_images.contains_key(&key) {
            self.main_screen_options.hash_image(key.0, key.1, app);
        }

        frame.render_stateful_widget(
            ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
                ratatui_image::FilterType::Triangle,
            ))),
            poster_area,
            self.main_screen_options
                .hashed_images
                .get_mut(&key)
                .unwrap(),
        );
    }
}
