use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use simple_encrypt::{decrypt_bytes, encrypt_bytes};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserTokens {
    pub client_id:     String,
    pub client_secret: String,

    pub access_token:  String,
    pub refresh_token: String,
    pub expires_on:    i64,
}

impl UserTokens {
    pub fn has_secrets(&self) -> bool {
        !(self.client_id.is_empty() || self.client_secret.is_empty())
    }

    pub fn has_tokens(&self) -> bool {
        !(self.access_token.is_empty() || self.refresh_token.is_empty())
    }
}

#[derive(Clone, Default)]
pub struct TraktTokens {
    user_tokens: UserTokens,

    home_dir: PathBuf,
}

impl TraktTokens {
    pub fn new(home_dir: &PathBuf) -> Self {
        Self {
            home_dir: home_dir.clone(),

            user_tokens: UserTokens::default(),
        }
    }

    pub fn init(home_dir: &PathBuf) -> anyhow::Result<UserTokens> {
        let tokens_file_exists = home_dir.join(".trakt_tokens").is_file();

        if tokens_file_exists {
            Self::read_creds(home_dir)
        } else {
            bail!("Trakt: User tokens file does not exist.")
        }
    }

    fn read_creds(home_dir: &PathBuf) -> anyhow::Result<UserTokens> {
        let encrypted_data =
            fs::read(&home_dir.join(".trakt_tokens")).context("Trakt: unable to read tokens")?;

        serde_json::from_str(
            &String::from_utf8(
                decrypt_bytes(&encrypted_data, b"0123456789abcdef0123456789abcdef")
                    .context("Trakt: error decrypting user tokens")?,
            )
            .context("Trakt: error decoding utf8")?,
        )
        .context("Trakt: error parsing user tokens")
    }

    pub fn set_creds(&mut self, user_tokens: UserTokens) -> anyhow::Result<()> {
        self.user_tokens = user_tokens;

        self.save_creds()
    }

    pub fn save_creds(&self) -> anyhow::Result<()> {
        let data = serde_json::to_string(&self.user_tokens)?;

        fs::write(
            &self.home_dir.join(".trakt_tokens"),
            &encrypt_bytes(data.as_bytes(), b"0123456789abcdef0123456789abcdef")
                .context("Trakt: failed to encrypt user tokens")?,
        )
        .context("Trakt: failed to write encrypted file")
    }

    pub fn client_id(&self) -> &str {
        &self.user_tokens.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.user_tokens.client_secret
    }

    pub fn client_id_owned(&self) -> String {
        self.user_tokens.client_id.clone()
    }

    pub fn client_secret_owned(&self) -> String {
        self.user_tokens.client_secret.clone()
    }

    pub fn access_token(&self) -> &str {
        &self.user_tokens.access_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.user_tokens.refresh_token
    }

    pub fn expires_on(&self) -> i64 {
        self.user_tokens.expires_on
    }

    pub fn access_token_owned(&self) -> String {
        self.user_tokens.access_token.clone()
    }

    pub fn refresh_token_owned(&self) -> String {
        self.user_tokens.refresh_token.clone()
    }
}
