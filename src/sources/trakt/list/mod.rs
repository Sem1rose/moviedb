use itertools::Itertools;
use reqwest::{
    Method,
    blocking::{ClientBuilder, Response},
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, USER_AGENT},
};
use serde_json::json;

use crate::sources::trakt::{
    fetch_paginated_source,
    list::smo::{ListDetails, ListPrivacy},
    send_request_deserialized, send_trakt_request,
    smo::ListItem,
};

pub mod smo;

pub fn create_list(
    client_id: &str,
    access_token: &str,
    list_name: &str,
    list_description: &str,
    list_privacy: ListPrivacy,
    list_ranked: bool,
) -> anyhow::Result<ListDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let body = json!({
        "name": list_name,
        "description": list_description,
        "privacy": list_privacy,
        "display_numbers": list_ranked,
        "allow_comments": true,
    });

    send_request_deserialized(
        &client,
        "https://api.trakt.tv/users/me/lists",
        &headers,
        Some(&body),
        None,
        format!("Trakt: Unable to create list: {list_name}"),
    )
}

pub fn delete_list(client_id: &str, access_token: &str, list_id: u64) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/users/me/lists/{list_id}"),
        &headers,
        None,
        None,
        Method::DELETE,
    )
}

pub fn get_user_lists(client_id: &str, access_token: &str) -> anyhow::Result<Vec<ListDetails>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    fetch_paginated_source(
        &client,
        "https://api.trakt.tv/users/me/lists",
        &headers,
        None,
        None,
        None,
        // "PunchPlay: Error while getting user lists",
    )
    .map(|x| x.into_iter().map(|y| y.1).flatten().collect_vec())
}

pub fn get_list_details(
    client_id: &str,
    access_token: &str,
    list_id: u64,
) -> anyhow::Result<ListDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let mut list_details: ListDetails = send_request_deserialized(
        &client,
        &format!("https://api.trakt.tv/users/me/lists/{list_id}"),
        &headers,
        None,
        None,
        "Trakt: Error while getting list details",
    )?;
    list_details.items = get_list_items(client_id, access_token, list_id)?;

    Ok(list_details)
}

pub fn get_list_items(
    client_id: &str,
    access_token: &str,
    list_id: u64,
) -> anyhow::Result<Vec<ListItem>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let query = [("extended", "full")];
    fetch_paginated_source(
        &client,
        &format!("https://api.trakt.tv/users/me/lists/{list_id}/items/movie"),
        &headers,
        None,
        // None,
        Some(&query),
        None,
    )
    .map(|x| x.into_iter().map(|y| y.1).flatten().collect_vec())
}

pub fn add_movies_to_list(
    client_id: &str,
    access_token: &str,
    list_id: u64,
    items: Vec<u64>,
) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let body = json!({
        "movies": items.into_iter().map(|x| json!({
            "ids": {
                "tmdb": x
            }
        })).collect_vec()
    });

    send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/users/me/lists/{list_id}/items"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn remove_movies_from_list(
    client_id: &str,
    access_token: &str,
    list_id: u64,
    items: Vec<u64>,
) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {access_token}").parse().unwrap(),
    );
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let body = json!({
        "movies": items.into_iter().map(|x| json!({
            "ids": {
                "tmdb": x
            }
        })).collect_vec()
    });

    send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/users/me/lists/{list_id}/items/remove"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}
