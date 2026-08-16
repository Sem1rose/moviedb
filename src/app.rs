use std::{cell::RefCell, fs, path::PathBuf, rc::Rc, thread, time::Duration};

use anyhow::bail;
use itertools::Itertools;
use log::{error, info};
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventState, KeyModifiers};

use crate::{
    config::Config,
    drawer::Drawer,
    helpers::{default_rc, new_rc},
    key_event_handler::KeyEventHandler,
    load_file, omdb,
    popups::Popups,
    screens::Screens,
    tokens::*,
    types::{
        Collection, Entry, FxIndexMap, HistoryEntry, ListID, ListItem, Movie, MovieDetailsResponse,
        Person, Term, initialize_terminal, try_restore_terminal,
    },
};

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

    pub trakt_tokens:      TraktTokens,
    pub punch_play_tokens: PunchPlayTokens,
    pub tmdb_tokens:       TMDBTokens,
    pub omdb_tokens:       OMDBTokens,
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

            trakt_tokens: TraktTokens::new(&home_dir),
            punch_play_tokens: PunchPlayTokens::new(&home_dir),
            tmdb_tokens: TMDBTokens::new(&home_dir),
            omdb_tokens: OMDBTokens::new(&home_dir),

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

            self.terminal
                .draw(|frame| {
                    self.drawer.render_app(frame, &mut self.key_event_handler);
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

    #[allow(clippy::too_many_arguments)]
    pub fn fetch_movie(
        omdb_api_key: &str,
        trakt_client_id: &str,
        punch_play_access_token: &str,
        tmdb_access_token: &str,
        trakt_status: Option<bool>,
        punch_play_status: Option<bool>,
        omdb_status: bool,
        tmdb_id: u32,
    ) -> anyhow::Result<MovieDetailsResponse> {
        let mut trakt_result = None;
        let mut punch_play_result = None;
        let mut omdb_result = None;

        thread::scope(|s| {
            let tmdb_handle = {
                s.spawn(move || tmdb::movie::get_movie_details(tmdb_access_token, tmdb_id, true))
            };
            let punch_play_handle = if punch_play_status.unwrap_or(false) {
                Some(s.spawn(move || {
                    punch_play::movie::get_movie_details(punch_play_access_token, tmdb_id)
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
            let trakt_handle = if trakt_status.is_some() {
                Some({
                    let imdb_id = imdb_id.clone();
                    s.spawn(move || trakt::movie::get_movie_details(trakt_client_id, &imdb_id))
                })
            } else {
                None
            };
            let omdb_handle = if omdb_status {
                Some({
                    let imdb_id = imdb_id.clone();
                    s.spawn(move || omdb::get_movie_details(omdb_api_key, &imdb_id))
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

            // _ = tmdb::movie::get_movie_artworks(
            //     &cache_dir,
            //     &tmdb_access_token,
            //     tmdb_result.as_ref(),
            //     tmdb_id,
            // );

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
            if let Some(Popups::EditMovie(edit_movie_popup)) = self.drawer.active_popup.as_mut() {
                let rating = format!(
                    "{:.1}",
                    edit_movie_popup.rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                let date = if ["now", ""].contains(
                    &edit_movie_popup.date_input.lines()[0]
                        .trim()
                        .to_lowercase()
                        .as_str(),
                ) {
                    chrono::Local::now()
                } else {
                    edit_movie_popup.rating_input.lines()[0].parse().unwrap()
                };

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
            }

            let watchlist = &mut main_screen.lists.get_mut(&ListID::Watchlist).unwrap().items;
            if let Some(index) = watchlist.iter().position(|x| x.id == movie_id) {
                watchlist.swap_remove(index);
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
        let (movie, date, rating) = if let Some(Popups::AddMovie(add_movie_popup)) =
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
            let watchlist = &mut main_screen.lists.get_mut(&ListID::Watchlist).unwrap().items;
            if let Some(index) = watchlist.iter().position(|x| x.id == movie_id) {
                watchlist.swap_remove(index);
            }

            if matches!(main_screen.selected_list, ListID::Watched) {
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
                    .iter()
                    .any(|x| x.id == movie_id)
                {
                    main_screen
                        .lists
                        .get_mut(&main_screen.selected_list)
                        .unwrap()
                        .items
                        .push(ListItem {
                            id:       movie_id,
                            added_at: date.naive_local(),
                        });

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
        let movie = if let Some(Popups::AddMovie(add_movie_popup)) =
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
            if let Some(Popups::EditMovie(edit_movie_popup)) = self.drawer.active_popup.as_ref() {
                let rating = format!(
                    "{:.1}",
                    edit_movie_popup.rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                let date = if ["now", ""].contains(
                    &edit_movie_popup.date_input.lines()[0]
                        .trim()
                        .to_lowercase()
                        .as_str(),
                ) {
                    chrono::Local::now()
                } else {
                    edit_movie_popup.date_input.lines()[0].parse().unwrap()
                };

                self.watched
                    .borrow_mut()
                    .entry(main_screen.current_movie().unwrap().id)
                    .and_modify(|x| {
                        if let Some(latest) = x.history.last_mut() {
                            latest.date = date;
                            latest.rating = rating;
                        };
                        x.history
                            .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
                    });
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
            } else {
                let index = main_screen
                    .lists
                    .get_mut(&main_screen.selected_list)
                    .unwrap()
                    .items
                    .iter()
                    .position(|x| x.id == movie_id)
                    .unwrap();
                main_screen
                    .lists
                    .get_mut(&main_screen.selected_list)
                    .unwrap()
                    .items
                    .remove(index);

                main_screen.save_lists();
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

    pub fn set_tmdb_user_tokens(&mut self) {
        if let Some(Popups::TMDBInit(tmdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = tmdb_init_popup.user_tokens.take() {
                self.tmdb_tokens.set_creds(tokens).unwrap();
            }
        }
        self.drawer
            .image_renderer
            .update_access_token(self.tmdb_tokens.access_token());
        self.drawer.close_popup();
    }

    pub fn set_punch_play_user_tokens(&mut self) {
        if let Some(Popups::PunchPlayInit(punch_play_init_popup)) =
            self.drawer.active_popup.as_mut()
        {
            if let Some(tokens) = punch_play_init_popup.user_tokens.take() {
                self.punch_play_tokens.set_creds(tokens).unwrap();
            }
            self.drawer.close_popup();
        }
    }

    pub fn set_trakt_user_tokens(&mut self) {
        if let Some(Popups::TraktInit(trakt_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = trakt_init_popup.user_tokens.take() {
                self.trakt_tokens.set_creds(tokens).unwrap();
            }
            self.drawer.close_popup();
        }
    }

    pub fn set_omdb_user_tokens(&mut self) {
        if let Some(Popups::OMDBInit(omdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = omdb_init_popup.tokens.take() {
                self.omdb_tokens.set_creds(tokens).unwrap();
            }
            self.drawer.close_popup();
        }
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
            Event::Paste(string) =>
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
                },
            Event::Resize(_, _) => (),
        }
    }
}
