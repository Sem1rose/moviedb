use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use simple_encrypt::{decrypt_bytes, encrypt_bytes};

#[derive(Clone, Default)]
pub struct OMDBTokens {
    key: String,

    pub status: bool,
    home_dir:   PathBuf,
}

impl OMDBTokens {
    pub fn new(home_dir: &PathBuf) -> Self {
        Self {
            home_dir: home_dir.clone(),

            ..Default::default()
        }
    }

    pub fn init(home_dir: &PathBuf) -> anyhow::Result<String> {
        if home_dir.join(".omdb_tokens").is_file() {
            Self::read_creds(home_dir)
        } else {
            bail!("OMDB: User tokens file does not exist.")
        }
    }

    pub fn read_creds(home_dir: &PathBuf) -> anyhow::Result<String> {
        let encrypted_data =
            fs::read(&home_dir.join(".omdb_tokens")).context("OMDB: unable to read tokens")?;

        String::from_utf8(
            decrypt_bytes(&encrypted_data, b"0123456789abcdef0123456789abcdef")
                .context("OMDB: error decrypting user tokens")?,
        )
        .context("OMDB: error decoding utf8")
    }

    pub fn set_creds(&mut self, key: String) -> anyhow::Result<()> {
        self.key = key;
        self.status = true;

        self.save_creds()
    }

    fn save_creds(&self) -> anyhow::Result<()> {
        fs::write(
            &self.home_dir.join(".omdb_tokens"),
            &encrypt_bytes(self.key.as_bytes(), b"0123456789abcdef0123456789abcdef")
                .context("OMDB: failed to encrypt user tokens")?,
        )
        .context("OMDB: failed to write encrypted file")
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn key_owned(&self) -> String {
        self.key.clone()
    }
}
