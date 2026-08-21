use std::{thread, time::Duration};

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

use crate::smo::ResponseError;

#[macro_use]
pub mod smo;
pub mod list;
pub mod movie;
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
    let headers = headers!(client_id, app_name, app_version);
    let query = query!(
        app_name,
        app_version,
        ("to", "simkl"),
        (id_source, id),
        ("type", id_type)
    );

    crate::send_simkl_request(
        &client,
        &format!("https://api.simkl.com/redirect"),
        Some(&headers),
        None,
        Some(&query),
        Method::GET,
    )
    .map(|x| {
        x.headers()
            .into_iter()
            .find_map(|(name, value)| {
                if name.as_str() == "location" {
                    Some(
                        value
                            .to_str()
                            .unwrap()
                            .strip_prefix("//simkl.com/")
                            .unwrap()
                            .split('/')
                            .dropping(1)
                            .take(2)
                            .collect_tuple()
                            .map(|(id, slug)| {
                                (
                                    id.parse().unwrap(),
                                    slug.split('?').nth(0).unwrap().to_string(),
                                )
                            }),
                    )
                } else {
                    None
                }
                .flatten()
            })
            .unwrap()
    })
}

fn send_simkl_request(
    client: &Client,
    url: &str,
    headers: Option<&HeaderMap>,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    method: Method,
) -> anyhow::Result<Response> {
    let mut retries = 0;
    loop {
        let mut request = client.request(method.clone(), url);
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
        if !response.status().is_success() {
            if retries < 2 && [429, 500, 502, 503].contains(&response.status().as_u16()) {
                thread::sleep(Duration::from_secs(2u64.pow(retries)));
                retries += 1;

                continue;
            }

            break Ok(response);
        }

        break Ok(response);
    }
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
        return Err(match response.json::<ResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!("Unknown error."),
        })
        .context(error_context.to_string());
    }

    response.json().map_err(Into::into)
}

fn _fetch_paginated_source(
    client: &Client,
    url: &str,
    headers: Option<&HeaderMap>,
    body: Option<&Value>,
    query: Option<&[(&str, &str)]>,
    short_circuit_with_error: Option<&str>,
) -> anyhow::Result<Vec<(u32, Response)>> {
    let mut results = vec![];
    let mut page = 1;
    let mut total_pages = 0;
    loop {
        let page_string = format!("{page}");
        let pagination_query = [("page", page_string.as_str()), ("limit", "50")].into_iter();
        let query = query
            .as_ref()
            .map(|x| x.to_vec())
            .unwrap_or_default()
            .into_iter()
            .chain(pagination_query)
            .collect_vec();
        let res = send_simkl_request(client, url, headers, body, Some(&query), Method::GET);
        match res {
            Ok(response) => {
                if total_pages == 0 {
                    total_pages = response
                        .headers()
                        .iter()
                        .find_map(|(name, value)| {
                            if name.as_str() == "X-Pagination-Page-Count" {
                                Some(value.to_str().unwrap().parse::<u32>().ok()).flatten()
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                }
                if !response.status().is_success() {
                    if let Some(error_context) = short_circuit_with_error {
                        return Err(match response.json::<ResponseError>() {
                            Ok(err) => err.into(),
                            Err(_) => anyhow!("Unknown error."),
                        })
                        .context(error_context.to_string());
                    }
                } else {
                    results.push((page, response));
                }
            }
            Err(err) =>
                if let Some(error_context) = short_circuit_with_error {
                    return Err(err).context(error_context.to_string());
                },
        }

        if total_pages > 0 && page >= total_pages || page >= 20 {
            break;
        }
        page += 1;
    }

    Ok(results)
}
