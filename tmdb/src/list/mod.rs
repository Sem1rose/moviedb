use reqwest::{
    Method,
    blocking::{ClientBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::json;

use crate::list::smo::ListDetails;

pub mod smo;

pub fn get_user_lists(access_token: &str, account_id: u32) -> anyhow::Result<Vec<ListDetails>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    #[derive(Deserialize)]
    struct UserListsResponse {
        results: Vec<ListDetails>,
    }

    crate::send_request_deserialized::<UserListsResponse>(
        &client,
        &format!("https://api.themoviedb.org/3/account/{account_id}/lists"),
        &headers,
        None,
        None,
        "TMDB: Error while getting user lists",
    )
    .map(|x| x.results)
}

pub fn get_list_details(access_token: &str, id: u32) -> anyhow::Result<ListDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    crate::send_request_deserialized(
        &client,
        &format!("https://api.themoviedb.org/3/list/{id}"),
        &headers,
        None,
        None,
        "TMDB: Error while getting list details",
    )
}

pub fn add_item_to_list(
    access_token: &str,
    list_id: u32,
    item_id: u32,
) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let body = json!({
        "media_id": item_id
    });

    crate::send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/list/{list_id}/add_item"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn remove_item_from_list(
    access_token: &str,
    list_id: u32,
    item_id: u32,
) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let body = json!({
        "media_id": item_id
    });

    crate::send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/list/{list_id}/remove_item"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}
