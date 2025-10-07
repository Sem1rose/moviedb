use crate::{
    config::{Config, Credentials},
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TMDBCredentials {
    session_id: String,
}

// impl TMDBCredentials {
//     pub fn new() -> Self {
//         Self {
//             session_id: "".into(),
//         }
//     }
// }

#[derive(Clone)]
pub struct TMDBConfig {
    tmdb_credentials: TMDBCredentials,

    access_token: String,
    tx_init: Sender<OptionalResult<String>>,
}

impl TMDBConfig {
    pub fn new(tx_init: Sender<OptionalResult<String>>) -> Self {
        Self {
            tx_init,
            access_token: "".into(),
            tmdb_credentials: TMDBCredentials::default(),
        }
    }

    fn check_files(&mut self, config: &Config) -> Result<bool> {
        if !config.dirs.encryption_key_file.is_file() {
            let key: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            let _ = fs::remove_file(&config.dirs.tmdb_encrypted_creds_file);

            fs::write(&config.dirs.encryption_key_file, key)?;
        }

        if config.dirs.tmdb_encrypted_creds_file.is_file() {
            // self.read_creds(config)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn init(&mut self, config: &Config) {
        let file_contents =
            read_to_string(".credentials").expect("Couldn't read credentials from .credentials!");
        let creds: Credentials = serde_json::from_str(&file_contents)
            .expect("Couldn't deserialize credentials at .credentials");

        self.set_access_token(creds.tmdb_access_token);

        let result = self.check_files(config);
        if let Ok(true) = result {
            let tx_result = self.tx_init.clone();
            let conf_cloned = config.clone();

            thread::spawn(move || {
                tx_result.send(TMDBConfig::read_creds(&conf_cloned).map_err(Some))
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

        let mut encrypted_file = File::open(&config.dirs.tmdb_encrypted_creds_file)?;

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

    // pub fn init_creds(&mut self, access_token: String) {
    //     // let access_token = self.get_input(String::from("Enter your access token:"));

    //     self.tmdb_credentials = TMDBCredentials::new(access_token);
    // }

    pub fn save_creds(&self, config: &Config) -> Result<()> {
        let key = fs::read(&config.dirs.encryption_key_file)?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(&config.dirs.tmdb_encrypted_creds_file)?;
        let dump_json = serde_json::to_string(&self.tmdb_credentials)?;

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
        self.tmdb_credentials.session_id != *""
    }
}
