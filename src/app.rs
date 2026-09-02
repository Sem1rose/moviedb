use std::{cell::RefCell, fs, path::PathBuf, rc::Rc, thread, time::Duration};

use anyhow::{anyhow, bail};
use itertools::Itertools;
use log::{error, info};
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventState, KeyModifiers};
use rustc_hash::FxHashMap;
use strum::IntoDiscriminant;

use crate::{
    config::Config,
    drawer::Drawer,
    helpers::{default_rc, new_rc},
    image_backend::ImageID,
    key_event_handler::KeyEventHandler,
    load_file, omdb,
    popups::Popup,
    processors::{Processor, ProcessorDiscriminants},
    screens::Screens,
    tokens::*,
    types::{
        Collection, Entry, FxIndexMap, HistoryEntry, ListID, ListItem, Movie, MovieDetailsResponse,
        Person, SyncItem, Term, initialize_terminal, try_restore_terminal,
    },
};

const SYNC: bool = true;

pub struct App {
    pub _cache_dir: PathBuf,
    pub home_dir:   PathBuf,
    pub quit:       bool,

    pub movies:      Rc<RefCell<FxIndexMap<u32, Movie>>>,
    pub watched:     Rc<RefCell<FxIndexMap<u32, Entry>>>,
    pub persons:     Rc<RefCell<FxIndexMap<u32, Person>>>,
    pub collections: Rc<RefCell<FxIndexMap<u32, Collection>>>,

    terminal:              Term,
    pub drawer:            Drawer,
    pub key_event_handler: KeyEventHandler,
    pub config:            Rc<RefCell<Config>>,

    pub tmdb_tokens:       TMDBTokens,
    pub simkl_tokens:      SimklTokens,
    pub punch_play_tokens: PunchPlayTokens,
    pub trakt_tokens:      TraktTokens,
    pub omdb_tokens:       OMDBTokens,

    processors: FxHashMap<ProcessorDiscriminants, Processor>,
}

impl App {
    pub fn new() -> Self {
        let home_dir = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join("moviedb");
        let cache_dir = dirs::cache_dir()
            .expect("Couldn't get user's cache dir")
            .join("moviedb");
        let config = new_rc(Config::new(&home_dir));

        Self {
            terminal: initialize_terminal().expect("Unable to initialize terminal"),
            key_event_handler: KeyEventHandler::default(),
            drawer: Drawer::new(&home_dir, &cache_dir, config.clone()),

            config,
            movies: default_rc(),
            watched: default_rc(),
            persons: default_rc(),
            collections: default_rc(),

            tmdb_tokens: TMDBTokens::new(&home_dir),
            simkl_tokens: SimklTokens::new(&home_dir),
            punch_play_tokens: PunchPlayTokens::new(&home_dir),
            trakt_tokens: TraktTokens::new(&home_dir),
            omdb_tokens: OMDBTokens::new(&home_dir),

            processors: FxHashMap::from_iter(
                Processor::default_all()
                    .into_iter()
                    .map(|x| (x.discriminant(), x)),
            ),
            quit: false,
            home_dir,
            _cache_dir: cache_dir,
        }
        .load_data()
    }

    pub fn load_data(self) -> Self {
        if let Some(x) = load_file!("movies", self.home_dir) {
            *self.movies.borrow_mut() =
                FxIndexMap::from_iter(x.into_iter().map(|x: Movie| (x.id, x)));
        }
        if let Some(x) = load_file!("watched", self.home_dir) {
            *self.watched.borrow_mut() =
                FxIndexMap::from_iter(x.into_iter().map(|x: Entry| (x.movie_id, x)));
        }
        if let Some(x) = load_file!("persons", self.home_dir) {
            *self.persons.borrow_mut() =
                FxIndexMap::from_iter(x.into_iter().map(|x: Person| (x.id, x)));
        }
        if let Some(x) = load_file!("collections", &self.home_dir) {
            *self.collections.borrow_mut() =
                FxIndexMap::from_iter(x.into_iter().map(|x: Collection| (x.id, x)));
        }

        self
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.key_event_handler.clear();

            self.update_processors();

            self.terminal
                .draw(|frame| {
                    self.drawer.render_app(
                        frame,
                        &mut self.key_event_handler,
                        self.processors.values().filter(|x| x.needs_render()),
                    );
                })
                .map(|_| ())?;

            let mut executed_immediate = false;
            for callback in self.key_event_handler.get_execute_immediates() {
                callback(self, crate::key_event_handler::Data::None);
                executed_immediate = true;
            }

            if !executed_immediate
                && event::poll(if self.drawer.check_refresh_immediate() {
                    Duration::ZERO
                } else if self.drawer.check_refresh_delayed() {
                    Duration::from_millis(15)
                } else {
                    Duration::MAX
                })?
            {
                if let Ok(event) = event::read() {
                    self.handle_event(event);
                }
            }

            if self.quit {
                break;
            }
        }

        try_restore_terminal()?;

        Ok(())
    }

    pub fn initialize_processors(&mut self) {
        for processor in self.processors.values_mut() {
            match processor {
                Processor::HistorySyncer(history_syncer_processor) => history_syncer_processor
                    .initialize(
                        self.tmdb_tokens.clone(),
                        self.simkl_tokens.clone(),
                        self.punch_play_tokens.clone(),
                    ),
            }
        }
    }

    fn update_processors(&mut self) {
        for processor in self.processors.values_mut() {
            processor.update(&mut self.key_event_handler);
        }
    }

    // pub fn get_processor(&self, processor: ProcessorDiscriminants) -> Option<&Processor> {
    //     self.processors.get(&processor)
    // }
    pub fn get_processor_mut(
        &mut self,
        processor: ProcessorDiscriminants,
    ) -> Option<&mut Processor> {
        self.terminal.backend_mut();
        self.processors.get_mut(&processor)
    }

    pub fn fetch_movie_details(
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
        tmdb_id: u32,
    ) -> anyhow::Result<MovieDetailsResponse> {
        let mut trakt_result = None;
        let mut punch_play_result = None;
        let mut omdb_result = None;

        thread::scope(|s| {
            let tmdb_handle =
                { s.spawn(|| tmdb::movie::get_movie_details(tmdb_tokens.access_token(), tmdb_id)) };
            let punch_play_handle = if punch_play_tokens.status.unwrap_or(false) {
                Some(s.spawn(move || {
                    punch_play::movie::get_movie_details(punch_play_tokens.access_token(), tmdb_id)
                }))
            } else {
                None
            };
            let tmdb_result = match tmdb_handle.join() {
                Err(e) => {
                    bail!("{:#?}", e)
                }
                Ok(val) => match val {
                    Err(error) => {
                        bail!(error);
                    }
                    Ok(val) => Some(val),
                },
            };

            let imdb_id = tmdb_result.as_ref().unwrap().imdb_id.clone();
            let trakt_handle = if trakt_tokens.status.is_some() {
                Some({
                    let imdb_id = imdb_id.clone();
                    s.spawn(move || {
                        trakt::movie::get_movie_details(trakt_tokens.client_id(), &imdb_id)
                    })
                })
            } else {
                None
            };
            let omdb_handle = if false && omdb_tokens.status {
                Some({
                    let imdb_id = imdb_id.clone();
                    s.spawn(move || omdb::get_movie_details(omdb_tokens.key(), &imdb_id))
                })
            } else {
                None
            };

            if let Some(handle) = trakt_handle {
                trakt_result = handle
                    .join()
                    .map(|x| {
                        x.inspect_err(|err| {
                            error!("Trakt error while fetching movie details: {err:?}")
                        })
                        .ok()
                    })
                    .ok()
                    .flatten();
            }
            if let Some(handle) = punch_play_handle {
                punch_play_result = handle
                    .join()
                    .map(|x| {
                        x.inspect_err(|err| {
                            error!("PunchPlay error while fetching movie details: {err:?}")
                        })
                        .ok()
                    })
                    .ok()
                    .flatten();
            }
            if let Some(handle) = omdb_handle {
                omdb_result = handle
                    .join()
                    .map(|x| {
                        x.inspect_err(|err| {
                            error!("OMDB error while fetching movie details: {err:?}")
                        })
                        .ok()
                    })
                    .ok()
                    .flatten();
            }

            _ = tmdb::movie::get_movie_artworks(
                &dirs::cache_dir()
                    .expect("Couldn't get user's cache dir")
                    .join("moviedb"),
                tmdb_tokens.access_token(),
                tmdb_result.clone(),
                tmdb_id,
                None,
            );

            Ok(MovieDetailsResponse {
                tmdb:       tmdb_result,
                punch_play: punch_play_result,
                trakt:      trakt_result,
                omdb:       omdb_result,
            })
        })
    }

    pub fn add_play(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            let movie_id = main_screen.current_movie().unwrap().id;
            if let Some(Popup::ManagePlays(manage_plays_popup)) = self.drawer.active_popup.as_mut()
            {
                let rating = format!(
                    "{:.1}",
                    manage_plays_popup.rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                let input = manage_plays_popup.date_input.lines()[0].to_lowercase();
                let date = if ["now", ""].contains(&input.trim()) {
                    chrono::Local::now()
                } else if input.trim() == "unknown" {
                    Default::default()
                } else {
                    input.parse().unwrap()
                }
                .to_utc();

                self.watched
                    .borrow_mut()
                    .entry(movie_id)
                    .and_modify(|x| x.add_play(date, rating, None))
                    .or_insert(Entry {
                        movie_id,
                        history: vec![HistoryEntry {
                            date,
                            rating,
                            note: None,
                        }],
                    });
                if SYNC {
                    if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                        .processors
                        .get_mut(&ProcessorDiscriminants::HistorySyncer)
                    {
                        history_syncer_processor.add_sync_item(SyncItem::AddPlay {
                            movie_id,
                            date,
                            rating,
                        });
                    }
                }
            }

            let watchlist = &mut main_screen.lists.get_mut(&ListID::Watchlist).unwrap().items;
            if let Some(index) = watchlist.keys().position(|x| *x == movie_id) {
                watchlist.swap_remove(&(index as u32));
            }
            main_screen.save_lists();

            if matches!(main_screen.selected_list, ListID::Watchlist) {
                if self.watched.borrow().contains_key(&movie_id) {
                    main_screen.open_list_and_select_movie(
                        &mut self.key_event_handler,
                        ListID::Watched,
                        movie_id,
                    );
                }
            } else {
                main_screen.filter_sort_movies(false);
                main_screen.goto_index(
                    main_screen
                        .filtered_movies
                        .iter()
                        .position(|x| x.id == movie_id)
                        .unwrap() as isize,
                );
            }
        }

        self.save_data(false, true, false, false);
    }

    pub fn add_movie(&mut self) {
        let (movie, date, rating) = if let Some(Popup::AddMovie(add_movie_popup)) =
            self.drawer.active_popup.as_mut()
        {
            let tmdb_movie_details = add_movie_popup.tmdb_movie_details_result.take().unwrap();
            let trakt_movie_details = add_movie_popup.trakt_movie_details_result.take();
            let punch_play_movie_details = add_movie_popup.punch_play_movie_details_result.take();
            let omdb_movie_details = add_movie_popup.omdb_movie_details_result.take();

            if let Some(credits) = tmdb_movie_details.credits.as_ref() {
                for person in credits.cast.iter().take(14).chain(credits.crew.iter()) {
                    self.persons
                        .borrow_mut()
                        .entry(person.id)
                        .or_insert(person.into());
                }
            }
            if let Some(collection_details) = tmdb_movie_details.collection_details.as_ref() {
                self.collections
                    .borrow_mut()
                    .entry(collection_details.id)
                    .and_modify(|x| {
                        if x.parts.is_empty() {
                            x.parts = collection_details.parts.iter().map(|x| x.id).collect();
                        }
                    })
                    .or_insert(Collection {
                        id:    collection_details.id,
                        name:  collection_details.name.clone(),
                        parts: collection_details.parts.iter().map(|x| x.id).collect(),
                    });
            } else if let Some(collection) = tmdb_movie_details.belongs_to_collection.as_ref() {
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

            (movie, add_movie_popup.date, add_movie_popup.user_rating)
        } else {
            unreachable!()
        };
        let movie_id = movie.id;

        info!("{movie:#?}");
        // if the movie is already cached, remove it because the info is probably outdated.
        match self.movies.borrow_mut().entry(movie_id) {
            indexmap::map::Entry::Occupied(mut occupied_entry) => *occupied_entry.get_mut() = movie,
            indexmap::map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert_entry(movie);
            }
        }

        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            if matches!(main_screen.selected_list, ListID::Watched) {
                let new_play = self.watched.borrow().contains_key(&movie_id);
                self.watched
                    .borrow_mut()
                    .entry(movie_id)
                    .and_modify(|x| x.add_play(date, rating, None))
                    .or_insert(Entry {
                        movie_id,
                        history: vec![HistoryEntry {
                            date,
                            rating,
                            note: None,
                        }],
                    });

                let watchlist = &mut main_screen.lists.get_mut(&ListID::Watchlist).unwrap().items;
                if let Some(index) = watchlist.keys().position(|&x| x == movie_id) {
                    watchlist.swap_remove(&(index as u32));
                }

                if SYNC {
                    if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                        .processors
                        .get_mut(&ProcessorDiscriminants::HistorySyncer)
                    {
                        if new_play {
                            history_syncer_processor.add_sync_item(SyncItem::AddPlay {
                                movie_id,
                                date,
                                rating,
                            });
                        } else {
                            history_syncer_processor.add_sync_item(SyncItem::AddToWatched {
                                movie_id,
                                date,
                                rating,
                            });
                        }
                    }
                }

                main_screen.filter_sort_movies(false);
                main_screen.goto_index(
                    main_screen
                        .filtered_movies
                        .iter()
                        .position(|x| x.id == movie_id)
                        .unwrap() as isize,
                );
            } else {
                if !main_screen.lists[&main_screen.selected_list]
                    .items
                    .keys()
                    .any(|&x| x == movie_id)
                {
                    if SYNC {
                        if matches!(
                            main_screen.selected_list,
                            ListID::Watchlist | ListID::PunchPlay(_) | ListID::TMDB(_)
                        ) {
                            if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                                .processors
                                .get_mut(&ProcessorDiscriminants::HistorySyncer)
                            {
                                history_syncer_processor.add_sync_item(SyncItem::AddToList {
                                    list: main_screen.selected_list,
                                    movie_id,
                                    date,
                                });
                            }
                        }
                    }

                    main_screen
                        .lists
                        .get_mut(&main_screen.selected_list)
                        .unwrap()
                        .items
                        .insert(
                            movie_id,
                            ListItem {
                                id:       movie_id,
                                added_at: date,
                            },
                        );

                    main_screen.filter_sort_movies(false);
                }
                main_screen.goto_index(
                    main_screen
                        .filtered_movies
                        .iter()
                        .position(|x| x.id == movie_id)
                        .unwrap() as isize,
                );
            }
            main_screen.save_lists();
        }

        self.drawer.close_popup();
        self.save_data(true, true, true, true);
    }

    pub fn update_movie_details(&mut self) {
        let movie = if let Some(Popup::AddMovie(add_movie_popup)) =
            self.drawer.active_popup.as_mut()
        {
            let tmdb_movie_details = add_movie_popup.tmdb_movie_details_result.take().unwrap();
            let trakt_movie_details = add_movie_popup.trakt_movie_details_result.take();
            let punch_play_movie_details = add_movie_popup.punch_play_movie_details_result.take();
            let omdb_movie_details = add_movie_popup.omdb_movie_details_result.take();

            if let Some(credits) = tmdb_movie_details.credits.as_ref() {
                for person in credits.cast.iter().take(14).chain(credits.crew.iter()) {
                    self.persons
                        .borrow_mut()
                        .entry(person.id)
                        .or_insert(person.into());
                }
            }

            if let Some(collection_details) = tmdb_movie_details.collection_details.as_ref() {
                self.collections
                    .borrow_mut()
                    .entry(collection_details.id)
                    .and_modify(|x| {
                        if x.parts.is_empty() {
                            x.parts = collection_details.parts.iter().map(|x| x.id).collect();
                        }
                    })
                    .or_insert(Collection {
                        id:    collection_details.id,
                        name:  collection_details.name.clone(),
                        parts: collection_details.parts.iter().map(|x| x.id).collect(),
                    });
            } else if let Some(collection) = tmdb_movie_details.belongs_to_collection.as_ref() {
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

            movie
        } else {
            unreachable!()
        };

        info!("{movie:#?}");
        self.movies
            .borrow_mut()
            .entry(movie.id)
            .and_modify(|x| *x = movie);

        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            main_screen.filter_sort_movies(true);
        }

        self.drawer.close_popup();
        self.save_data(true, false, true, true);
    }

    pub fn edit_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            if let Some(Popup::ManagePlays(manage_plays_popup)) = self.drawer.active_popup.as_ref()
            {
                let rating = format!(
                    "{:.1}",
                    manage_plays_popup.rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                let input = manage_plays_popup.date_input.lines()[0].to_lowercase();
                let date = if ["now", ""].contains(&input.trim()) {
                    chrono::Local::now()
                } else if input.trim() == "unknown" {
                    Default::default()
                } else {
                    input.parse().unwrap()
                }
                .to_utc();

                let movie_id = main_screen.current_movie().unwrap().id;
                self.watched.borrow_mut().entry(movie_id).and_modify(|x| {
                    if let Some(latest) = x.history.last_mut() {
                        latest.date = date;
                        latest.rating = rating;
                    };
                    x.history
                        .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
                });

                if SYNC {
                    if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                        .processors
                        .get_mut(&ProcessorDiscriminants::HistorySyncer)
                    {
                        history_syncer_processor.add_sync_item(SyncItem::Edit {
                            movie_id,
                            date,
                            rating,
                        });
                    }
                }
            }
            main_screen.filter_sort_movies(true);
        }

        self.save_data(false, true, false, false);
    }

    pub fn remove_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            let movie_id = main_screen.current_movie().unwrap().id;
            if matches!(main_screen.selected_list, ListID::Watched) {
                self.watched
                    .borrow_mut()
                    .swap_remove(&main_screen.current_movie().unwrap().id);

                if SYNC {
                    if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                        .processors
                        .get_mut(&ProcessorDiscriminants::HistorySyncer)
                    {
                        history_syncer_processor
                            .add_sync_item(SyncItem::RemoveFromWatched { movie_id });
                    }
                }
            } else {
                main_screen
                    .lists
                    .get_mut(&main_screen.selected_list)
                    .unwrap()
                    .items
                    .shift_remove(&movie_id);

                main_screen.save_lists();

                if SYNC {
                    if matches!(
                        main_screen.selected_list,
                        ListID::Watchlist | ListID::PunchPlay(_) | ListID::TMDB(_)
                    ) {
                        if let Some(Processor::HistorySyncer(history_syncer_processor)) = self
                            .processors
                            .get_mut(&ProcessorDiscriminants::HistorySyncer)
                        {
                            history_syncer_processor.add_sync_item(SyncItem::RemoveFromList {
                                list: main_screen.selected_list,
                                movie_id,
                            });
                        }
                    }
                }
            }

            let pos = main_screen.filtered_movies.iter().position(|x| {
                x.id == main_screen
                    .current_movie()
                    .map(|x| x.id)
                    .unwrap_or(u32::MAX)
            });
            let new_selected_index = if let Some(pos) = pos {
                if pos == main_screen.filtered_movies.len() - 1 {
                    pos as isize - 1
                } else {
                    pos as isize
                }
            } else {
                -1
            };

            main_screen.filter_sort_movies(false);
            main_screen.goto_index(new_selected_index);
        }

        self.save_data(false, true, false, false);
    }

    pub fn edit_movie_play(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_ref() {
            let movie_id = main_screen.current_movie().unwrap().id;

            if let Some(Popup::ManagePlays(manage_plays_popup)) = self.drawer.active_popup.as_ref()
            {
                let rating = format!(
                    "{:.1}",
                    manage_plays_popup.rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                let input = manage_plays_popup.date_input.lines()[0].to_lowercase();
                let date = if ["now", ""].contains(&input.trim()) {
                    chrono::Local::now()
                } else if input.trim() == "unknown" {
                    Default::default()
                } else {
                    input.parse().unwrap()
                }
                .to_utc();

                self.watched.borrow_mut().entry(movie_id).and_modify(|x| {
                    let len = x.history.len();
                    x.history[len - 1 - manage_plays_popup.scrollview.selected_index].date = date;
                    x.history[len - 1 - manage_plays_popup.scrollview.selected_index].rating =
                        rating;

                    x.history
                        .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
                });
            }
        }

        self.save_data(false, true, false, false);
    }

    pub fn remove_movie_play(&mut self) {
        if let Some(Popup::ManagePlays(manage_plays_popup)) = self.drawer.active_popup.as_ref() {
            let movie_id = if let Some(Screens::MainScreen(main_screen)) =
                self.drawer.current_screen.as_ref()
            {
                main_screen.current_movie().unwrap().id
            } else {
                unreachable!()
            };

            let mut empty = false;
            self.watched.borrow_mut().entry(movie_id).and_modify(|x| {
                x.history
                    .remove(x.history.len() - 1 - manage_plays_popup.scrollview.selected_index);
                empty = x.history.is_empty();
            });

            if empty {
                if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut()
                {
                    self.watched
                        .borrow_mut()
                        .swap_remove(&main_screen.current_movie().unwrap().id);
                    main_screen.filter_sort_movies(false);

                    if matches!(main_screen.selected_list, ListID::Watched) {
                        self.drawer.close_popup();
                    }
                }
            }
        }

        self.save_data(false, true, false, false);
    }

    pub fn change_movie_artworks(&mut self) {
        let mut updated = false;
        if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
            self.drawer.active_popup.as_mut()
        {
            self.movies
                .borrow_mut()
                .entry(change_artworks_popup.movie_id)
                .and_modify(|movie| {
                    let backdrop = if change_artworks_popup.chosen_backdrop == 0 {
                        None
                    } else if let Some(tmdb::smo::MovieDetails {
                        images: Some(images),
                        ..
                    }) = change_artworks_popup.movie_images.as_ref()
                    {
                        Some(
                            images.backdrops[change_artworks_popup.chosen_backdrop - 1]
                                .file_path
                                .clone(),
                        )
                    } else {
                        unreachable!()
                    };

                    let poster = if change_artworks_popup.chosen_poster == 0 {
                        None
                    } else if let Some(tmdb::smo::MovieDetails {
                        images: Some(images),
                        ..
                    }) = change_artworks_popup.movie_images.as_ref()
                    {
                        Some(
                            images.posters[change_artworks_popup.chosen_poster - 1]
                                .file_path
                                .clone(),
                        )
                    } else {
                        unreachable!()
                    };

                    info!(
                        "{backdrop:?} {:?}\n{poster:?} {:?}",
                        movie.override_backdrop, movie.override_poster
                    );

                    if backdrop != movie.override_backdrop {
                        movie.override_backdrop = backdrop;
                        self.drawer.image_renderer.delete_image_file(ImageID::Movie(
                            change_artworks_popup.movie_id,
                            None,
                            true,
                        ));
                        self.drawer.image_renderer.hash_image(ImageID::Movie(
                            change_artworks_popup.movie_id,
                            movie.override_backdrop.clone(),
                            true,
                        ));
                        updated = true;
                    }
                    if poster != movie.override_poster {
                        movie.override_poster = poster;
                        self.drawer.image_renderer.delete_image_file(ImageID::Movie(
                            change_artworks_popup.movie_id,
                            None,
                            false,
                        ));
                        self.drawer.image_renderer.hash_image(ImageID::Movie(
                            change_artworks_popup.movie_id,
                            movie.override_poster.clone(),
                            false,
                        ));
                        updated = true;
                    }
                });
        }

        if updated {
            if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
                main_screen.filter_sort_movies(true);
            }
        }

        self.save_data(true, false, false, false);
    }

    pub fn set_tmdb_user_tokens(&mut self) {
        if let Some(Popup::TMDBInit(tmdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = tmdb_init_popup.user_tokens.take() {
                self.tmdb_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer
            .image_renderer
            .update_access_token(self.tmdb_tokens.access_token());
        self.drawer.close_popup();
    }

    pub fn set_simkl_user_tokens(&mut self) {
        if let Some(Popup::SimklInit(simkl_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = simkl_init_popup.user_tokens.take() {
                self.simkl_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer.close_popup();
    }

    pub fn set_punch_play_user_tokens(&mut self) {
        if let Some(Popup::PunchPlayInit(punch_play_init_popup)) = self.drawer.active_popup.as_mut()
        {
            if let Some(tokens) = punch_play_init_popup.user_tokens.take() {
                self.punch_play_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer.close_popup();
    }

    pub fn set_trakt_user_tokens(&mut self) {
        if let Some(Popup::TraktInit(trakt_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = trakt_init_popup.user_tokens.take() {
                self.trakt_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer.close_popup();
    }

    pub fn set_omdb_user_tokens(&mut self) {
        if let Some(Popup::OMDBInit(omdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = omdb_init_popup.tokens.take() {
                self.omdb_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer.close_popup();
    }

    pub fn _refetch_watched(&mut self) {
        let punch_play_access_token = self.punch_play_tokens.access_token();
        let (Ok(mut watch_history), Ok(mut ratings)) = thread::scope(|s| {
            let watch_history_handle =
                s.spawn(move || punch_play::movie::get_watch_history(punch_play_access_token));
            let ratings_handle =
                s.spawn(move || punch_play::movie::get_rated_movies(punch_play_access_token));

            (
                watch_history_handle
                    .join()
                    .map_err(|_| anyhow!("Error joining thread"))
                    .flatten(),
                ratings_handle
                    .join()
                    .map_err(|_| anyhow!("Error joining thread"))
                    .flatten(),
            )
        }) else {
            return;
        };

        ratings.retain(|x| x.kind == "movie");
        let ratings = FxIndexMap::from_iter(ratings.into_iter().map(|x| (x.tmdb_id, x)));
        watch_history.retain(|x| x.kind == "movie");

        let history = watch_history
            .into_iter()
            .filter_map(|mut x| {
                ratings.get(&x.tmdb_id).map(|y| {
                    x.rating = y.rating;
                    // x.rated_at = y.rated_at;
                    // x.is_favourite = y.is_favourite;

                    Entry::from(x)
                })
            })
            .collect_vec();

        {
            let mut watched = self.watched.borrow_mut();
            *watched = FxIndexMap::from_iter(
                history
                    .into_iter()
                    .chain(watched.drain(..).map(|x| x.1))
                    .sorted_by_key(|x| x.movie_id)
                    .chunk_by(|x| x.movie_id)
                    .into_iter()
                    .map(|(_, x)| x.reduce(|acc, x| acc + x).unwrap())
                    .map(|x: Entry| (x.movie_id, x)),
            );
        }

        self.save_data(false, true, false, false);
    }

    pub fn save_data(
        &self,
        save_movies: bool,
        save_watched: bool,
        save_persons: bool,
        save_collections: bool,
    ) {
        macro_rules! save {
            ($name:expr, $obj:expr) => {
                let path = &self.home_dir.join(format!("{}.json", $name));
                match serde_json::to_string_pretty(&$obj.collect_vec()) {
                    Err(error) => {
                        error!("Error while trying to serialize {}: {error}", $name)
                    }
                    Ok(serialized) => {
                        _ = fs::rename(path, self.home_dir.join($name).with_extension("json.bak"));
                        if let Err(error) = fs::write(path, serialized) {
                            error!("Error while trying to save {}: {error}", $name)
                        }
                    }
                }
            };
        }

        if save_movies {
            save!("movies", self.movies.borrow().values());
        }
        if save_watched {
            save!("watched", self.watched.borrow().values());
        }
        if save_persons {
            save!("persons", self.persons.borrow().values());
        }
        if save_collections {
            save!("collections", self.collections.borrow().values());
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => {
                if let Some((callback, data)) =
                    self.key_event_handler.handle_key_event(event, &self.drawer)
                {
                    callback(self, data);
                }
            }
            Event::Mouse(event) => {
                if let Some((callback, data)) = self
                    .key_event_handler
                    .handle_mouse_event(event, &self.drawer)
                {
                    callback(self, data);
                }
            }
            Event::FocusGained => (),
            Event::FocusLost => (),
            Event::Paste(string) => {
                if let Some(callback) = self.key_event_handler.try_get_key_bind(
                    crate::key_event_handler::Bind::Input,
                    self.key_event_handler.get_state(&self.drawer),
                ) {
                    for c in string.chars() {
                        callback(
                            self,
                            crate::key_event_handler::Data::Key(KeyEvent {
                                code:      event::KeyCode::Char(c),
                                modifiers: KeyModifiers::NONE,
                                kind:      event::KeyEventKind::Press,
                                state:     KeyEventState::NONE,
                            }),
                        );
                    }
                }
            }
            Event::Resize(_, _) => (),
        }
    }
}
