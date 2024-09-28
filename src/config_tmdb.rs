use cocoon::Cocoon;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File},
    io::{stdin, stdout, Write},
    path::Path,
};

const ENCRYPTION_KEY: &str = ".key";
const ENCRYPTED_FILE: &str = ".credentials";
#[derive(Serialize, Deserialize, Debug)]
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

pub struct Conf {
    pub home: Box<Path>,
    pub cache: Box<Path>,
    tmdb_credentials: TMDBCredentials,
}

impl Conf {
    pub fn new() -> Self {
        let home = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join("moviedb");
        let cache = dirs::cache_dir()
            .expect("Couldn't get user's cache dir")
            .join("moviedb");
        Self {
            home: home.into_boxed_path(),
            cache: cache.into_boxed_path(),
            tmdb_credentials: TMDBCredentials::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.home.is_dir() {
            fs::create_dir(&self.home)?;
        }

        if !self.home.join(ENCRYPTION_KEY).is_file() {
            let key: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            let _ = fs::remove_file(self.home.join(ENCRYPTED_FILE));

            fs::write(self.home.join(ENCRYPTION_KEY), key)?;
        }

        if self.home.join(ENCRYPTED_FILE).is_file() {
            if self.read_creds().is_err() {
                self.init_creds();
            }
        } else {
            self.init_creds();
        }
        Ok(())
    }

    fn init_creds(&mut self) {
        let access_token = self.get_input(String::from("Enter your access token:"));

        self.tmdb_credentials = TMDBCredentials::new(access_token);
    }

    fn read_creds(&mut self) -> Result<(), Box<dyn Error>> {
        let key = fs::read(self.home.join(ENCRYPTION_KEY))?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(self.home.join(ENCRYPTED_FILE))?;
        let decrypted_creds = String::from_utf8(cocoon.parse(&mut encrypted_file)?)?;

        self.tmdb_credentials = serde_json::from_str(&decrypted_creds)?;
        // println!("{:#?}", self.trakt_credentials);
        Ok(())
    }

    pub fn save_creds(&self) -> Result<(), Box<dyn Error>> {
        let key = fs::read(self.home.join(ENCRYPTION_KEY))?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(self.home.join(ENCRYPTED_FILE))?;
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
