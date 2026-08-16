use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use itertools::Itertools;
use log::info;
use ratatui::{
    Frame,
    layout::Flex,
    macros::{horizontal, vertical},
    style::{Style, Stylize, palette::tailwind},
    text::Text,
    widgets::{Gauge, Padding},
};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    app::App,
    helpers,
    key_event_handler::KeyEventHandler,
    popups::PopupTrait,
    tokens::{OMDBTokens, PunchPlayTokens, TraktTokens, tmdb_tokens::TMDBTokens},
    types::{Collection, Entry, FxIndexMap, List, Movie, MovieDetailsResponse, Person},
    widgets,
};

const BATCH_SIZE: usize = 8;
#[derive(Default)]
pub struct FetchMoviesPopup {
    count:    usize,
    progress: usize,
    num_sent: usize,
    errored:  Option<(u32, String)>,
    pub done: bool,
    started:  bool,

    movies:           Rc<RefCell<FxIndexMap<u32, Movie>>>,
    collections:      Rc<RefCell<FxIndexMap<u32, Collection>>>,
    persons:          Rc<RefCell<FxIndexMap<u32, Person>>>,
    unfetched_movies: Vec<u32>,

    tmdb_tokens:       TMDBTokens,
    punch_play_tokens: PunchPlayTokens,
    trakt_tokens:      TraktTokens,
    omdb_tokens:       OMDBTokens,

    tx_details_request:  Option<Sender<u32>>,
    rx_details_response: Option<Receiver<(u32, anyhow::Result<MovieDetailsResponse>)>>,

    tick:           u64,
    throbber_state: ThrobberState,
}

impl FetchMoviesPopup {
    fn start_thread(mut self) -> Self {
        if self.done {
            return self;
        }
        let (tx_details_request, rx_details_request) = channel::<u32>();
        let (tx_details_response, rx_details_response) =
            channel::<(u32, anyhow::Result<MovieDetailsResponse>)>();

        let trakt_status = self.trakt_tokens.status;
        let punch_play_status = self.punch_play_tokens.status;
        let omdb_status = self.omdb_tokens.status;
        let omdb_api_key = self.omdb_tokens.key_owned();
        let trakt_client_id = self.trakt_tokens.client_id_owned();
        let punch_play_access_token = self.punch_play_tokens.access_token_owned();
        let tmdb_access_token = self.tmdb_tokens.access_token_owned();

        thread::spawn(move || {
            for movie_id in rx_details_request.iter() {
                let tx_response = tx_details_response.clone();

                let omdb_api_key = omdb_api_key.clone();
                let trakt_client_id = trakt_client_id.clone();
                let punch_play_access_token = punch_play_access_token.clone();
                let tmdb_access_token = tmdb_access_token.clone();
                let tmdb_access_token = tmdb_access_token.clone();
                thread::spawn(move || {
                    _ = tx_response.send((
                        movie_id,
                        App::fetch_movie(
                            &omdb_api_key,
                            &trakt_client_id,
                            &punch_play_access_token,
                            &tmdb_access_token,
                            trakt_status,
                            punch_play_status,
                            omdb_status,
                            movie_id,
                        ),
                    ));
                });
            }
        });

        self.tx_details_request = Some(tx_details_request);
        self.rx_details_response = Some(rx_details_response);
        self.started = true;

        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
        movies: Rc<RefCell<FxIndexMap<u32, Movie>>>,
        collections: Rc<RefCell<FxIndexMap<u32, Collection>>>,
        persons: Rc<RefCell<FxIndexMap<u32, Person>>>,
        watched: &FxIndexMap<u32, Entry>,
        lists: &[&List],
    ) {
        let unfetched_movies = {
            let borrowed_movies = movies.borrow();
            watched
                .keys()
                .chain(lists.iter().flat_map(|x| x.items.iter().map(|x| &x.id)))
                .filter(|x| !borrowed_movies.contains_key(*x))
                .copied()
                .collect_vec()
        };

        info!("{unfetched_movies:?}");

        *self = Self {
            tmdb_tokens,
            punch_play_tokens,
            trakt_tokens,
            omdb_tokens,

            movies,
            collections,
            persons,
            count: unfetched_movies.len(),
            done: unfetched_movies.is_empty(),
            unfetched_movies,

            ..Default::default()
        }
        .start_thread();
    }
}

impl PopupTrait for FetchMoviesPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, None)
    }

    fn update_next_frame(&self) -> bool {
        self.started && !self.done
    }

    fn update(&mut self) {
        if !self.started {
            return;
        }

        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        if let (Some(rx_details_response), Some(tx_details_request)) = (
            self.rx_details_response.as_ref(),
            self.tx_details_request.as_ref(),
        ) {
            for (movie_id, fetch_result) in rx_details_response.try_iter() {
                match fetch_result {
                    Ok(mut movie_details) => {
                        if let Some((i, _)) = self.errored {
                            if i == movie_id {
                                self.errored = None;
                            }
                        }

                        self.progress += 1;

                        let tmdb_movie_details = movie_details.tmdb.take().unwrap();
                        let trakt_movie_details = movie_details.trakt.take();
                        let punch_play_movie_details = movie_details.punch_play.take();
                        let omdb_movie_details = movie_details.omdb.take();
                        if let Some(credits) = tmdb_movie_details.credits.as_ref() {
                            for person in credits.cast.iter().take(14).chain(credits.crew.iter()) {
                                self.persons
                                    .borrow_mut()
                                    .entry(person.id)
                                    .or_insert(person.into());
                            }
                        }
                        if let Some(collection_details) =
                            tmdb_movie_details.collection_details.as_ref()
                        {
                            self.collections
                                .borrow_mut()
                                .entry(collection_details.id)
                                .and_modify(|x| {
                                    if x.parts.is_empty() {
                                        x.parts =
                                            collection_details.parts.iter().map(|x| x.id).collect();
                                    }
                                })
                                .or_insert(Collection {
                                    id:    collection_details.id,
                                    name:  collection_details.name.clone(),
                                    parts: collection_details.parts.iter().map(|x| x.id).collect(),
                                });
                        } else if let Some(collection) =
                            tmdb_movie_details.belongs_to_collection.as_ref()
                        {
                            self.collections
                                .borrow_mut()
                                .entry(collection.id)
                                .or_insert(Collection {
                                    id:    collection.id,
                                    name:  collection.name.clone(),
                                    parts: vec![],
                                });
                        }

                        let mut movie = Movie::from(tmdb_movie_details);
                        if let Some(trakt_details) = trakt_movie_details {
                            movie.add_trakt_details(trakt_details);
                        }
                        if let Some(punch_play_details) = punch_play_movie_details {
                            movie.add_punch_play_details(punch_play_details);
                        }
                        if let Some(omdb) = omdb_movie_details {
                            movie.add_omdb_details(omdb);
                        }

                        info!("{movie:#?}");
                        match self.movies.borrow_mut().entry(movie_id) {
                            indexmap::map::Entry::Occupied(mut occupied_entry) =>
                                *occupied_entry.get_mut() = movie,
                            indexmap::map::Entry::Vacant(vacant_entry) => {
                                vacant_entry.insert_entry(movie);
                            }
                        }
                    }
                    Err(error) => {
                        self.errored = Some((movie_id, format!("{error:#}")));
                        _ = tx_details_request.send(movie_id);
                    }
                }
            }

            if self.num_sent.saturating_sub(self.progress) < BATCH_SIZE {
                for tmdb_id in self.unfetched_movies.drain(
                    ..(BATCH_SIZE - (self.num_sent.saturating_sub(self.progress)))
                        .min(self.unfetched_movies.len()),
                ) {
                    _ = tx_details_request.send(tmdb_id);
                    self.num_sent += 1;
                }
            }
        }

        if self.progress == self.count {
            self.done = true;
        }
    }

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        if !self.started {
            return;
        }

        key_event_handler.clear();

        let popup_area = widgets::window(
            frame,
            helpers::centered_area(
                if self.errored.is_some() { 11 } else { 9 },
                60,
                frame.area(),
            ),
            " Fetching Movies ",
            self.errored.is_some(),
        );

        let [_, throbber_area, _, progress_area] = vertical![==1, ==1, ==1, ==3].areas(popup_area);

        let [throbber_area] = horizontal![==1].flex(Flex::Center).areas(throbber_area);

        let throbber = Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));
        frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);

        let progress_area = helpers::add_padding(progress_area, Padding::horizontal(2));

        let progress_gauge = Gauge::default()
            .ratio(if self.count == 0 {
                0.0
            } else {
                self.progress as f64 / self.count as f64
            })
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(
                format!("{}/{}", self.progress, self.count)
                    .fg(tailwind::PINK.c500)
                    .bold(),
            )
            .use_unicode(true);

        frame.render_widget(progress_gauge, progress_area);

        if let Some((id, error)) = self.errored.as_ref() {
            let errored_text = format!("{id} errored: {error}");

            let text_area = helpers::add_padding(
                vertical![>=1, ==2].split(popup_area)[1],
                Padding::horizontal(2),
            );
            frame.render_widget(
                Text::from_iter(helpers::wrap_text(&errored_text, text_area.width as usize))
                    .fg(tailwind::RED.c500)
                    .bold()
                    .centered(),
                text_area,
            );
        }
    }
}
