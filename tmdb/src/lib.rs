use std::{collections::HashMap, path::PathBuf};

use itertools::Itertools;
use reqwest::{
    blocking::{Client, RequestBuilder, Response},
    header::HeaderMap,
};

pub mod movie;
pub mod smo;
pub mod tokens;

pub(crate) fn send_tmdb_request(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<HashMap<&str, &str>>,
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

pub(crate) fn download_image(client: Client, url: &str, path: PathBuf) -> anyhow::Result<()> {
    let image_bytes = client.get(url).send()?.bytes()?.into_iter().collect_vec();

    if let Ok(img) = image::load_from_memory(&image_bytes) {
        img.save(path)?;
    }

    Ok(())
}
