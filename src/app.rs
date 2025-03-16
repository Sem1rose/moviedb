use crate::{
    config_tmdb::TMDBConfig,
    config_trakt::TraktConfig,
    draw::Drawer,
    popups::Popups,
    tmdb::{self, RequestResponseError, TMDBDetailsResponse},
    trakt::{self, TokenResponseError, TraktDetailsResponse},
};
use log::{debug, error};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use serde::Deserialize;
use std::{
    fs::{create_dir, read_to_string, rename, write},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub type Result<T> = std::result::Result<T, Errors>;

#[derive(Debug, thiserror::Error)]
pub enum Errors {
    #[error("TMDB request error: {0}")]
    TMDBRequest(#[from] RequestResponseError),

    #[error("Trakt request error: {0}")]
    TraktRequest(#[from] TokenResponseError),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("cocoon encryption error: {0}")]
    Encryption(#[from] cocoon::Error),

    #[error("image error: {0}")]
    Image(#[from] image::error::ImageError),

    #[error("ratatui_image error: {0}")]
    EncodeImage(#[from] ratatui_image::errors::Errors),

    #[error("{0}")]
    Other(String),
}

#[derive(Clone)]
pub struct Dirs {
    pub home: PathBuf,
    pub cache: PathBuf,
    pub ratings_file: PathBuf,
    pub encryption_key_file: PathBuf,
    pub trakt_encrypted_file: PathBuf,
    pub tmdb_encrypted_file: PathBuf,
    pub cached_movies_file: PathBuf,
    pub poster_cache: PathBuf,
    pub backdrop_cache: PathBuf,
}

impl Dirs {
    pub fn new(home: PathBuf, cache: PathBuf) -> Self {
        let ratings_file = home.join("ratings.json");
        let encryption_key_file = home.join(".key");
        let trakt_encrypted_file = home.join(".credentials_trakt");
        let tmdb_encrypted_file = home.join(".credentials_tmdb");
        let cached_movies_file = cache.join(".cached_movies");
        let poster_cache = cache.join("posters");
        let backdrop_cache = cache.join("backdrops");

        Self {
            home,
            cache,
            ratings_file,
            encryption_key_file,
            trakt_encrypted_file,
            tmdb_encrypted_file,
            cached_movies_file,
            poster_cache,
            backdrop_cache,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub dirs: Dirs,
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
            dirs: Dirs::new(home, cache),
        }
    }

    pub fn init_dirs(&mut self) -> Result<()> {
        if !self.dirs.home.is_dir() {
            create_dir(&self.dirs.home)?;
        }
        if !self.dirs.ratings_file.is_file() {
            write(&self.dirs.ratings_file, "[]")?;
        }
        if !self.dirs.cache.is_dir() {
            create_dir(&self.dirs.cache)?;
        }
        if !self.dirs.cached_movies_file.is_file() {
            write(&self.dirs.cached_movies_file, "")?;
        }
        if !self.dirs.poster_cache.is_dir() {
            create_dir(&self.dirs.poster_cache)?;
        }
        if !self.dirs.backdrop_cache.is_dir() {
            create_dir(&self.dirs.backdrop_cache)?;
        }
        Ok(())
    }
}

pub struct App {
    // pub single_shot: bool,
    pub should_quit: bool,
    pub movies: Vec<Movie>,

    pub config: Config,
    pub tmdb_config: TMDBConfig,
    pub trakt_config: TraktConfig,
    // pub tx_main: Sender<Event>,
    // pub rx_main: Receiver<Event>,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.init_dirs()?;

        let tmdb_config = TMDBConfig::new();
        let trakt_config = TraktConfig::new();

        // let (tx_main, rx_main) = mpsc::channel();

        Ok(Self {
            should_quit: false,
            movies: vec![],
            config,
            tmdb_config,
            trakt_config,
            // tx_main,
            // rx_main,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.tmdb_config.init(&self.config)?;
        tmdb::populate_tokens(&self.config, &mut self.tmdb_config)?;
        debug!("TMDB config init finished successfully.");

        self.trakt_config.init(&self.config)?;
        trakt::populate_tokens(&self.config, &mut self.trakt_config)?;
        debug!("Trakt config init finished successfully.");

        // let x =
        //     trakt::get_movie_poster_banner(&self.config, &self.trakt_config, "tt1130884".into());

        // if x.is_err() {
        //     panic!("{}", x.unwrap_err());
        // } else {
        //     panic!("{}", x.unwrap());
        // }

        self.fetch_movies()?;

        Ok(())
    }

    pub fn set_movies(&mut self, _movies: Vec<Movie>) {
        self.movies = _movies;
    }

    pub fn fetch_movies(&mut self) -> Result<()> {
        let file_path = &self.config.dirs.ratings_file;

        let file_contents = read_to_string(file_path).unwrap_or_else(|_| {
            panic!("Couldn't read database contents at {}", file_path.display())
        });

        let result = serde_json::from_str(&file_contents);
        if let Err(error) = result {
            error!("couldn't deserialize ratings file, backing it up and creating a blank one: {error}");

            let mut renamed = self.config.dirs.home.join("corrupted_ratings.json");
            let mut i = 1;
            while renamed.exists() {
                renamed = self
                    .config
                    .dirs
                    .home
                    .join(format!("corrupted_ratings_{i}.json"));
                i += 1;
            }

            rename(file_path, renamed)?;

            write(&self.config.dirs.ratings_file, "[]")?;
        } else {
            let movies = result.unwrap();
            self.set_movies(movies);
        }

        Ok(())
    }

    pub fn save_movies(&self) -> Result<()> {
        let string = serde_json::to_string_pretty(self.movies.as_slice()).unwrap();

        rename(
            &self.config.dirs.ratings_file,
            self.config.dirs.ratings_file.with_extension("json.bak"),
        )?;
        write(&self.config.dirs.ratings_file, string)?;

        Ok(())
    }

    pub fn handle_app_events(&mut self, event: Event, drawer: &mut Drawer) -> Result<()> {
        match event {
            Event::Key(event) => {
                let kind = event.kind;
                let code = event.code;

                if kind != KeyEventKind::Press {
                    return Ok(());
                }

                match code {
                    KeyCode::Char('Q') => {
                        panic!("RELEASE ME");
                    }
                    KeyCode::Char('q') => {
                        if drawer.accepting_input {
                            drawer.handle_input(event);
                        } else {
                            self.should_quit = true;
                        }
                    }
                    KeyCode::Char('a') => {
                        if drawer.active_popup.is_none() {
                            drawer.open_add_movie_popup();
                            // drawer.clear_images = true;
                        } else if drawer.accepting_input {
                            drawer.handle_input(event);
                        }
                    }
                    KeyCode::Char('e') => {
                        if drawer.active_popup.is_none() {
                            drawer.open_edit_movie_popup();
                            // drawer.clear_images = true;
                        } else if drawer.accepting_input {
                            drawer.handle_input(event);
                        }
                    }
                    KeyCode::Char('d') => {
                        if drawer.active_popup.is_none() {
                            drawer.open_remove_movie_popup();
                            // drawer.clear_images = true;
                        } else if drawer.accepting_input {
                            drawer.handle_input(event);
                        }
                    }
                    KeyCode::Char('f') => {
                        drawer.main_screen_options.redraw_all_image(self);
                    }
                    KeyCode::Char('g') => {
                        drawer.main_screen_options.clear_all_image();
                    }
                    KeyCode::Delete => {
                        if drawer.active_popup.is_none() {
                            drawer.open_remove_movie_popup();
                            // drawer.clear_images = true;
                        } else if drawer.accepting_input {
                            drawer.handle_input(event);
                        }
                    }
                    KeyCode::Esc => {
                        if drawer.active_popup.is_some() {
                            drawer.close_popups();
                            // drawer.clear_images(false);
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
                            drawer.handle_input(event);
                        } else {
                            drawer.inc_selection_horiz(self);
                        }
                    }
                    KeyCode::Left => {
                        if drawer.accepting_input {
                            drawer.handle_input(event);
                        } else {
                            drawer.dec_selection_horiz(self);
                        }
                    }
                    KeyCode::Enter => match drawer.active_popup {
                        Some(Popups::AddMovie) => {
                            if *drawer.add_movie_popup_options.failed.lock().unwrap() {
                                drawer.close_popups();
                                // drawer.clear_images(false);
                            } else if drawer.add_movie_popup_options.phase == 0
                                && drawer.add_movie_popup_options.search_input.value() != ""
                            {
                                drawer.add_movie_popup_options.finished_search_input = true;
                                drawer.queue_update();
                            } else if drawer.add_movie_popup_options.phase == 2 {
                                drawer.add_movie_popup_options.movie_selected = true;
                                drawer.queue_update();
                            } else if drawer.add_movie_popup_options.phase == 3
                                && drawer.add_movie_popup_options.search_input.value() != ""
                                && drawer.add_movie_popup_options.user_rating_valid
                            {
                                drawer.add_movie_popup_options.got_user_rating = true;
                                drawer.queue_update();
                            }
                        }
                        Some(Popups::EditMovie) => {
                            if drawer.edit_movie_popup_options.errored {
                                drawer.close_popups();
                                // drawer.clear_images(false);
                                drawer.queue_update();
                            } else if !drawer.edit_movie_popup_options.got_user_rating
                                && drawer.edit_movie_popup_options.user_rating_input.value() != ""
                                && drawer.edit_movie_popup_options.user_rating_valid
                            {
                                drawer.edit_movie_popup_options.got_user_rating = true;
                                drawer.queue_update();
                            }
                        }
                        Some(Popups::RemoveMovie) => {
                            if drawer.remove_movie_popup_options.errored {
                                drawer.close_popups();
                                // drawer.clear_images(false);
                            } else if drawer.remove_movie_popup_options.selected == 1 {
                                drawer.remove_movie_popup_options.confirmed = true;
                                drawer.queue_update();
                            } else if drawer.remove_movie_popup_options.selected == 0 {
                                drawer.close_popups();
                                // drawer.clear_images(false);
                            }
                        }
                        _ => {}
                    },
                    _ => {
                        if drawer.accepting_input {
                            drawer.handle_input(event);
                        }
                    }
                }
            }
            _ => (),
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
