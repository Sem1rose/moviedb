use crate::{
    app::{App, Rating},
    draw::{CurrentScreen, Drawer},
    helpers::ellipsize_string,
};
use log::error;
use rand::prelude::*;
use ratatui::style::{Color, Style, Stylize};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use ratatui_image::{
    errors::Errors,
    picker::Picker,
    protocol::StatefulProtocol,
    thread::{ResizeRequest, ResizeResponse, ThreadImage, ThreadProtocol},
};
use ratatui_macros::{horizontal, line, text, vertical};
use std::{
    sync::mpsc::{self, Sender},
    thread,
};
use style::palette::tailwind;

pub enum ImageEvents {
    DrawImage(usize, Result<ResizeResponse, Errors>),
    LoadImage(u64, Result<StatefulProtocol, Errors>),
}

pub struct MainScreen {
    pub num_visible_movies: usize,
    pub scroll_pos: usize,
    pub selected: usize,

    pub rng: ThreadRng,

    //                                   movie_id  fanart   cache
    // pub loaded_images_cache: HashMap<(usize, bool), StatefulProtocol>,
    pub images: Vec<(usize, ThreadProtocol)>,
    pub image_drawn: Vec<bool>,
    pub tickets: Vec<Option<u64>>,
    pub tickets_age: Vec<usize>,

    pub rx_main: mpsc::Receiver<ImageEvents>,
    pub tx_worker_collector: mpsc::Sender<mpsc::Receiver<ResizeRequest>>,
    pub tx_load_decode: mpsc::Sender<(u64, String)>,
}

impl Default for MainScreen {
    fn default() -> Self {
        let (tx_main, rx_main) = mpsc::channel();

        let (tx_load_decode, tx_worker_collector) = MainScreen::start_workers(tx_main);

        let (tx_fanart_worker, rx_fanart_worker) = mpsc::channel::<ResizeRequest>();
        let _ = tx_worker_collector.send(rx_fanart_worker);

        Self {
            scroll_pos: 0,
            selected: 0,
            num_visible_movies: 0,

            rng: rand::rng(),

            // first element is reserved for the fanart image
            images: vec![(0, ThreadProtocol::new(tx_fanart_worker, None))],
            image_drawn: vec![false],
            tickets: vec![None],
            tickets_age: vec![0],

            rx_main,
            tx_load_decode,
            tx_worker_collector,
        }
    }
}

impl MainScreen {
    fn start_workers(
        tx_main: Sender<ImageEvents>,
    ) -> (Sender<(u64, String)>, Sender<mpsc::Receiver<ResizeRequest>>) {
        let (tx_load_decode, rx_load_decode) = mpsc::channel::<(u64, String)>();
        let (tx_worker_collector, rx_worker_collector) = mpsc::channel();

        let tx_main_sender = tx_main.clone();
        let picker =
            Picker::from_query_stdio().expect("error querying graphics capabilities: {error}");
        thread::spawn(move || {
            for (ticket, path) in rx_load_decode.iter() {
                let tx_main = tx_main_sender.clone();
                thread::spawn(move || {
                    let open_result = image::ImageReader::open(path);

                    if let Err(error) = open_result {
                        let _ =
                            tx_main.send(ImageEvents::LoadImage(ticket, Err(Errors::Io(error))));
                    } else if let Ok(reader) = open_result {
                        let decode_result = reader.decode();

                        if let Err(error) = decode_result {
                            let _ = tx_main
                                .send(ImageEvents::LoadImage(ticket, Err(Errors::Image(error))));
                        } else if let Ok(decoded) = decode_result {
                            let _ = tx_main.send(ImageEvents::LoadImage(
                                ticket,
                                Ok(picker.new_resize_protocol(decoded)),
                            ));
                        }
                    }
                });
            }
        });

        let tx_main_sender = tx_main.clone();
        let mut rx_workers: Vec<std::sync::mpsc::Receiver<ResizeRequest>> = vec![]; // index 0 is always the fanart image
        thread::spawn(move || loop {
            for rx_worker in rx_worker_collector.try_iter() {
                rx_workers.push(rx_worker);
            }

            let mut dropped = vec![];
            for (id, rx_worker) in rx_workers.iter_mut().enumerate() {
                let message = rx_worker.try_recv();

                if let Ok(request) = message {
                    tx_main_sender
                        .send(ImageEvents::DrawImage(id, request.resize_encode()))
                        .unwrap();
                } else if let Err(error) = message {
                    if error == std::sync::mpsc::TryRecvError::Disconnected {
                        dropped.push(id);
                    }
                }
            }

            for x in dropped {
                if rx_workers.len() > x {
                    rx_workers.remove(x);
                }
            }
        });

        (tx_load_decode, tx_worker_collector)
    }

    pub fn current_movie_index(&self) -> usize {
        self.scroll_pos + self.selected
    }

    pub fn read_channels(&mut self) {
        for image_event in self.rx_main.try_iter() {
            match image_event {
                ImageEvents::LoadImage(ticket, result) => {
                    if let Ok(protocol) = result {
                        let item = self
                            .tickets
                            .iter()
                            .position(|&x| x.is_some() && x.unwrap() == ticket);

                        if item.is_some() {
                            self.images[item.unwrap()].1.replace_protocol(protocol);
                            self.image_drawn[item.unwrap()] = true;
                            self.tickets[item.unwrap()] = None;
                        }
                    } else if let Err(error) = result {
                        error!("Error while loading: {}", error);
                    }
                }
                ImageEvents::DrawImage(id, result) => {
                    if let Ok(response) = result {
                        let item = self.images.iter().position(|&(x, _)| x == id);

                        if let Some(i) = item {
                            self.images[i].1.update_resized_protocol(response);
                        }
                    } else if let Err(error) = result {
                        println!("Error while drawing: {}", error);
                    }
                }
            }
        }
    }

    fn push_poster_image(&mut self) {
        let (tx_worker, rx_worker) = mpsc::channel::<ResizeRequest>();

        let new_poster_image = ThreadProtocol::new(tx_worker, None);
        let _ = self.tx_worker_collector.send(rx_worker);

        self.images.push((self.images.len(), new_poster_image));
        self.image_drawn.push(false);
        self.tickets.push(None);
        self.tickets_age.push(0);
    }

    fn pop_poster_image(&mut self) {
        let item = self
            .images
            .iter()
            .position(|&(i, _)| i == self.images.len() - 1);

        if let Some(id) = item {
            self.images.remove(id);
            self.image_drawn.remove(id);
            self.tickets.remove(id);
            self.tickets_age.remove(id);
        } else {
            let _ = self.images.pop();
            let _ = self.image_drawn.pop();
            let _ = self.tickets.pop();
            let _ = self.tickets_age.pop();
        }
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: usize) {
        if self.num_visible_movies == 0 {
            self.num_visible_movies = num_movies_visible;

            for _ in 0..num_movies_visible {
                self.push_poster_image();
            }
        } else if self.num_visible_movies != num_movies_visible {
            if self.num_visible_movies > num_movies_visible {
                for _ in 0..(self.num_visible_movies - num_movies_visible) {
                    self.pop_poster_image();
                }
            } else {
                for _ in 0..(num_movies_visible - self.num_visible_movies) {
                    self.push_poster_image();
                }
            }

            // don't know why i did all of this
            self.num_visible_movies = num_movies_visible;
            if self.selected >= num_movies_visible {
                self.selected = num_movies_visible - 1;
            }

            self.scroll_pos = self.current_movie_index() - self.selected;

            self.clear_all_image();
            self.clear_tickets();
        }
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) -> bool {
        if num_movies == 0 {
            return false;
        }

        if self.current_movie_index() < num_movies - 1 {
            self.clear_image(0, true);

            if self.selected < self.num_visible_movies - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;

                if self.images.len() > 1 {
                    self.images[1..].rotate_left(1);
                    self.image_drawn[1..].rotate_left(1);
                    self.tickets[1..].rotate_left(1);
                    self.tickets_age[1..].rotate_left(1);

                    self.clear_image(self.num_visible_movies - 1, false);
                } else {
                    self.clear_all_image();
                }
            }

            return true;
        }

        false
    }

    pub fn dec_movie_selection(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;

            self.clear_image(0, true);

            return true;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;

            if self.images.len() > 1 {
                self.images[1..].rotate_right(1);
                self.image_drawn[1..].rotate_right(1);
                self.tickets[1..].rotate_right(1);
                self.tickets_age[1..].rotate_right(1);

                self.clear_image(0, false);
                self.clear_image(0, true);
            } else {
                self.clear_all_image();
            }

            return true;
        }

        false
    }

    pub fn clear_tickets(&mut self) {
        for i in 0..self.tickets.len() {
            self.tickets[i] = None;
        }
    }

    pub fn inc_tickets_age(&mut self) {
        // for i in 0..self.tickets_age.len() {
        //     if self.tickets[i].is_some() {
        //         self.tickets_age[i] += 1;
        //     } else {
        //         self.tickets_age[i] = 0;
        //     }

        //     if self.tickets_age[i] > 20 {
        //         self.tickets_age[i] = 0;
        //         self.clear_image(i, false);
        //     }
        // }
    }

    pub fn redraw_all_image(&mut self, app: &App) {
        for i in 0..self.num_visible_movies {
            self.draw_image(app, i, false);

            if i == self.selected {
                self.draw_image(app, 0, true);
            }
        }
    }

    // INPUT => image_id -> 0..num_visible_movies if fanart else doesn't matter will be set to 0
    pub fn draw_image(&mut self, app: &App, image_id: usize, fanart: bool) {
        let index = if fanart { self.selected } else { image_id };

        let ticket_id = self.get_image_index(index, fanart);

        if self.tickets[ticket_id].is_some() {
            return;
        }

        let path = format!(
            "{}",
            if fanart {
                &app.config.dirs.backdrop_cache
            } else {
                &app.config.dirs.poster_cache
            }
            .join(format!(
                "{}.jpg",
                app.movies[self.scroll_pos + index].tmdb_id
            ))
            .display()
        );

        let ticket = self.create_ticket(ticket_id, fanart);

        let result = self.tx_load_decode.send((ticket, path));

        if result.is_ok() {
            self.tickets[ticket_id] = Some(ticket);
        }
    }

    pub fn clear_all_image(&mut self) {
        for i in 0..self.num_visible_movies {
            self.clear_image(i, false);
        }
        self.clear_image(0, true);
    }

    pub fn clear_image(&mut self, image_id: usize, fanart: bool) {
        let id = self.get_image_index(image_id, fanart);
        self.images[id].1.empty_protocol();
        self.image_drawn[id] = false;
        self.tickets[id] = None;
        self.tickets_age[id] = 0;
    }

    // i is from 0..num_movies_visible, we add one because the indices always start at 1 because 0 is reserved for the fanart image.
    fn get_image_index(&self, i: usize, fanart: bool) -> usize {
        if fanart {
            0
        } else {
            i + 1
        }
    }

    fn decode_ticket_id(&self, id: usize) -> (usize, bool) {
        if id == 0 {
            (0, true)
        } else {
            (id - 1, false)
        }
    }

    fn create_ticket(&mut self, id: usize, fanart: bool) -> u64 {
        self.rng.next_u64()
    }
}

impl Drawer {
    pub fn open_main_screen(&mut self) {
        self.close_popups();
        self.current_screen = CurrentScreen::MainScreen;
    }

    pub fn render_movies_list(&mut self, frame: &mut Frame, app: &mut App) -> Result<(), Errors> {
        self.main_screen_options.read_channels();

        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        let movies_lay = Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies])
            .split(horiz_lay[1]);

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

        let index = self.main_screen_options.get_image_index(id, false);
        if !self.main_screen_options.image_drawn[index] {
            self.main_screen_options.draw_image(app, id, false);
        }

        frame.render_stateful_widget(
            ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
                ratatui_image::FilterType::Triangle,
            ))),
            poster_area,
            &mut self.main_screen_options.images[index].1,
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

        if !self.main_screen_options.image_drawn[0] {
            self.main_screen_options.draw_image(app, 0, true);
        }

        frame.render_stateful_widget(
            ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
                ratatui_image::FilterType::Triangle,
            ))),
            poster_area,
            &mut self.main_screen_options.images[0].1,
        );

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
    }
}
