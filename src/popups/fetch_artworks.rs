use crate::{
    helpers::{add_padding, dynamic_popup}, key_event_handler::KeyEventHandler, trakt, tmdb, types::{Movie, MovieID}
};
use ratatui::{
    Frame, layout::*, macros::{horizontal, vertical}, prelude::*, style::palette::{material, tailwind}, text::ToSpan, widgets::*
};
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    path::PathBuf,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

#[derive(Default)]
pub struct FetchArtworksPopup {
    pub done: bool,
    pub started: bool,
    pub progress: usize,
    errored: Option<u32>,
    movies: Vec<MovieID>,
    trakt_client_id: String,
    tmdb_access_token: String,

    tx_fetch_request: Option<Sender<Option<MovieID>>>,
    rx_fetch_response: Option<Receiver<(MovieID, anyhow::Result<()>)>>,

    tick: u64,
    cache_dir: PathBuf,
    throbber_state: ThrobberState,
}

impl FetchArtworksPopup {
    pub fn new(cache_dir: &PathBuf) -> Self {
        Self {
            cache_dir: cache_dir.clone(),
            ..Default::default()
        }
    }

    pub fn update_next_frame(&self) -> bool {
        self.started
    }

    fn start_thread(&mut self) {
        let (tx_fetch_request, rx_fetch_request) = channel::<Option<MovieID>>();
        let (tx_fetch_response, rx_fetch_response) = channel::<(MovieID, anyhow::Result<()>)>();
        let cache_dir = self.cache_dir.clone();
        let trakt_client_id = self.trakt_client_id.clone();
        let tmdb_access_token = self.tmdb_access_token.clone();

        thread::spawn(move || {
            for fetch_request in rx_fetch_request.iter() {
                if fetch_request.is_none() {
                    break;
                }

                let request = fetch_request.unwrap();
                let tx_response = tx_fetch_response.clone();

                let cache_dir = cache_dir.clone();
                let trakt_client_id = trakt_client_id.clone();
                let tmdb_access_token = tmdb_access_token.clone();
                thread::spawn(move || {
                    let result = if !trakt_client_id.is_empty() {
                        trakt::get_movie_poster_banner(
                            &cache_dir,
                            &trakt_client_id,
                            &request.imdb.clone(),
                        )
                    } else {
                        tmdb::get_movie_poster_banner(
                            &cache_dir,
                            &tmdb_access_token,
                            request.tmdb,
                        )
                    };

                    _ = if result.is_ok() {
                        tx_response.send((request, result))
                    } else if !trakt_client_id.is_empty() {
                        let result = tmdb::get_movie_poster_banner(
                            &cache_dir,
                            &tmdb_access_token,
                            request.tmdb,
                        );

                        tx_response.send((request, result))
                    } else {
                        tx_response.send((request, result))
                    };
                });
            }
        });

        self.tx_fetch_request = Some(tx_fetch_request);
        self.rx_fetch_response = Some(rx_fetch_response);
    }

    fn check_artwork_fetched(&self, id: u32) -> bool {
        self.cache_dir
            .join("posters")
            .join(format!("{}.jpg", id))
            .is_file()
        && self.cache_dir
            .join("backdrops")
            .join(format!("{}.jpg", id))
            .is_file()
    }

    fn fetch_artworks(&mut self) {
        self.start_thread();

        for movie_id in &self.movies {
            if !self.check_artwork_fetched(movie_id.tmdb) {
                _ = self
                    .tx_fetch_request
                    .as_ref()
                    .unwrap()
                    .send(Some(movie_id.clone()));
            } else {
                self.progress += 1;
            }
        }
    }

    pub fn set_movies(&mut self, movies: &[Movie], trakt_client_id: &str, tmdb_access_token: &str) {
        self.trakt_client_id = trakt_client_id.to_string();
        self.tmdb_access_token = tmdb_access_token.to_string();
        self.movies = movies.iter().map(|x| x.id.clone()).collect();

        self.done = false;
        self.started = true;
        self.fetch_artworks();
    }

    pub fn update (&mut self) {
        if !self.started {
            return;
        }

        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        for (id, fetch_result) in self.rx_fetch_response.as_ref().unwrap().try_iter() {
            if fetch_result.is_err() {
                self.errored = Some(id.tmdb);
                _ = self.tx_fetch_request.as_ref().unwrap().send(Some(id));
            } else {
                if let Some(i) = self.errored {
                    if i == id.tmdb {
                        self.errored = None;
                    }
                }
                self.progress += 1;
            }
        }

        if self.progress == self.movies.len() && !self.done {
            self.done = true;
            self.started = false;

            _ = self
                .tx_fetch_request
                .as_ref()
                .unwrap()
                .send(None);
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();

        let progress = self.progress;
        let num_movies = self.movies.len();

        let popup_area = dynamic_popup(
            frame,
            Some(8),
            5.0,
            tailwind::BLUE.c950,
            "  Fetching posters  ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
        );

        let [_, throbber_area, _, progress_area, _, errored_area] = vertical![==1, ==1, ==1, ==3, >=1, ==1].areas(popup_area);

        let [throbber_area] = horizontal![==1]
            .flex(Flex::Center)
            .areas(throbber_area);

        let throbber = Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));
        frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);

        let progress_area = add_padding(progress_area, Padding::horizontal(2));

        let progress_gauge = Gauge::default()
            .ratio(if num_movies == 0 { 0.0 } else { progress as f64 / num_movies as f64 })
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(format!("{}/{}", progress, num_movies).fg(tailwind::PINK.c500).bold())
            .use_unicode(true);

        frame.render_widget(progress_gauge, progress_area);

        if let Some(id) = self.errored {
            let errored_text = format!("movie {id} errored");

            let [text_lay] = horizontal![==(errored_text.len() as u16)]
                .flex(Flex::Center)
                .areas(errored_area);

            frame.render_widget(errored_text.to_span().fg(tailwind::RED.c500).bold(), text_lay);
        }
    }
}
