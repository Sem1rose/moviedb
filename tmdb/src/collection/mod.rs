pub mod smo;
use std::path::Path;

use reqwest::{blocking::ClientBuilder, header::HeaderMap};

use crate::collection::smo::CollectionDetails;

pub fn get_collection_details(access_token: &str, id: u32) -> anyhow::Result<CollectionDetails> {
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
        &format!("https://api.themoviedb.org/3/collection/{id}"),
        &headers,
        None,
        None,
        "TMDB: Error while getting collection details",
    )
}

pub fn get_collection_artwork(
    cache_dir: &Path,
    access_token: &str,
    id: u32,
) -> anyhow::Result<bool> {
    let collection_details = get_collection_details(access_token, id)?;

    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    if let Some(profile_path) = collection_details.poster_path {
        let path = cache_dir.join("collections").join(format!("{}.jpg", id));
        crate::download_image(
            client,
            &format!("https://image.tmdb.org/t/p/{}/{}", "w500", profile_path),
            path,
        )?;

        Ok(true)
    } else {
        Ok(false)
    }
}
