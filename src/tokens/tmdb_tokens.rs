use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use simple_encrypt::{decrypt_bytes, encrypt_bytes};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserTokens {
    pub access_token: String,

    pub session_id: String,
}

impl UserTokens {
    pub fn has_access_token(&self) -> bool {
        !self.access_token.is_empty()
    }

    pub fn has_session_id(&self) -> bool {
        !self.session_id.is_empty()
    }
}

#[derive(Clone, Default)]
pub struct TMDBTokens {
    user_tokens: UserTokens,

    home_dir: PathBuf,
}

impl TMDBTokens {
    pub fn new(home_dir: &PathBuf) -> Self {
        Self {
            user_tokens: UserTokens::default(),

            home_dir: home_dir.clone(),
        }
    }

    pub fn init(home_dir: &PathBuf) -> anyhow::Result<UserTokens> {
        let tokens_file_exists = home_dir.join(".tmdb_tokens").is_file();

        if tokens_file_exists {
            Self::read_creds(home_dir)
        } else {
            bail!("TMDB: User tokens file does not exist.")
        }
    }

    fn read_creds(home_dir: &PathBuf) -> anyhow::Result<UserTokens> {
        let encrypted_data =
            fs::read(&home_dir.join(".tmdb_tokens")).context("TMDB: unable to read tokens")?;

        serde_json::from_str(
            &String::from_utf8(
                decrypt_bytes(&encrypted_data, b"0123456789abcdef0123456789abcdef")
                    .context("TMDB: error decrypting user tokens")?,
            )
            .context("TMDB: error decoding utf8")?,
        )
        .context("TMDB: error parsing user tokens")
    }

    pub fn set_creds(&mut self, user_tokens: UserTokens) -> anyhow::Result<()> {
        self.user_tokens = user_tokens;

        self.save_creds()
    }

    fn save_creds(&self) -> anyhow::Result<()> {
        let data = serde_json::to_string(&self.user_tokens)?;

        fs::write(
            &self.home_dir.join(".tmdb_tokens"),
            &encrypt_bytes(data.as_bytes(), b"0123456789abcdef0123456789abcdef")
                .context("TMDB: failed to encrypt user tokens")?,
        )
        .context("TMDB: failed to write encrypted file")
    }

    pub fn access_token(&self) -> &str {
        &self.user_tokens.access_token
    }

    pub fn access_token_owned(&self) -> String {
        self.user_tokens.access_token.clone()
    }

    pub fn session_id(&self) -> &str {
        &self.user_tokens.session_id
    }

    pub fn session_id_owned(&self) -> String {
        self.user_tokens.session_id.clone()
    }
}
