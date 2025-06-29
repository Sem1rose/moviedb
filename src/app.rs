use crate::{
    config_tmdb::TMDBConfig,
    config_trakt::TraktConfig,
    draw::Drawer,
    popups::Popups,
    screens::Screens,
    tmdb::{RequestResponseError, TMDBDetailsResponse},
    trakt::{TokenResponseError, TraktDetailsResponse},
};
use log::{debug, error};
use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
use serde::Deserialize;
use std::{
    fs::{create_dir, read_to_string, rename, write},
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
};

pub type Result<T> = color_eyre::Result<T, Errors>;
pub type OptionalResult<T> = color_eyre::Result<T, Option<Errors>>;

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
    EyreReport(#[from] color_eyre::Report),

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
    pub config: Config,

    pub rx_tmdb: Receiver<OptionalResult<String>>,
    pub tmdb_config: TMDBConfig,

    pub rx_trakt: Receiver<OptionalResult<String>>,
    pub trakt_config: TraktConfig,

    pub movies: Vec<Movie>,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.init_dirs()?;

        let (tx_tmdb, rx_tmdb) = channel();
        let tmdb_config = TMDBConfig::new(tx_tmdb);

        let (tx_trakt, rx_trakt) = channel();
        let trakt_config = TraktConfig::new(tx_trakt);

        Ok(Self {
            movies: vec![],
            config,

            rx_tmdb,
            tmdb_config,

            rx_trakt,
            trakt_config,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        // self.tmdb_config.init(&self.config)?;
        // tmdb::populate_tokens(&self.config, &mut self.tmdb_config)?;
        // debug!("TMDB config init finished successfully.");

        // self.trakt_config.init(&self.config)?;
        // trakt::populate_tokens(&self.config, &mut self.trakt_config)?;
        // debug!("Trakt config init finished successfully.");

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
            let movies: Vec<Movie> = result.unwrap();
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
                if event.code == KeyCode::Char('*') {
                    panic!("RELEASE ME!");
                }

                if drawer.active_popup.is_some() {
                    match drawer.active_popup.as_ref().unwrap() {
                        Popups::AddMovie => {
                            drawer.add_movie_popup_handle_key_events(self, event)?;
                        }
                        Popups::EditMovie => {
                            drawer.edit_movie_popup_handle_key_events(event)?;
                        }
                        Popups::RemoveMovie => {
                            drawer.remove_movie_popup_handle_key_events(event)?;
                        }
                        Popups::Error => {
                            drawer.error_popup_handle_key_events(event)?;
                        }
                        Popups::TMDBInit => {
                            drawer.tmdb_init_popup_handle_key_events(event)?;
                        }
                        Popups::TraktInit => {
                            // drawer.error_popup_handle_key_events(event)?;
                        }
                        Popups::FetchArtwork => (),
                    }
                } else {
                    match drawer.current_screen {
                        Screens::InitScreen => (),
                        Screens::MainScreen => {
                            drawer.main_screen_handle_key_events(self, event)?;
                        }
                        Screens::TermSizeWarn => (),
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
