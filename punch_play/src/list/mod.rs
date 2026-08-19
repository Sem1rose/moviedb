use anyhow::anyhow;
use reqwest::{
    Method,
    blocking::{ClientBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::list::smo::ListDetails;

pub mod smo;

pub fn test(access_token: &str, id: u32) -> anyhow::Result<Value> {
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
        &format!("https://punchplay.tv/api/platform/v1/lists/{id}"),
        &headers,
        None,
        None,
        "PunchPlay: Error while getting list details",
    )
}
pub fn get_user_lists(access_token: &str) -> anyhow::Result<Vec<ListDetails>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    #[derive(Deserialize)]
    // #[serde(rename_all = "camelCase")]
    struct UserListsResponse {
        items: Vec<ListDetails>, // next_cursor: Option<String>,
    }

    crate::send_request_deserialized::<UserListsResponse>(
        &client,
        &format!("https://punchplay.tv/api/platform/v1/me/lists"),
        &headers,
        None,
        None,
        "PunchPlay: Error while getting user lists",
    )
    .map(|x| x.items)
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
        &format!("https://punchplay.tv/api/platform/v1/lists/{id}"),
        &headers,
        None,
        None,
        "PunchPlay: Error while getting list details",
    )
}

pub fn add_item_to_list(
    access_token: &str,
    list_id: u32,
    item_type: &str,
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
        "kind": item_type,
        "sourceId": item_id,
        "title": ""
    });
    
    crate::send_punch_play_request(
        &client,
        &format!("https://punchplay.tv/api/platform/v1/lists/{list_id}/items"),
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
    
    crate::send_punch_play_request(
        &client,
        &format!("https://punchplay.tv/api/platform/v1/lists/{list_id}/items/{item_id}"),
        &headers,
        None,
        None,
        Method::DELETE,
    )
}

pub fn remove_id_from_list(
    access_token: &str,
    list_id: u32,
    tmdb_id: u32,
) -> anyhow::Result<Response> {
    let item_id = get_list_details(access_token, list_id)?
    .items
    .unwrap_or_default()
    .into_iter()
    .find(|x| x.tmdb_id == tmdb_id)
    .map(|x| x.id)
    .ok_or(anyhow!("PunchPlay: Item probably not in this list"))?;

    remove_item_from_list(access_token, list_id, item_id)
}

pub fn delete_list(access_token: &str, id: u32) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    crate::send_punch_play_request(
        &client,
        &format!("https://punchplay.tv/api/platform/v1/lists/{id}"),
        &headers,
        None,
        None,
        Method::DELETE,
    )
}