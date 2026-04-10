use crate::{tokens::Credentials, types::*};
use anyhow::{bail, Context};
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

#[derive(Clone, Default)]
pub struct OMDBTokens {
    key: String,

    home_dir: PathBuf,
}

impl OMDBTokens {
    pub fn new(home_dir: &PathBuf) -> Self {
        Self {
            home_dir: home_dir.clone(),

            ..Default::default()
        }
    }

    pub fn init(&self) -> anyhow::Result<String> {
        let tokens_file_exists = self.home_dir.join(".omdb_tokens").is_file();
        if tokens_file_exists {
            self.read_creds()
        } else {
            bail!("OMDB: User tokens file does not exist.")
        }
    }

    fn read_creds(&self) -> anyhow::Result<String> {
        let cocoon = Cocoon::new(b"0123456789abcdef");
        let mut encrypted_file = File::open(&self.home_dir.join(".omdb_tokens"))?;

        String::from_utf8(
            cocoon
                .parse(&mut encrypted_file)
                .context("OMDB: error decrypting user tokens")?,
        )
        .context("OMDB: error decoding utf8")
    }

    pub fn set_creds(&mut self, key: String) -> anyhow::Result<()> {
        self.key = key;

        self.save_creds()
    }

    fn save_creds(&self) -> anyhow::Result<()> {
        let mut cocoon = Cocoon::new(b"0123456789abcdef");

        let mut encrypted_file = File::create(&self.home_dir.join(".omdb_tokens"))?;

        cocoon.dump(self.key.clone().into_bytes(), &mut encrypted_file)?;

        Ok(())
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
