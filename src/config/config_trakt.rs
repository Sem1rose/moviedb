use crate::{
    config::{Config, Credentials},
    trakt::TraktTokens,
    types::*,
};
use cocoon::Cocoon;
use log::{debug, error};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, read_to_string, File},
    sync::mpsc::Sender,
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TraktCredentials {
    access_token: String,
    refresh_token: String,
    expires_on: i32,
}

impl Default for TraktCredentials {
    fn default() -> Self {
        Self {
            access_token: "".into(),
            refresh_token: "".into(),
            expires_on: -1,
        }
    }
}

#[derive(Clone)]
pub struct TraktConfig {
    trakt_credentials: TraktCredentials,

    client_secret: String,
    client_id: String,
    tx_init: Sender<OptionalResult<String>>,
}

impl TraktConfig {
    pub fn new(tx_init: Sender<OptionalResult<String>>) -> Self {
        Self {
            tx_init,
            client_id: "".into(),
            client_secret: "".into(),
            trakt_credentials: TraktCredentials::default(),
        }
    }

    fn check_files(&mut self, config: &Config) -> Result<bool> {
        if !config.dirs.encryption_key_file.is_file() {
            let key: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            let _ = fs::remove_file(&config.dirs.trakt_encrypted_creds_file);

            fs::write(&config.dirs.encryption_key_file, key)?;
        }

        if config.dirs.trakt_encrypted_creds_file.is_file() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn init(&mut self, config: &Config) {
        let result = self.check_files(config);
        if let Ok(true) = result {
            let tx_result = self.tx_init.clone();
            let conf_cloned = config.clone();

            thread::spawn(move || {
                tx_result.send(TraktConfig::read_creds(&conf_cloned).map_err(Some))
            });
        } else if let Ok(false) = result {
            // debug!("Initializing a new Trakt config...");

            let _ = self.tx_init.send(Err(None));
        } else if let Err(error) = result {
            // error!("Error reading Trakt config file, initializing a new config...");

            let _ = self.tx_init.send(Err(Some(error)));
        }
    }

    fn read_creds(config: &Config) -> Result<String> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&config.dirs.trakt_encrypted_creds_file)?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);

        result.map_err(|error| Errors::Other(format!("Trakt: error decoding utf8: {}", error)))
    }

    pub fn set_creds(&mut self, data: String) -> Result<()> {
        self.trakt_credentials = serde_json::from_str(&data)?;

        Ok(())
    }

    pub fn save_creds(&self, config: &Config) -> Result<()> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&config.dirs.trakt_encrypted_creds_file)?;
        let dump_json = serde_json::to_string(&self.trakt_credentials)?;

        cocoon.dump(dump_json.into_bytes(), &mut encrypted_file)?;

        Ok(())
    }

    pub fn set_secrets(&mut self, client_id: String, client_secret: String) {
        self.client_id = client_id;
        self.client_secret = client_secret;
    }

    pub fn set_tokens(&mut self, tokens: TraktTokens) {
        self.trakt_credentials.access_token = tokens.access_token;
        self.trakt_credentials.refresh_token = tokens.refresh_token;
        self.trakt_credentials.expires_on = tokens.expires_on;
    }

    pub fn has_tokens(&self) -> bool {
        !self.trakt_credentials.access_token.is_empty()
            && !self.trakt_credentials.refresh_token.is_empty()
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn access_token(&self) -> &str {
        &self.trakt_credentials.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.trakt_credentials.refresh_token
    }

    pub fn client_id_owned(&self) -> String {
        self.client_id.clone()
    }

    pub fn client_secret_owned(&self) -> String {
        self.client_secret.clone()
    }

    pub fn access_token_owned(&self) -> String {
        self.trakt_credentials.access_token.clone()
    }

    pub fn refresh_token_owned(&self) -> String {
        self.trakt_credentials.refresh_token.clone()
    }

    pub fn tokens_expiration_date(&self) -> i32 {
        self.trakt_credentials.expires_on
    }
}
