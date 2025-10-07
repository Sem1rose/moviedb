use crate::{
    config::{config_omdb::OMDBConfig, config_tmdb::TMDBConfig, config_trakt::TraktConfig, Config},
    draw::Drawer,
    popups::Popups,
    screens::Screens,
    types::*,
};
use log::{debug, error};
use ratatui::crossterm::event::{Event, KeyCode};
use std::{
    fs::{read_to_string, rename, write},
    sync::mpsc::{channel, Receiver},
};

pub struct App {
    pub config: Config,

    pub rx_tmdb: Receiver<OptionalResult<String>>,
    pub tmdb_config: TMDBConfig,

    pub rx_trakt: Receiver<OptionalResult<String>>,
    pub trakt_config: TraktConfig,

    pub omdb_config: OMDBConfig,

    pub movies: Vec<Movie>,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut config = Config::default();
        config.init_dirs()?;

        let (tx_tmdb, rx_tmdb) = channel();
        let mut tmdb_config = TMDBConfig::new(tx_tmdb);
        tmdb_config.init(&config);

        let (tx_trakt, rx_trakt) = channel();
        let mut trakt_config = TraktConfig::new(tx_trakt);
        trakt_config.init(&config);

        let mut omdb_config = OMDBConfig::default();
        omdb_config.init();

        Ok(Self {
            movies: vec![],
            config,

            rx_tmdb,
            tmdb_config,

            rx_trakt,
            trakt_config,

            omdb_config,
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
                            if drawer.add_movie_popup.handle_key_events(self, event) {
                                drawer.close_popups();
                            }
                        }
                        Popups::EditMovie => {
                            if drawer.edit_movie_popup.handle_key_events(event) {
                                drawer.close_popups();
                            }
                        }
                        Popups::RemoveMovie => {
                            if drawer.remove_movie_popup.handle_key_events(event) {
                                drawer.close_popups();
                            }
                        }
                        Popups::Error => {
                            if drawer.error_popup_handle_key_events(event) {
                                drawer.close_popups();
                            }
                        }
                        Popups::TMDBInit => {
                            drawer.tmdb_init_popup.handle_key_events(event)?;
                        }
                        Popups::TraktInit => {
                            drawer.trakt_init_popup.handle_key_events(event)?;
                        }
                        Popups::FetchArtwork => (),
                    }
                } else {
                    match drawer.current_screen {
                        Screens::InitScreen => (),
                        Screens::MainScreen => {
                            drawer.main_screen_handle_key_events(self, event);
                        }
                        Screens::TermSizeWarn => (),
                    }
                }
            }
            Event::FocusGained => (),
            Event::FocusLost => (),
            Event::Mouse(_) => (),
            Event::Paste(_) => (),
            Event::Resize(_, _) => (),
        }

        Ok(())
    }
}
