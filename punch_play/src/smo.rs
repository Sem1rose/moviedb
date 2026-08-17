use std::{error::Error, fmt::Display};

use serde::Deserialize;

pub use crate::movie::smo::*;

#[derive(Deserialize, Debug)]
pub struct RequestResponseError {
    pub error:   Option<String>,
    pub message: Option<String>,
}
impl Error for RequestResponseError {}
impl Display for RequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}|{}",
            self.message.as_ref().unwrap_or(&"no message".into()),
            self.error.as_ref().unwrap_or(&"no error".into())
        )
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct RequestDeviceCodeResponse {
    // user_code: String,
    pub device_code:               String,
    // verification_uri: String,
    pub verification_uri_complete: String,
    // verification_uri_qr: String,
    // expires_in: usize,
    // scope: String
}

#[derive(Deserialize, Debug)]
pub struct AccessTokenResponse {
    pub access_token:  String,
    pub refresh_token: String,
    // pub token_type: String,
    pub expires_in:    i64,
    // pub refresh_expires_in: usize,
    // pub scope: String
}
