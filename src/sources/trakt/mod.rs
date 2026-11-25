use std::time::Duration;

use anyhow::{Context, anyhow};
use itertools::Itertools;
use log::info;
use reqwest::{
    Method,
    blocking::{Client, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::Value;
use smo::RequestResponseError;

pub mod list;
pub mod movie;
pub mod smo;
pub mod tokens;

pub(crate) fn send_trakt_request(
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

fn send_request_deserialized<T: for<'a> Deserialize<'a>>(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    error_context: impl ToString,
) -> anyhow::Result<T> {
    let method = if body.is_none() { Method::GET } else { Method::POST };
    let response = send_trakt_request(client, url, headers, body, query, method)?;
    if !response.status().is_success() {
        return Err(match response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(err) => anyhow!(format!("{:?}", err)),
        })
        .context(error_context.to_string());
    }

    response.json().map_err(Into::into)
}

fn fetch_paginated_source<T: for<'a> Deserialize<'a>>(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    short_circuit_with_error: Option<&str>,
) -> anyhow::Result<Vec<(u32, Vec<T>)>> {
    let mut results = Vec::with_capacity(256);
    let mut page = 1;
    let mut total_pages = 0;
    loop {
        let page_string = page.to_string();
        let pagination_query = [("page", page_string.as_str()), ("limit", "250")].into_iter();
        let query = query
            .as_ref()
            .map(|x| x.to_vec())
            .unwrap_or_default()
            .into_iter()
            .chain(pagination_query)
            .collect_vec();
        let res = send_trakt_request(client, url, headers, body, Some(&query), Method::GET);
        match res {
            Ok(response) => {
                if total_pages == 0 {
                    if let Some(num_pages) = response.headers().iter().find_map(|(name, value)| {
                        if name.as_str().to_lowercase() == "x-pagination-page-count" {
                            Some(value.to_str().unwrap().parse::<u32>().ok()).flatten()
                        } else {
                            None
                        }
                    }) {
                        total_pages = num_pages;
                    } else {
                        break;
                    }
                }
                if !response.status().is_success() {
                    if response.status().as_u16() == 429 {
                        let retry_after = response
                            .headers()
                            .iter()
                            .find_map(|(name, value)| {
                                if name.as_str().to_lowercase() == "retry-after" {
                                    Some(value.to_str().unwrap().parse::<u64>().ok()).flatten()
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(5);

                        std::thread::sleep(Duration::from_secs(retry_after));
                        continue;
                    } else if let Some(error_context) = short_circuit_with_error {
                        return Err(match response.json::<RequestResponseError>() {
                            Ok(err) => err.into(),
                            Err(_) => anyhow!("Unknown error."),
                        })
                        .context(error_context.to_string());
                    }
                } else {
                    results.push((page, response.json()?));
                }
            }
            Err(err) =>
                if let Some(error_context) = short_circuit_with_error {
                    return Err(err).context(error_context.to_string());
                },
        }

        if page >= total_pages || page >= 15 {
            break;
        }

        page += 1;
    }
    info!("{page}/{total_pages}");

    Ok(results)
}

// pub(crate) fn download_image(client: Client, url: &str, path: PathBuf) -> anyhow::Result<()> {
//     let image_url = url.strip_suffix(".webp").unwrap_or(url);

//     let image_bytes = client
//         .get(format!("https://{image_url}"))
//         .send()?
//         .bytes()?
//         .into_iter()
//         .collect_vec();

//     if let Ok(img) = image::load_from_memory(&image_bytes) {
//         img.save(path)?;
//     }

//     Ok(())
// }
