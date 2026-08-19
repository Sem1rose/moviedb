use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use simple_encrypt::{decrypt_bytes, encrypt_bytes};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserTokens {
    pub client_id:     String,
    pub client_secret: String,

    pub access_token: String,
    pub app_name:     String,
    pub app_version:  String,
}

impl UserTokens {
    pub fn has_secrets(&self) -> bool {
        !(self.client_id.is_empty() || self.client_secret.is_empty())
    }

    pub fn has_tokens(&self) -> bool {
        !self.access_token.is_empty()
    }
}

#[derive(Clone, Default, Debug)]
pub struct SimklTokens {
    user_tokens: UserTokens,

    pub status: Option<bool>,
    home_dir:   PathBuf,
}

#[allow(dead_code)]
impl SimklTokens {
    pub fn new(home_dir: &Path) -> Self {
        Self {
            home_dir: home_dir.to_path_buf(),

            status:      None,
            user_tokens: UserTokens::default(),
        }
    }

    pub fn init(home_dir: &Path) -> anyhow::Result<UserTokens> {
        if home_dir.join(".simkl_tokens").is_file() {
            Self::read_creds(home_dir)
        } else {
            bail!("Simkl: User tokens file does not exist.")
        }
    }

    pub fn read_creds(home_dir: &Path) -> anyhow::Result<UserTokens> {
        let encrypted_data =
            fs::read(home_dir.join(".simkl_tokens")).context("Simkl: unable to read tokens")?;

        serde_json::from_str(
            &String::from_utf8(
                decrypt_bytes(&encrypted_data, b"0123456789abcdef0123456789abcdef")
                    .context("Simkl: error decrypting user tokens")?,
            )
            .context("Simkl: error decoding utf8")?,
        )
        .context("Simkl: error parsing user tokens")
    }

    pub fn set_creds(&mut self, user_tokens: UserTokens) -> anyhow::Result<()> {
        self.user_tokens = user_tokens;

        self.status = if self.user_tokens.has_tokens() {
            Some(true)
        } else if self.user_tokens.has_secrets() {
            Some(false)
        } else {
            None
        };

        self.save_creds()
    }

    pub fn save_creds(&self) -> anyhow::Result<()> {
        let data = serde_json::to_string(&self.user_tokens)?;

        fs::write(
            self.home_dir.join(".simkl_tokens"),
            &encrypt_bytes(data.as_bytes(), b"0123456789abcdef0123456789abcdef")
                .context("Simkl: failed to encrypt user tokens")?,
        )
        .context("Simkl: failed to write encrypted file")
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

    pub fn app_name(&self) -> &str {
        &self.user_tokens.app_name
    }

    pub fn app_version(&self) -> &str {
        &self.user_tokens.app_version
    }

    pub fn access_token_owned(&self) -> String {
        self.user_tokens.access_token.clone()
    }

    pub fn app_name_owned(&self) -> String {
        self.user_tokens.app_name.clone()
    }

    pub fn app_version_owned(&self) -> String {
        self.user_tokens.app_version.clone()
    }
}
