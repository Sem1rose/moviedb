pub mod config_omdb;
pub mod config_tmdb;
pub mod config_trakt;

use crate::types::Result;
use serde::Deserialize;
use std::{
    fs::{create_dir, write},
    path::PathBuf,
};

#[derive(Deserialize)]
pub struct Credentials {
    pub trakt_client_id: String,
    pub trakt_client_secret: String,
    pub tmdb_access_token: String,
    pub omdb_key: String,
}

#[derive(Clone)]
pub struct Dirs {
    pub home: PathBuf,
    pub cache: PathBuf,
    pub ratings_file: PathBuf,
    pub encryption_key_file: PathBuf,
    pub trakt_encrypted_creds_file: PathBuf,
    pub tmdb_encrypted_creds_file: PathBuf,
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
            trakt_encrypted_creds_file: trakt_encrypted_file,
            tmdb_encrypted_creds_file: tmdb_encrypted_file,
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

impl Default for Config {
    fn default() -> Self {
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
}

impl Config {
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
