use std::collections::HashMap;

use reqwest::{
    blocking::{Client, RequestBuilder, Response},
    header::HeaderMap,
};

pub mod movie;
pub mod smo;
pub mod tokens;

pub(crate) fn send_punch_play_request(
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
