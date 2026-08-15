use std::{path::Path, thread};

use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

use crate::smo::{PunchPlayDetailsResponse, PunchPlaySearchResponse, PunchPlaySearchResult};

pub mod smo;

pub fn find_movie(name: &str) -> anyhow::Result<Vec<PunchPlaySearchResult>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let query = [("q", name), ("type", "movie")];
    crate::send_request_deserialized::<PunchPlaySearchResponse>(
        &client,
        "https://punchplay.tv/api/public/v1/catalog/search",
        &headers,
        None,
        Some(&query),
        format!("PunchPlay: Error while searching for movie {}", name),
    )
    .map(|x| x.items)
}

pub fn get_movie_details(
    access_token: &str,
    tmdb_id: u32,
) -> anyhow::Result<PunchPlayDetailsResponse> {
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
        &format!("https://punchplay.tv/api/platform/v1/title/movie/{tmdb_id}"),
        &headers,
        None,
        None,
        "PunchPlay: Error while getting movie details",
    )
}

pub fn get_movie_poster_banner(
    cache_dir: &Path,
    access_token: &str,
    tmdb_id: u32,
) -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    let client = Client::builder().build()?;

    let movie_details = get_movie_details(access_token, tmdb_id)?.title;

    let path = cache_dir
        .join("posters")
        .join(format!("{}.jpg", movie_details.tmdb_id));
    let poster_handle = {
        let client = client.clone();

        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_details.poster_url.is_empty() {
                crate::download_image(client, movie_details.poster_url.as_str(), path)?;
            }

            Ok(())
        })
    };

    let path = cache_dir
        .join("backdrops")
        .join(format!("{}.jpg", movie_details.tmdb_id));
    let backdrop_handle = {
        let client = client.clone();

        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_details.backdrop_url.is_empty() {
                crate::download_image(client, &movie_details.backdrop_url.as_str(), path)?;
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}
