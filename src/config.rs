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
struct TraktCredentials {
    client_secret: Box<str>,
    client_id: Box<str>,
    access_token: Box<str>,
    refresh_token: Box<str>,
    expires_on: i32,
}

impl TraktCredentials {
    pub fn new(client_secret: String, client_id: String) -> Self {
        Self {
            client_secret: client_secret.into_boxed_str(),
            client_id: client_id.into_boxed_str(),
            access_token: "".into(),
            refresh_token: "".into(),
            expires_on: -1,
        }
    }

    pub fn default() -> Self {
        Self {
            client_secret: "".into(),
            client_id: "".into(),
            access_token: "".into(),
            refresh_token: "".into(),
            expires_on: -1,
        }
    }
}

pub struct Conf {
    home: Box<Path>,
    trakt_credentials: TraktCredentials,
}

impl Conf {
    pub fn new() -> Self {
        let home = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join(".moviedb");
        Self {
            home: home.into_boxed_path(),
            trakt_credentials: TraktCredentials::default(),
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
        let client_id = self.get_input(String::from("Enter your client id:"));
        let client_secret = self.get_input(String::from("Enter your client secret:"));

        self.trakt_credentials = TraktCredentials::new(client_secret, client_id);
    }

    fn read_creds(&mut self) -> Result<(), Box<dyn Error>> {
        let key = fs::read(self.home.join(ENCRYPTION_KEY))?;
        let cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::open(self.home.join(ENCRYPTED_FILE))?;
        let decrypted_creds = String::from_utf8(cocoon.parse(&mut encrypted_file)?)?;

        self.trakt_credentials = serde_json::from_str(&decrypted_creds)?;
        // println!("{:#?}", self.trakt_credentials);
        Ok(())
    }

    pub fn save_creds(&self) -> Result<(), Box<dyn Error>> {
        let key = fs::read(self.home.join(ENCRYPTION_KEY))?;
        let mut cocoon = Cocoon::new(&key);

        let mut encrypted_file = File::create(self.home.join(ENCRYPTED_FILE))?;
        let dump_json = serde_json::to_string(&self.trakt_credentials)?;

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

    pub fn set_trakt_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        created_at: i32,
        expires_in: i32,
    ) {
        self.trakt_credentials.access_token = access_token.into_boxed_str();
        self.trakt_credentials.refresh_token = refresh_token.into_boxed_str();
        self.trakt_credentials.expires_on = created_at + expires_in;
    }

    pub fn has_tokens(&self) -> bool {
        self.trakt_credentials.access_token != "".into()
            && self.trakt_credentials.refresh_token != "".into()
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

    pub fn tokens_expiration_data(&self) -> i32 {
        self.trakt_credentials.expires_on
    }
}
