use crate::{tokens::Credentials, trakt::TokenResponse, types::*};
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
    access_token: String,
    refresh_token: String,
    expires_on: i64,
}

impl From<TokenResponse> for Tokens {
    fn from(val: TokenResponse) -> Self {
        Tokens {
            access_token: val.access_token,
            refresh_token: val.refresh_token,
            expires_on: val.created_at + val.expires_in,
        }
    }
}

#[derive(Clone, Default)]
pub struct TraktTokens {
    client_id: String,
    client_secret: String,

    tokens: Tokens,
    // tx_init: Sender<OptionalResult<String>>,
}

impl TraktTokens {
    pub fn new(/*tx_init: Sender<OptionalResult<String>>,*/ creds: &Credentials) -> Self {
        Self {
            // tx_init,
            client_id: creds.trakt_client_id.clone(),
            client_secret: creds.trakt_client_secret.clone(),

            tokens: Tokens::default(),
        }
    }

    fn check_files(&mut self, home_dir: &PathBuf) -> anyhow::Result<bool> {
        if !home_dir.join(".key").is_file() {
            let key: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            _ = fs::remove_file(&home_dir.join(".trakt_tokens"));

            fs::write(&home_dir.join(".key"), key)?;
        }

        Ok(home_dir.join(".trakt_tokens").is_file())
    }

    pub fn init(&mut self, home_dir: &PathBuf) {
        let result = self.check_files(home_dir);
        if let Ok(true) = result {
            // let tx_result = self.tx_init.clone();

            let home_dir = home_dir.clone();
            // thread::spawn(move || tx_result.send(TraktTokens::read_creds(&home_dir).map_err(Some)));
        } else if let Ok(false) = result {
            // debug!("Initializing a new Trakt config...");

            // _ = self.tx_init.send(Err(None));
        } else if let Err(error) = result {
            // error!("Error reading Trakt config file, initializing a new config...");

            // _ = self.tx_init.send(Err(Some(error)));
        }
    }

    fn read_creds(home_dir: &PathBuf) -> anyhow::Result<String> {
        let key = fs::read(&home_dir.join(".key"))?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&home_dir.join(".trakt_tokens"))?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);

        result.context("Trakt: error decoding utf8")
    }

    pub fn set_creds(&mut self, data: String) -> anyhow::Result<()> {
        self.tokens = serde_json::from_str(&data)?;

        Ok(())
    }

    pub fn save_creds(&self, home_dir: &PathBuf) -> anyhow::Result<()> {
        let key = fs::read(&home_dir.join(".key"))?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&home_dir.join(".trakt_tokens"))?;
        let dump_json = serde_json::to_string(&self.tokens)?;

        cocoon.dump(dump_json.into_bytes(), &mut encrypted_file)?;

        Ok(())
    }

    pub fn set_secrets(&mut self, client_id: String, client_secret: String) {
        self.client_id = client_id;
        self.client_secret = client_secret;
    }

    pub fn has_tokens(&self) -> bool {
        !(self.tokens.access_token.is_empty() || self.tokens.refresh_token.is_empty())
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn access_token(&self) -> &str {
        &self.tokens.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.tokens.refresh_token
    }

    pub fn client_id_owned(&self) -> String {
        self.client_id.clone()
    }

    pub fn client_secret_owned(&self) -> String {
        self.client_secret.clone()
    }

    pub fn access_token_owned(&self) -> String {
        self.tokens.access_token.clone()
    }

    pub fn refresh_token_owned(&self) -> String {
        self.tokens.refresh_token.clone()
    }

    pub fn expires_on(&self) -> i64 {
        self.tokens.expires_on
    }
}
