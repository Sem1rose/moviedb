use crate::{config::Config, trakt::TraktTokens, types::*};
use cocoon::Cocoon;
use log::{debug, error};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    sync::mpsc::Sender,
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TraktCredentials {
    client_secret: String,
    client_id: String,
    access_token: String,
    refresh_token: String,
    expires_on: i32,
}

impl Default for TraktCredentials {
    fn default() -> Self {
        Self {
            client_secret: "".into(),
            client_id: "".into(),
            access_token: "".into(),
            refresh_token: "".into(),
            expires_on: -1,
        }
    }
}
impl TraktCredentials {
    pub fn new(client_secret: String, client_id: String) -> Self {
        Self {
            client_secret,
            client_id,
            access_token: "".into(),
            refresh_token: "".into(),
            expires_on: -1,
        }
    }
}

#[derive(Clone)]
pub struct TraktConfig {
    trakt_credentials: TraktCredentials,

    tx_init: Sender<OptionalResult<String>>,
}

impl TraktConfig {
    pub fn new(tx_init: Sender<OptionalResult<String>>) -> Self {
        Self {
            tx_init,
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
            // self.read_creds(config)?;

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

            // self.init_creds();
        } else if let Err(error) = result {
            // error!("Error reading Trakt config file, initializing a new config...");

            let _ = self.tx_init.send(Err(Some(error)));

            // self.init_creds();
            // } else {
            //     let _ = self.tx_init.send(Ok(()));
        }
    }

    fn read_creds(config: &Config) -> Result<String> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&config.dirs.trakt_encrypted_creds_file)?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);

        result.map_err(|error| Errors::Other(format!("Trakt: error decoding utf8: {}", error)))
        // if let Ok(decrypted_creds) = result {
        //     self.set_creds(decrypted_creds)
        // } else {
        //     Err(Errors::Other(format!(
        //         "Trakt: error decoding utf8: {}",
        //         result.unwrap_err()
        //     )))
        // }
    }

    pub fn set_creds(&mut self, data: String) -> Result<()> {
        self.trakt_credentials = serde_json::from_str(&data)?;

        Ok(())
    }

    pub fn init_creds(&mut self, client_id: String, client_secret: String) {
        // let client_id = self.get_input(String::from("Enter your client id:"));
        // let client_secret = self.get_input(String::from("Enter your client secret:"));

        self.trakt_credentials = TraktCredentials::new(client_secret, client_id);
    }

    pub fn save_creds(&self, config: &Config) -> Result<()> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&config.dirs.trakt_encrypted_creds_file)?;
        let dump_json = serde_json::to_string(&self.trakt_credentials)?;

        cocoon.dump(dump_json.into_bytes(), &mut encrypted_file)?;

        Ok(())
    }

    // fn get_input(&self, prompt: String) -> String {
    //     print!("{prompt} ");
    //     let _ = stdout().flush();

    //     let mut input = String::new();
    //     stdin()
    //         .read_line(&mut input)
    //         .expect("Did not enter a correct string");
    //     if let Some('\n') = input.chars().next_back() {
    //         input.pop();
    //     }
    //     if let Some('\r') = input.chars().next_back() {
    //         input.pop();
    //     }

    //     input
    // }

    pub fn set_secrets(&mut self, client_id: &str, client_secret: &str) {
        self.trakt_credentials.client_id = client_id.to_string();
        self.trakt_credentials.client_secret = client_secret.to_string();
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
        &self.trakt_credentials.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.trakt_credentials.client_secret
    }

    pub fn access_token(&self) -> &str {
        &self.trakt_credentials.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.trakt_credentials.refresh_token
    }

    pub fn client_id_owned(&self) -> String {
        self.trakt_credentials.client_id.clone()
    }

    pub fn client_secret_owned(&self) -> String {
        self.trakt_credentials.client_secret.clone()
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
