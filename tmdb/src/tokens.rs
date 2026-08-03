use std::{collections::HashMap, sync::mpsc::Sender};

use anyhow::{Context, anyhow, bail};
use reqwest::{blocking::ClientBuilder, header::HeaderMap};

use crate::{
    send_tmdb_request,
    smo::{RequestResponseError, RequestSessionIDResponse, RequestTokenResponse},
};

// https://developer.themoviedb.org/docs/authentication-user
pub fn get_session_id(
    access_token: &str,
    tx_authorization_url: Sender<String>,
) -> anyhow::Result<String> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    // Step 1: create a request token
    let request_token_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/token/new",
        &headers,
        None,
        None,
    )?;

    if !request_token_response.status().is_success() {
        return Err(
            match request_token_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            },
        )
        .context("TMDB: Error while getting a request token");
    }

    let request_token = request_token_response
        .json::<RequestTokenResponse>()?
        .request_token;

    // Step 2: ask the user for permission
    let authorization_url = format!("https://www.themoviedb.org/authenticate/{}", request_token);
    _ = tx_authorization_url.send(authorization_url.clone());

    // Step 3: wait for user permission
    let mut request_token_response = send_tmdb_request(
        &client,
        &format!(
            "https://www.themoviedb.org/authenticate/{}/allow",
            request_token
        ),
        &headers,
        None,
        None,
    )?;
    let mut retries = 0;
    while !request_token_response.status().is_success() {
        retries += 1;
        if retries > 50 {
            bail!("TMDB: couldn't authenticate request token, max retries reached");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
        request_token_response = send_tmdb_request(
            &client,
            &format!(
                "https://www.themoviedb.org/authenticate/{}/allow",
                request_token
            ),
            &headers,
            None,
            None,
        )?;
    }
    drop(tx_authorization_url);

    // The request token has been approved by the user
    // Step 4: finally create a new session ID
    let mut body = HashMap::new();
    body.insert("request_token", request_token.as_str());
    let create_session_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/session/new",
        &headers,
        Some(body),
        None,
    )?;

    if !create_session_response.status().is_success() {
        return Err(
            match create_session_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            },
        )
        .context("TMDB: Error while creating a new session ID");
    }

    let session_id = create_session_response
        .json::<RequestSessionIDResponse>()?
        .session_id;
    Ok(session_id)
}
