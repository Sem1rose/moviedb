use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::{Alignment, Flex},
    macros::{horizontal, vertical},
    style::{
        Style, Stylize,
        palette::{material, tailwind},
    },
    text::Text,
    widgets::{Gauge, Padding},
};
use strum::AsRefStr;
use throbber_widgets_tui::{Throbber, ThrobberState};
use tmdb;
use trakt;

use crate::{
    helpers::{add_padding, create_popup, dynamic_area, wrap_text},
    key_event_handler::KeyEventHandler,
    popups::PopupTrait,
    tokens::{tmdb_tokens::TMDBTokens, trakt_tokens::TraktTokens},
    types::{Movie, MovieID},
};

#[derive(Default, AsRefStr)]
#[strum(serialize_all = "title_case")]
pub enum Phase {
    #[default]
    Initializing,
    MovieArtworks,
    PersonArtworks,
    CollectionArtworks,
    Done,
}

pub enum ItemID {
    Movie(MovieID),
    Person(u32),
    Collection(u32),
}

#[derive(Default)]
pub struct FetchArtworksPopup {
    pub phase:    Phase,
    pub progress: usize,
    errored:      Option<(u32, String)>,
    movies:       Vec<MovieID>,
    persons:      Vec<u32>,
    collections:  Vec<u32>,

    trakt_client_id:   Option<String>,
    tmdb_access_token: Option<String>,

    tx_fetch_request:  Option<Sender<ItemID>>,
    rx_fetch_response: Option<Receiver<(ItemID, anyhow::Result<()>)>>,

    tick:           u64,
    cache_dir:      PathBuf,
    throbber_state: ThrobberState,
}

impl FetchArtworksPopup {
    pub fn new(cache_dir: &PathBuf) -> Self {
        Self {
            cache_dir: cache_dir.clone(),
            errored: None,
            ..Default::default()
        }
    }

    fn start_thread(&mut self) {
        let (tx_fetch_request, rx_fetch_request) = channel::<ItemID>();
        let (tx_fetch_response, rx_fetch_response) = channel::<(ItemID, anyhow::Result<()>)>();
        let cache_dir = self.cache_dir.clone();
        let trakt_client_id = self.trakt_client_id.clone();
        let tmdb_access_token = self.tmdb_access_token.clone();

        thread::spawn(move || {
            for request in rx_fetch_request.iter() {
                let tx_response = tx_fetch_response.clone();

                let cache_dir = cache_dir.clone();
                let trakt_client_id = trakt_client_id.clone();
                let tmdb_access_token = tmdb_access_token.clone();
                thread::spawn(move || {
                    let result = if let Some(trakt_client_id) = trakt_client_id.as_ref() {
                        match &request {
                            ItemID::Movie(movie_id) => trakt::movie::get_movie_poster_banner(
                                &cache_dir,
                                trakt_client_id,
                                &movie_id.imdb.clone(),
                            ),
                            ItemID::Person(_) => todo!(),
                            ItemID::Collection(_) => todo!(),
                        }
                    } else if let Some(tmdb_access_token) = tmdb_access_token.as_ref() {
                        match &request {
                            ItemID::Movie(movie_id) => tmdb::movie::get_movie_poster_banner(
                                &cache_dir,
                                tmdb_access_token,
                                movie_id.tmdb,
                            ),
                            ItemID::Person(id) =>
                                tmdb::movie::get_person_artwork(&cache_dir, tmdb_access_token, *id),
                            ItemID::Collection(id) => tmdb::movie::get_collection_artwork(
                                &cache_dir,
                                tmdb_access_token,
                                *id,
                            ),
                        }
                    } else {
                        unreachable!();
                    };

                    tx_response.send((request, result))
                });
            }
        });

        self.tx_fetch_request = Some(tx_fetch_request);
        self.rx_fetch_response = Some(rx_fetch_response);
    }

    pub fn advance_phase(&mut self) {
        self.progress = 0;
        self.errored = None;
        self.phase = match self.phase {
            Phase::Initializing => {
                let check_artwork_fetched = |id: u32| -> bool {
                    self.cache_dir
                        .join("posters")
                        .join(format!("{id}.jpg"))
                        .is_file()
                        && self
                            .cache_dir
                            .join("backdrops")
                            .join(format!("{id}.jpg"))
                            .is_file()
                };

                for movie_id in &self.movies {
                    if !check_artwork_fetched(movie_id.tmdb) {
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Movie(movie_id.clone()));
                    } else {
                        self.progress += 1;
                    }
                }

                Phase::MovieArtworks
            }
            Phase::MovieArtworks => {
                let check_artwork_fetched = |id: &u32| -> bool {
                    self.cache_dir
                        .join("persons")
                        .join(format!("{id}.jpg"))
                        .is_file()
                };

                for id in &self.persons {
                    if !check_artwork_fetched(id) {
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Person(*id));
                    } else {
                        self.progress += 1;
                    }
                }

                Phase::PersonArtworks
            }
            Phase::PersonArtworks => {
                let check_artwork_fetched = |id: &u32| -> bool {
                    self.cache_dir
                        .join("collections")
                        .join(format!("{id}.jpg"))
                        .is_file()
                };

                for id in &self.collections {
                    if !check_artwork_fetched(id) {
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Collection(*id));
                    } else {
                        self.progress += 1;
                    }
                }

                Phase::CollectionArtworks
            }
            Phase::CollectionArtworks => {
                drop(self.tx_fetch_request.take().unwrap());

                Phase::Done
            }
            _ => Phase::Initializing,
        };
    }

    pub fn initialize(
        &mut self,
        movies: &[Movie],
        trakt_tokens: &TraktTokens,
        tmdb_tokens: &TMDBTokens,
    ) {
        self.trakt_client_id = if trakt_tokens.status.is_some() {
            Some(trakt_tokens.client_id_owned())
        } else {
            None
        };
        self.tmdb_access_token = if tmdb_tokens.status.is_some() {
            Some(tmdb_tokens.access_token_owned())
        } else {
            None
        };
        self.movies = movies.iter().map(|x| x.id.clone()).collect();

        self.start_thread();
        self.advance_phase();
    }
}

impl PopupTrait for FetchArtworksPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, None)
    }

    fn update_next_frame(&self) -> bool {
        !matches!(self.phase, Phase::Done)
    }

    fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        match self.phase {
            Phase::MovieArtworks => {
                for (item_id, fetch_result) in self.rx_fetch_response.as_ref().unwrap().try_iter() {
                    let ItemID::Movie(id) = item_id else {
                        unreachable!()
                    };

                    if let Err(error) = fetch_result {
                        self.errored = Some((id.tmdb, format!("{error:#}")));
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Movie(id));
                    } else {
                        if let Some((i, _)) = self.errored {
                            if i == id.tmdb {
                                self.errored = None;
                            }
                        }

                        self.progress += 1;
                    }
                }

                if self.progress == self.movies.len() {
                    self.advance_phase();
                }
            }
            Phase::PersonArtworks => {
                for (item_id, fetch_result) in self.rx_fetch_response.as_ref().unwrap().try_iter() {
                    let ItemID::Person(id) = item_id else {
                        unreachable!()
                    };

                    if let Err(error) = fetch_result {
                        self.errored = Some((id, format!("{error:#}")));
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Person(id));
                    } else {
                        if let Some((i, _)) = self.errored {
                            if i == id {
                                self.errored = None;
                            }
                        }

                        self.progress += 1;
                    }
                }

                if self.progress == self.persons.len() {
                    self.advance_phase();
                }
            }
            Phase::CollectionArtworks => {
                for (item_id, fetch_result) in self.rx_fetch_response.as_ref().unwrap().try_iter() {
                    let ItemID::Collection(id) = item_id else {
                        unreachable!()
                    };

                    if let Err(error) = fetch_result {
                        self.errored = Some((id, format!("{error:#}")));
                        _ = self
                            .tx_fetch_request
                            .as_ref()
                            .unwrap()
                            .send(ItemID::Collection(id));
                    } else {
                        if let Some((i, _)) = self.errored {
                            if i == id {
                                self.errored = None;
                            }
                        }

                        self.progress += 1;
                    }
                }

                if self.progress == self.collections.len() {
                    self.advance_phase();
                }
            }
            _ => (),
        }
    }

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();

        let progress = self.progress;
        let num_movies = self.movies.len();

        let popup_area = create_popup(
            frame,
            dynamic_area(
                if self.errored.is_some() { 11 } else { 9 },
                5.5,
                frame.area(),
            ),
            " Fetching posters ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
            tailwind::BLUE.c950,
            self.errored.is_some(),
        );

        let [_, throbber_area, _, progress_area] = vertical![==1, ==1, ==1, ==3].areas(popup_area);

        let [throbber_area] = horizontal![==1].flex(Flex::Center).areas(throbber_area);

        let throbber = Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));
        frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);

        let progress_area = add_padding(progress_area, Padding::horizontal(2));

        let progress_gauge = Gauge::default()
            .ratio(if num_movies == 0 {
                0.0
            } else {
                progress as f64 / num_movies as f64
            })
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(
                format!("{}/{}", progress, num_movies)
                    .fg(tailwind::PINK.c500)
                    .bold(),
            )
            .use_unicode(true);

        frame.render_widget(progress_gauge, progress_area);

        if let Some((id, error)) = self.errored.as_ref() {
            let errored_text = format!("{id} errored: {error}");

            let text_area = add_padding(
                vertical![>=1, ==2].split(popup_area)[1],
                Padding::horizontal(2),
            );
            frame.render_widget(
                Text::from_iter(wrap_text(&errored_text, text_area.width as usize))
                    .fg(tailwind::RED.c500)
                    .bold()
                    .centered(),
                text_area,
            );
        }
    }
}
