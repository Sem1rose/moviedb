use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use log::info;

use crate::{
    app::App,
    key_event_handler::KeyEventHandler,
    processors::ProcessorTrait,
    tokens::{OMDBTokens, PunchPlayTokens, TraktTokens, tmdb_tokens::TMDBTokens},
    types::{Collection, FxIndexMap, Movie, MovieDetailsResponse, Person},
};

const BATCH_SIZE: usize = 12;
#[derive(Default)]
pub struct MoviesFetcherProcessor {
    initialized: bool,
    progress:    usize,
    num_sent:    usize,
    pub idle:    bool,

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
}

impl MoviesFetcherProcessor {
    fn start_thread(mut self) -> Self {
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
                        App::fetch_movie_details(
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
    ) {
        if self.initialized {
            return;
        }

        *self = Self {
            initialized: true,
            tmdb_tokens,
            punch_play_tokens,
            trakt_tokens,
            omdb_tokens,

            movies,
            collections,
            persons,
            unfetched_movies: Vec::with_capacity(64),

            ..Default::default()
        }
        .start_thread();
    }

    pub fn fetch_movies(&mut self, ids: &[u32]) {
        if !self.initialized {
            return;
        }

        self.idle = false;

        let movies_borrowed = self.movies.borrow();
        self.unfetched_movies
            .extend(ids.iter().filter(|x| !movies_borrowed.contains_key(*x)));
    }
}

impl ProcessorTrait for MoviesFetcherProcessor {
    fn update(&mut self, _key_event_handler: &mut KeyEventHandler) {
        if !self.initialized {
            return;
        }

        if let (Some(rx_details_response), Some(tx_details_request)) = (
            self.rx_details_response.as_ref(),
            self.tx_details_request.as_ref(),
        ) {
            for (movie_id, fetch_result) in rx_details_response.try_iter() {
                match fetch_result {
                    Ok(mut movie_details) => {
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
                    Err(_) => {
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

        if !self.idle && self.num_sent == self.progress && self.unfetched_movies.is_empty() {
            self.idle = true;
            self.num_sent = 0;
            self.progress = 0;
        }
    }
}
