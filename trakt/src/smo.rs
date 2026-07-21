use std::{error::Error, fmt::Display};

use serde::Deserialize;

pub use crate::movie::smo::*;

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token:  String,
    // token_type: String,
    pub expires_in:    i64,
    pub refresh_token: String,
    // scope: String,
    pub created_at:    i64,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponseError {
    error:             String,
    error_description: String,
}
impl Error for TokenResponseError {}
impl Display for TokenResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.error_description)
    }
}
