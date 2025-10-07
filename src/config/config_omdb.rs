use crate::config::{Config, Credentials};
use std::fs::read_to_string;

#[derive(Clone, Default)]
pub struct OMDBConfig {
    key: String,
}

impl OMDBConfig {
    pub fn init(&mut self) {
        let file_contents =
            read_to_string(".credentials").expect("Couldn't read credentials from .credentials!");
        let creds: Credentials = serde_json::from_str(&file_contents)
            .expect("Couldn't deserialize credentials at .credentials");

        self.set_key(creds.omdb_key);
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn key_owned(&self) -> String {
        self.key.clone()
    }

    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    pub fn has_key(&self) -> bool {
        self.key != *""
    }
}
