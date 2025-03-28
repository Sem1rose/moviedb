use crate::app::{Config, Errors, OptionalResult, Result};
use cocoon::Cocoon;
use log::{debug, error};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{stdin, stdout, Write},
    sync::mpsc::Sender,
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TMDBCredentials {
    access_token: String,
    session_id: String,
}

impl TMDBCredentials {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
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

    tx_init: Sender<OptionalResult<String>>,
}

impl TMDBConfig {
    pub fn new(tx_init: Sender<OptionalResult<String>>) -> Self {
        Self {
            tx_init,
            tmdb_credentials: TMDBCredentials::default(),
        }
    }

    fn try_init(&mut self, config: &Config) -> Result<bool> {
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
            // self.read_creds(config)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn init(&mut self, config: &Config) {
        let result = self.try_init(config);
        if let Ok(true) = result {
            let tx_result = self.tx_init.clone();
            let conf_cloned = config.clone();

            thread::spawn(move || {
                tx_result.send(TMDBConfig::read_creds(&conf_cloned).map_err(|error| Some(error)))
            });
        } else if let Ok(false) = result {
            // debug!("Initializing a new TMDB config...");

            let _ = self.tx_init.send(Err(None));

            // self.init_creds();
        } else if let Err(error) = result {
            // error!("Error reading TMDB config file, initializing a new config...");

            let _ = self.tx_init.send(Err(Some(error)));

            // self.init_creds();
        }
    }

    fn read_creds(config: &Config) -> Result<String> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(&config.dirs.tmdb_encrypted_file)?;

        let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);

        result.map_err(|error| Errors::Other(format!("TMDB: error decoding utf8: {}", error)))

        // if let Ok(decrypted_creds) = result {
        //     Ok(decrypted_creds)
        //     // self.set_creds(decrypted_creds)
        // } else {
        //     Err(Errors::Other(format!(
        //         "TMDB: error decoding utf8: {}",
        //         result.unwrap_err()
        //     )))
        // }
    }

    // fn try_read_creds(config: &Config, tx_init: Sender<Result<String>>) -> Result<()> {
    //     let key = fs::read(&config.dirs.encryption_key_file)?;
    //     let cocoon = Cocoon::new(&key);

    //     let mut encrypted_file = File::open(&config.dirs.tmdb_encrypted_file)?;

    //     let result = String::from_utf8(cocoon.parse(&mut encrypted_file)?);
    //     if let Ok(decrypted_creds) = result {
    //         tx_init.send(Ok(decrypted_creds));
    //         Ok(())
    //     } else {
    //         Err(Errors::Other(format!(
    //             "TMDB: error decoding utf8: {}",
    //             result.unwrap_err()
    //         )))
    //     }
    // }

    pub fn set_creds(&mut self, data: String) -> Result<()> {
        self.tmdb_credentials = serde_json::from_str(&data)?;

        Ok(())
    }

    pub fn init_creds(&mut self, access_token: String) {
        // let access_token = self.get_input(String::from("Enter your access token:"));

        self.tmdb_credentials = TMDBCredentials::new(access_token);
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
        self.tmdb_credentials.session_id = session_id;
    }

    pub fn has_session_id(&self) -> bool {
        self.tmdb_credentials.session_id != *""
    }

    pub fn access_token(&self) -> &str {
        &self.tmdb_credentials.access_token
    }

    pub fn session_id(&self) -> &str {
        &self.tmdb_credentials.session_id
    }
}
