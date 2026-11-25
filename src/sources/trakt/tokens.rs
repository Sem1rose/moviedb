use std::sync::mpsc::Sender;

use anyhow::bail;
use reqwest::{
    Method,
    blocking::ClientBuilder,
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};
use serde_json::json;

use super::{send_trakt_request, smo::TokenResponse};
use crate::sources::trakt::{send_request_deserialized, smo::DeviceCodesResponse};

// https://developer.trakt.tv/docs/authentication-oauth
pub fn get_tokens(
    client_id: &str,
    tx_auth_url_code: Sender<(String, String)>,
) -> anyhow::Result<TokenResponse> {
    let client = reqwest::blocking::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let mut body = json!({
        "client_id": client_id
    });

    let device_code_response: DeviceCodesResponse = send_request_deserialized(
        &client,
        "https://auth.trakt.tv/oauth/device/code",
        &headers,
        Some(&body),
        None,
        "Trakt: Error while generating device codes",
    )?;
    tx_auth_url_code.send((
        device_code_response.verification_url,
        device_code_response.user_code,
    ))?;

    body = json!({
        "code": &device_code_response.device_code,
        "client_id": client_id
    });

    let mut request_token_response = send_trakt_request(
        &client,
        "https://auth.trakt.tv/oauth/device/token",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )?;
    let mut retries = 0;
    while !request_token_response.status().is_success() {
        if ![400, 429].contains(&request_token_response.status().as_u16()) {
            bail!(
                "Trakt: couldn't authenticate request token, {:?}",
                request_token_response.status()
            );
        }

        retries += 1;
        if retries > 50 {
            bail!("Trakt: couldn't authenticate request token, max retries reached");
        }

        std::thread::sleep(std::time::Duration::from_secs(
            device_code_response.interval,
        ));
        request_token_response = send_trakt_request(
            &client,
            "https://auth.trakt.tv/oauth/device/token",
            &headers,
            Some(&body),
            None,
            Method::POST,
        )?;
    }

    Ok(request_token_response.json()?)
}

pub fn refresh_tokens(client_id: &str, refresh_token: &str) -> anyhow::Result<TokenResponse> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let body = json!({
        "refresh_token": refresh_token,
        "client_id": client_id,
        "grant_type": "refresh_token"
    });

    send_request_deserialized(
        &client,
        "https://api.trakt.tv/oauth/token",
        &headers,
        Some(&body),
        None,
        "Trakt: Error while refreshing tokens",
    )
}
