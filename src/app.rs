use crate::{
    config_tmdb::TMDBConfig,
    config_trakt::TraktConfig,
    draw::Drawer,
    draw::Popup,
    tmdb::{self, TMDBDetailsResponse},
    trakt::{self, TraktDetailsResponse},
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use serde::Deserialize;
use std::{error::Error, fs, path::Path};

#[derive(Clone)]
pub struct Config {
    pub home: Box<Path>,
    pub cache: Box<Path>,
}
impl Config {
    pub fn new() -> Self {
        let home = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join("moviedb");
        let cache = dirs::cache_dir()
            .expect("Couldn't get user's cache dir")
            .join("moviedb");

        Self {
            home: home.into_boxed_path(),
            cache: cache.into_boxed_path(),
        }
    }

    pub fn init_dirs(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.home.is_dir() {
            fs::create_dir(&self.home)?;
        }
        if !self.home.join("ratings.json").is_file() {
            fs::write(self.home.join("ratings.json"), "[]")?;
        }
        if !self.cache.is_dir() {
            fs::create_dir(&self.cache)?;
        }
        if !self.cache.join("posters").is_dir() {
            fs::create_dir(self.cache.join("posters"))?;
        }
        if !self.cache.join("backdrops").is_dir() {
            fs::create_dir(self.cache.join("backdrops"))?;
        }
        Ok(())
    }
}

pub struct App {
    pub single_shot: bool,
    pub should_quit: bool,
    pub movies: Vec<Movie>,
    pub config: Config,
    pub tmdb_config: TMDBConfig,
    pub trakt_config: TraktConfig,
}

impl App {
    pub fn new(_single_shot: bool) -> Result<Self, Box<dyn Error>> {
        let mut config = Config::new();
        config.init_dirs()?;

        let tmdb_config = TMDBConfig::new();
        let trakt_config = TraktConfig::new();

        Ok(Self {
            single_shot: _single_shot,
            should_quit: false,
            movies: vec![],
            config,
            tmdb_config,
            trakt_config,
        })
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.tmdb_config.init(&self.config)?;
        tmdb::populate_tokens(&self.config, &mut self.tmdb_config)?;
        self.trakt_config.init(&self.config)?;
        trakt::populate_tokens(&self.config, &mut self.trakt_config)?;

        self.fetch_movies();

        Ok(())
    }

    pub fn set_movies(&mut self, _movies: Vec<Movie>) {
        self.movies = _movies;
    }

    pub fn fetch_movies(&mut self) {
        let file_path = self.config.home.join("ratings.json");

        let file_contents = fs::read_to_string(&file_path).unwrap_or_else(|_| {
            panic!("Couldn't read database contents at {}", file_path.display())
        });

        let movies = serde_json::from_str(&file_contents).expect("couldn't deserialize json!");

        self.set_movies(movies);
    }

    pub fn save_movies(&self) -> Result<(), Box<dyn Error>> {
        let string = serde_json::to_string_pretty(self.movies.as_slice()).unwrap();

        fs::rename(
            self.config.home.join("ratings.json"),
            self.config.home.join("ratings.json.bak"),
        )?;
        fs::write(self.config.home.join("ratings.json"), string)?;

        Ok(())
    }

    pub fn handle(&mut self, drawer: &mut Drawer) -> Result<(), Box<dyn Error>> {
        if event::poll(std::time::Duration::from_millis(0))? {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent { code, kind, .. }) => {
                    if kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    match code {
                        KeyCode::Char('q') => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Char('a') => {
                            if drawer.popup.is_none() {
                                drawer.open_add_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Char('e') => {
                            if drawer.popup.is_none() {
                                drawer.open_edit_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Char('d') => {
                            if drawer.popup.is_none() {
                                drawer.open_remove_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Delete => {
                            if drawer.popup.is_none() {
                                drawer.open_remove_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Esc => {
                            if drawer.popup.is_some() {
                                drawer.close_popups();
                                drawer.clear_images(false);
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Up => {
                            drawer.dec_selection(self);
                        }
                        KeyCode::Down => {
                            drawer.inc_selection(self);
                        }
                        KeyCode::Right => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            } else {
                                drawer.inc_selection_horiz(self);
                            }
                        }
                        KeyCode::Left => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            } else {
                                drawer.dec_selection_horiz(self);
                            }
                        }
                        KeyCode::Enter => match drawer.popup {
                            Some(Popup::AddMovie) => {
                                if *drawer.add_movie_popup_options.failed.lock().unwrap() {
                                    drawer.close_popups();
                                    drawer.clear_images(false);
                                } else if drawer.add_movie_popup_options.phase == 0
                                    && drawer.add_movie_popup_options.search_input.value() != ""
                                {
                                    drawer.add_movie_popup_options.finished_search_input = true;
                                    drawer.update = true;
                                } else if drawer.add_movie_popup_options.phase == 2 {
                                    drawer.add_movie_popup_options.movie_selected = true;
                                    drawer.update = true;
                                } else if drawer.add_movie_popup_options.phase == 3
                                    && drawer.add_movie_popup_options.search_input.value() != ""
                                    && drawer.add_movie_popup_options.user_rating_valid
                                {
                                    drawer.add_movie_popup_options.got_user_rating = true;
                                    drawer.update = true;
                                }
                            }
                            Some(Popup::EditMovie) => {
                                if drawer.edit_movie_popup_options.errored {
                                    drawer.close_popups();
                                    drawer.clear_images(false);
                                    drawer.update = true;
                                } else if !drawer.edit_movie_popup_options.got_user_rating
                                    && drawer.edit_movie_popup_options.user_rating_input.value()
                                        != ""
                                    && drawer.edit_movie_popup_options.user_rating_valid
                                {
                                    drawer.edit_movie_popup_options.got_user_rating = true;
                                    drawer.update = true;
                                }
                            }
                            Some(Popup::RemoveMovie) => {
                                if drawer.remove_movie_popup_options.errored {
                                    drawer.close_popups();
                                    drawer.clear_images(false);
                                } else if drawer.remove_movie_popup_options.selected == 1 {
                                    drawer.remove_movie_popup_options.confirmed = true;
                                    drawer.update = true;
                                } else if drawer.remove_movie_popup_options.selected == 0 {
                                    drawer.close_popups();
                                    drawer.clear_images(false);
                                }
                            }
                            _ => {}
                        },
                        _ => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                    }
                }
                Event::Resize(_, _) => {
                    drawer.clear_images(true);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub name: String,
    pub tmdb_id: u32,
    pub imdb_id: String,
    pub year: String,
    pub user_rating: f64,
    pub language: String,
    pub ratings: Vec<Rating>,
    pub genres: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<u32>,
    pub overview: String,
    pub runtime: u32,
    pub released: bool,
    pub tagline: String,
    pub trailer: Option<String>,
    // pub finished_at: String,
}

impl Movie {
    pub fn from(movie_details: TMDBDetailsResponse, user_rating: f64) -> Self {
        let mut collection: Option<String> = None;
        let mut collection_id: Option<u32> = None;
        if movie_details.belongs_to_collection.is_some() {
            collection = Some(movie_details.belongs_to_collection.clone().unwrap().name);
            collection_id = Some(movie_details.belongs_to_collection.clone().unwrap().id);
        }
        Self {
            name: movie_details.title,
            user_rating,
            ratings: vec![Rating::TMDB(
                movie_details.vote_average,
                movie_details.vote_count,
            )],
            year: movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
            language: movie_details.original_language,
            tmdb_id: movie_details.id,
            imdb_id: movie_details.imdb_id,
            genres: movie_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview: movie_details.overview,
            collection,
            collection_id,
            runtime: movie_details.runtime,
            released: movie_details.status == "Released",
            tagline: movie_details.tagline,
            trailer: None,
            // finished_at: "".into(),
        }
    }

    pub fn add_trakt_details(mut self, trakt_details: TraktDetailsResponse) -> Self {
        self.ratings
            .push(Rating::Trakt(trakt_details.rating, trakt_details.votes));
        self.trailer = trakt_details.trailer;

        self
    }
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub enum Rating {
    Trakt(f64, u32),
    TMDB(f64, u32),
}
