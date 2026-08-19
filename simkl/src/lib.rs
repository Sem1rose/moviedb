use std::path::PathBuf;

use anyhow::{Context, anyhow};
use itertools::Itertools;
use reqwest::{
    Method,
    blocking::{Client, ClientBuilder, Response},
    header::{HeaderMap, USER_AGENT},
    redirect::Policy,
};
use serde::Deserialize;
use serde_json::Value;

pub mod list;
pub mod movie;
pub mod smo;
pub mod tokens;

pub fn external_to_simkl_id_slug(
    client_id: &str,
    app_name: &str,
    app_version: &str,
    id: &str,
    id_source: &str, /* imdb,tmdb */
    id_type: &str,   /* movie,tv */
) -> anyhow::Result<(u32, String)> {
    let client = ClientBuilder::new().redirect(Policy::none()).build()?;

    let mut headers = HeaderMap::new();
    headers.insert("simkl-api-key", client_id.parse().unwrap());
    headers.insert(
        USER_AGENT,
        format!("{app_name}/{app_version}").parse().unwrap(),
    );

    crate::send_simkl_request(
        &client,
        &format!("https://api.simkl.com/redirect?to=simkl&{id_source}={id}&type={id_type}&app-name={app_name}&app-version={app_version}"),
        Some(&headers),
        None,
        None,
        Method::GET
    )
    .map(|x|
        x.headers().into_iter().find_map(|(name, value)|
            if name.as_str() == "location" {Some(value.to_str().unwrap().strip_prefix("//simkl.com/").unwrap().split('/').dropping(1).take(2).collect_tuple().map(|(id, slug)| (id.parse().unwrap(), slug.split('?').nth(0).unwrap().to_string())))} else {None}.flatten()
        ).unwrap()
    )
}

fn send_simkl_request(
    client: &Client,
    url: &str,
    headers: Option<&HeaderMap>,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    method: Method,
) -> anyhow::Result<Response> {
    let mut request = client.request(method, url);
    if headers.is_some() {
        request = request.headers(headers.cloned().unwrap());
    }
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
    headers: Option<&HeaderMap>,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    error_context: impl ToString,
) -> anyhow::Result<T> {
    let method = if body.is_none() { Method::GET } else { Method::POST };
    let response = send_simkl_request(client, url, headers, body, query, method)?;
    if !response.status().is_success() {
        return Err(match response.json::<Value>() {
            Ok(err) => anyhow!(err.to_string()),
            Err(_) => anyhow!(""),
        })
        .context(error_context.to_string());
    }

    response.json().map_err(Into::into)
}
