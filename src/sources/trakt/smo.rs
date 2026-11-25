use std::{error::Error, fmt::Display};

use serde::Deserialize;

pub use super::movie::smo::*;

#[derive(Deserialize, Debug)]
pub struct DeviceCodesResponse {
    pub device_code:      String,
    pub user_code:        String,
    pub verification_url: String,
    // expires_in: i64,
    pub interval:         u64,
}
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
pub struct RequestResponseError {
    error:             String,
    #[serde(default)]
    error_description: Option<String>,
}
impl Error for RequestResponseError {}
impl Display for RequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.error,
            self.error_description.clone().unwrap_or_default()
        )
    }
}
