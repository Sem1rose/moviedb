use crate::app::{Config, Errors, Result};
use cocoon::Cocoon;
use log::{debug, error};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{stdin, stdout, Write},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TMDBCredentials {
    access_token: Box<str>,
    session_id: Box<str>,
}

impl TMDBCredentials {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token: access_token.into_boxed_str(),
            session_id: "".into(),
        }
    }

    pub fn default() -> Self {
        Self {
            access_token: "".into(),
            session_id: "".into(),
        }
    }
}

#[derive(Clone)]
pub struct TMDBConfig {
    tmdb_credentials: TMDBCredentials,
}

impl TMDBConfig {
    pub fn new() -> Self {
        Self {
            tmdb_credentials: TMDBCredentials::default(),
        }
    }

    pub fn init(&mut self, config: &Config) -> Result<()> {
        if !config.dirs.encryption_key_file.is_file() {
            let key: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            let _ = fs::remove_file(&config.dirs.tmdb_encrypted_file);

            fs::write(&config.dirs.encryption_key_file, key)?;
        }

        if config.dirs.tmdb_encrypted_file.is_file() {
            if self.read_creds(config).is_err() {
                error!("Error reading TMDB config file, initializing a new config...");

                self.init_creds();
            }
        } else {
            debug!("Initializing a new TMDB config...");

            self.init_creds();
        }

        Ok(())
    }

    fn init_creds(&mut self) {
        let access_token = self.get_input(String::from("Enter your access token:"));

        self.tmdb_credentials = TMDBCredentials::new(access_token);
    }

    fn read_creds(&mut self, config: &Config) -> Result<()> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&config.dirs.tmdb_encrypted_file)?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);
        if let Ok(decrypted_creds) = result {
            self.tmdb_credentials = serde_json::from_str(&decrypted_creds)?;
        } else {
            return Err(Errors::Other(format!(
                "TMDB: error decoding utf8: {}",
                result.unwrap_err()
            )));
        }

        // debug!("tmdb credentials: {:#?}", self.tmdb_credentials);
        Ok(())
    }

    pub fn save_creds(&self, config: &Config) -> Result<()> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&config.dirs.tmdb_encrypted_file)?;
        let dump_json = serde_json::to_string(&self.tmdb_credentials)?;

        cocoon.dump(dump_json.into_bytes(), &mut encrypted_file)?;

        Ok(())
    }

    fn get_input(&self, prompt: String) -> String {
        print!("{prompt} ");
        let _ = stdout().flush();

        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .expect("Did not enter a correct string");
        if let Some('\n') = input.chars().next_back() {
            input.pop();
        }
        if let Some('\r') = input.chars().next_back() {
            input.pop();
        }

        input
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.tmdb_credentials.session_id = session_id.into_boxed_str();
    }

    pub fn has_session_id(&self) -> bool {
        self.tmdb_credentials.session_id != "".into()
    }

    pub fn access_token(&self) -> &str {
        &self.tmdb_credentials.access_token
    }

    pub fn session_id(&self) -> &str {
        &self.tmdb_credentials.session_id
    }
}
