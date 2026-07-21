use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use anyhow::{Context, bail};
use reqwest::{
    blocking::ClientBuilder,
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

use crate::{
    send_trakt_request,
    smo::{TokenResponse, TokenResponseError},
};

// https://trakt.docs.apiary.io/#reference/authentication-oauth/authorize/authorize-application
pub fn get_tokens(
    client_id: &str,
    client_secret: &str,
    tx_auth_url: Sender<String>,
    rx_auth_code: Receiver<String>,
) -> anyhow::Result<TokenResponse> {
    let client = reqwest::blocking::Client::new();

    // Step 1: ask the user for an authorization code
    let authorization_url = client
        .get("https://trakt.tv/oauth/authorize")
        .query(&[
            ("client_id", client_id),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
            ("response_type", "code"),
        ])
        .header(CONTENT_TYPE, "application/json")
        .build()?
        .url()
        .to_string();

    // Step 1.5: Validate the client id
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let validate_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/genres/movies",
        &headers,
        None,
        None,
    )?;
    if validate_response.status().as_u16() >= 400 {
        bail!("Trakt: Unable to validate user credentials");
    }

    _ = tx_auth_url.send(authorization_url);

    let auth_code = rx_auth_code
        .recv_timeout(std::time::Duration::from_secs(120))
        .unwrap_or_default();
    if auth_code.is_empty() {
        bail!("Trakt: no auth code received");
    }

    // Step 2: exchange authorization code for access token
    let mut body = HashMap::new();
    body.insert("code", auth_code.as_str());
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "authorization_code");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        &headers,
        Some(body),
        None,
    )?;

    if token_response.status().as_u16() >= 400 {
        return Err::<_, anyhow::Error>(match token_response.json::<TokenResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("Trakt: Error while exchanging auth code for an access token");
    }

    Ok(token_response.json::<TokenResponse>()?)
}

pub fn refresh_tokens(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> anyhow::Result<TokenResponse> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let mut body = HashMap::new();
    body.insert("refresh_token", refresh_token);
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "refresh_token");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        &headers,
        Some(body),
        None,
    )?;

    if token_response.status().as_u16() >= 400 {
        return Err::<_, anyhow::Error>(match token_response.json::<TokenResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("Trakt: Error while while refreshing access token");
    }

    Ok(token_response.json::<TokenResponse>()?)
}
