use reqwest::{blocking::ClientBuilder, header::HeaderMap};
use serde::Deserialize;
use serde_json::Value;

use crate::list::smo::ListDetails;

pub mod smo;

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
