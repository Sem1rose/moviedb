use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, anyhow};
use itertools::Itertools;
use reqwest::{
    blocking::{Client, RequestBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;

use crate::smo::RequestResponseError;

pub mod list;
pub mod movie;
pub mod smo;
pub mod tokens;

fn send_punch_play_request(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<&HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> anyhow::Result<Response> {
    let mut request: RequestBuilder;
    if body.is_none() {
        request = client.get(url).headers(headers.clone());
        if query.is_some() {
            request = request.query(&query.unwrap());
        }
    } else {
        request = client
            .post(url)
            .headers(headers.clone())
            .json(&body.clone().unwrap());
    }

    let response = request.send()?;
    Ok(response)
}

fn download_image(client: Client, url: &str, path: PathBuf) -> anyhow::Result<()> {
    let image_bytes = client.get(url).send()?.bytes()?.into_iter().collect_vec();

    if let Ok(img) = image::load_from_memory(&image_bytes) {
        img.save(path)?;
    }

    Ok(())
}

fn send_request_deserialized<T: for<'a> Deserialize<'a>>(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<&HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
    error_context: impl ToString,
) -> anyhow::Result<T> {
    let response = send_punch_play_request(client, url, headers, body, query)?;
    if !response.status().is_success() {
        return Err(match response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context(error_context.to_string());
    }

    response.json().map_err(Into::into)
}
