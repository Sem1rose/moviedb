use std::path::PathBuf;

use anyhow::{Context, anyhow};
use itertools::Itertools;
use reqwest::{
    Method,
    blocking::{Client, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::Value;

use crate::smo::RequestResponseError;

pub mod collection;
pub mod list;
pub mod movie;
pub mod smo;
pub mod tokens;

fn send_tmdb_request(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    method: Method,
) -> anyhow::Result<Response> {
    let mut request = client.request(method, url).headers(headers.clone());
    if query.is_some() {
        request = request.query(&query.unwrap());
    }
    if body.is_some() {
        request = request.json(&body.clone().unwrap());
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
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    error_context: impl ToString,
) -> anyhow::Result<T> {
    let method = if body.is_none() { Method::GET } else { Method::POST };
    let response = send_tmdb_request(client, url, headers, body, query, method)?;
    if !response.status().is_success() {
        return Err(match response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context(error_context.to_string());
    }

    response.json().map_err(Into::into)
}
