use std::{collections::HashMap, sync::mpsc::Sender};

use anyhow::{Context, anyhow};
use reqwest::{blocking::ClientBuilder, header::HeaderMap};

use crate::{
    send_punch_play_request,
    smo::{AccessTokenResponse, RequestDeviceCodeResponse, RequestResponseError},
};

// https://docs.punchplay.tv/quickstart
pub fn get_tokens(
    client_id: &str,
    client_secret: &str,
    tx_authorization_url: Sender<String>,
) -> anyhow::Result<AccessTokenResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let mut body = HashMap::new();
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);

    // Step 1: request a device code
    let device_code_response = send_punch_play_request(
        &client,
        "https://punchplay.tv/api/platform/v1/auth/device/code",
        &headers,
        Some(&body),
        None,
    )?;
    if !device_code_response.status().is_success() {
        return Err(match device_code_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("PunchPlay: Unable to validate user credentials");
    }
    let device_code_response = device_code_response.json::<RequestDeviceCodeResponse>()?;

    // Step 2: ask the user for permission
    _ = tx_authorization_url.send(device_code_response.verification_uri_complete.clone());

    // Step 3: wait for user permission
    body.insert("device_code", &device_code_response.device_code);
    body.insert("device_name", "moviedb");

    let mut token_response = send_punch_play_request(
        &client,
        "https://punchplay.tv/api/platform/v1/auth/device/token",
        &headers,
        Some(&body),
        None,
    )?;
    let mut retries = 0;
    while !token_response.status().is_success() {
        retries += 1;
        if retries > 50 {
            return Err(match token_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            })
            .context("PunchPlay: couldn't authenticate request token, max retries reached");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
        token_response = send_punch_play_request(
            &client,
            "https://punchplay.tv/api/platform/v1/auth/device/token",
            &headers,
            Some(&body),
            None,
        )?;
    }
    drop(tx_authorization_url);
    // The request token has been approved by the user

    token_response.json().map_err(Into::into)
}

pub fn refresh_tokens(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> anyhow::Result<AccessTokenResponse> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        "application/x-www-form-urlencoded".parse().unwrap(),
    );

    let mut body = HashMap::new();
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);
    body.insert("refresh_token", refresh_token);

    let token_response = send_punch_play_request(
        &client,
        "https://punchplay.tv/api/platform/v1/auth/refresh",
        &headers,
        Some(&body),
        None,
    )?;

    if !token_response.status().is_success() {
        return Err(match token_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("PunchPlay: Error while while refreshing access token");
    }

    token_response.json().map_err(Into::into)
}
