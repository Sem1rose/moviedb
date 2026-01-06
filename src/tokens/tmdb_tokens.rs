use crate::{tokens::Credentials, types::*};
use anyhow::Context;
use cocoon::Cocoon;
// use log::{debug, error};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Tokens {
    session_id: String,
}

#[derive(Clone, Default)]
pub struct TMDBTokens {
    // tx_init: Sender<OptionalResult<String>>,
    tmdb_credentials: Tokens,

    access_token: String,
}

impl TMDBTokens {
    pub fn new(/*tx_init: Sender<OptionalResult<String>>,*/ creds: &Credentials) -> Self {
        Self {
            // tx_init,
            access_token: creds.tmdb_access_token.clone(),

            tmdb_credentials: Tokens::default(),
        }
    }

    fn check_files(&mut self, home_dir: &PathBuf) -> anyhow::Result<bool> {
        if !home_dir.join(".key").is_file() {
            let key: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            _ = fs::remove_file(&home_dir.join(".tmdb_tokens"));

            fs::write(&home_dir.join(".key"), key)?;
        }

        Ok(home_dir.join(".tmdb_tokens").is_file())
    }

    pub fn init(&mut self, home_dir: &PathBuf) {
        let result = self.check_files(home_dir);
        if let Ok(true) = result {
            // let tx_result = self.tx_init.clone();
            let home_dir = home_dir.clone();

            // thread::spawn(move || tx_result.send(TMDBTokens::read_creds(&home_dir).map_err(Some)));
        } else if let Ok(false) = result {
            // debug!("Initializing a new TMDB config...");

            // _ = self.tx_init.send(Err(None));
        } else if let Err(error) = result {
            // error!("Error reading TMDB config file, initializing a new config...");

            // _ = self.tx_init.send(Err(Some(error)));
        }
    }

    fn read_creds(home_dir: &PathBuf) -> anyhow::Result<String> {
        let key = fs::read(&home_dir.join(".key"))?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&home_dir.join(".tmdb_tokens"))?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);

        result.context("TMDB: error decoding utf8")
    }

    pub fn set_creds(&mut self, data: String) -> anyhow::Result<()> {
        self.tmdb_credentials = serde_json::from_str(&data)?;

        Ok(())
    }

    pub fn save_creds(&self, home_dir: &PathBuf) -> anyhow::Result<()> {
        let key = fs::read(&home_dir.join(".key"))?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&home_dir.join(".tmdb_tokens"))?;
        let dump_json = serde_json::to_string(&self.tmdb_credentials)?;

        cocoon.dump(dump_json.into_bytes(), &mut encrypted_file)?;

        Ok(())
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn access_token_owned(&self) -> String {
        self.access_token.clone()
    }

    pub fn set_access_token(&mut self, access_token: String) {
        self.access_token = access_token;
    }

    pub fn session_id(&self) -> &str {
        &self.tmdb_credentials.session_id
    }

    pub fn session_id_owned(&self) -> String {
        self.tmdb_credentials.session_id.clone()
    }

    pub fn set_session_id(&mut self, session_id: String) {
        self.tmdb_credentials.session_id = session_id;
    }

    pub fn has_session_id(&self) -> bool {
        !self.tmdb_credentials.session_id.is_empty()
    }
}
