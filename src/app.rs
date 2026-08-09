use std::{cell::RefCell, fs, path::PathBuf, rc::Rc, time::Duration};

use itertools::Itertools;
use log::error;
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventState, KeyModifiers};

use crate::{
    config::Config,
    drawer::Drawer,
    helpers::{default_rc, new_rc},
    key_event_handler::KeyEventHandler,
    load_file,
    popups::Popups,
    screens::Screens,
    tokens::*,
    types::{
        Collection, Entry, FxIndexMap, HistoryEntry, Movie, Person, Term, initialize_terminal,
        reset_terminal,
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

            config: config,
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

            for callback in self.key_event_handler.get_execute_immediates() {
                callback(self, crate::key_event_handler::Data::None);
            }

            if !self.drawer.check_refresh_immediate() {
                if self.drawer.check_refresh_delayed() {
                    if event::poll(Duration::from_millis(15))? {
                        if let Ok(event) = event::read() {
                            self.handle_event(event);
                        }
                    }
                } else {
                    if let Ok(event) = event::read() {
                        self.handle_event(event);
                    }
                }
            }

            if self.quit {
                break;
            }
        }

        reset_terminal(&mut self.terminal)?;

        Ok(())
    }

    pub fn add_play(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            let selected_movie_id = main_screen.current_movie().unwrap().id;
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
                    .entry(selected_movie_id)
                    .and_modify(|x| x.add_play(date, rating, None))
                    .or_insert(Entry {
                        movie_id: selected_movie_id,
                        history:  vec![HistoryEntry {
                            date,
                            rating,
                            note: None,
                        }],
                    });
            }
            main_screen.filter_sort_movies(None);
            main_screen.goto_index(
                main_screen
                    .filtered_movies
                    .iter()
                    .position(|x| x.id == selected_movie_id)
                    .unwrap() as isize,
            );
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

        self.watched
            .borrow_mut()
            .entry(movie_id)
            .and_modify(|x| x.add_play(date, rating, None))
            .or_insert(Entry {
                movie_id: movie_id,
                history:  vec![HistoryEntry {
                    date,
                    rating,
                    note: None,
                }],
            });

        // if the movie is already cached, remove it because the info is probably outdated.
        self.movies
            .borrow_mut()
            .entry(movie_id)
            .and_modify(|x| *x = movie);

        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            main_screen.filter_sort_movies(None);
            main_screen.goto_index(
                main_screen
                    .filtered_movies
                    .iter()
                    .position(|x| x.id == movie_id)
                    .unwrap() as isize,
            );
        }

        self.drawer.close_popup();
        self.save_data(true, true, false, false);
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
                        x.history.last_mut().map(|x| {
                            x.date = date;
                            x.rating = rating;
                        });
                        x.history
                            .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
                    });
            }
            main_screen.filter_sort_movies(Some(true));
        }

        self.save_data(false, true, false, false);
    }

    pub fn remove_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            self.watched
                .borrow_mut()
                .swap_remove(&main_screen.current_movie().unwrap().id);

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

            main_screen.filter_sort_movies(None);
            main_screen.goto_index(new_selected_index);
        }

        self.save_data(false, true, false, false);
    }

    pub fn set_trakt_user_tokens(&mut self) {
        if let Some(Popups::TraktInit(trakt_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = trakt_init_popup.user_tokens.take() {
                self.trakt_tokens.set_creds(tokens).unwrap();
            }
            self.drawer.close_popup();
        }
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

    pub fn set_omdb_user_tokens(&mut self) {
        if let Some(Popups::OMDBInit(omdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = omdb_init_popup.tokens.take() {
                self.omdb_tokens.set_creds(tokens).unwrap();
            }
            self.drawer.close_popup();
        }
    }

    fn save_data(
        &self,
        save_movies: bool,
        save_watched: bool,
        save_persons: bool,
        save_collections: bool,
    ) {
        macro_rules! save {
            ($name:expr, $obj:expr) => {{
                let path = &self.home_dir.join(format!("{}.json", $name));
                match serde_json::to_string(&$obj.collect_vec()) {
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
            }};
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
                for c in string.chars() {
                    if let Some((callback, data)) = self.key_event_handler.handle_key_event(
                        KeyEvent {
                            code:      event::KeyCode::Char(c),
                            modifiers: KeyModifiers::NONE,
                            kind:      event::KeyEventKind::Press,
                            state:     KeyEventState::NONE,
                        },
                        &self.drawer,
                    ) {
                        callback(self, data);
                    }
                },
            Event::Resize(_, _) => (),
        }
    }
}
