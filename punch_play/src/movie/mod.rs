use std::{path::{Path, PathBuf}, thread};

use anyhow::{Context, anyhow};
use itertools::Itertools;
use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

use crate::{
    send_punch_play_request,
    smo::{
        PunchPlayDetailsResponse, PunchPlaySearchResponse, PunchPlaySearchResult,
        RequestResponseError,
    },
};

pub(crate) mod smo;

pub fn find_movie(name: &str) -> anyhow::Result<Vec<PunchPlaySearchResult>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let query = [("q", name), ("type", "movie")];
    let search_response = send_punch_play_request(
        &client,
        "https://punchplay.tv/api/public/v1/catalog/search",
        &headers,
        None,
        Some(&query),
    )?;
    if !search_response.status().is_success() {
        return Err(match search_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context(format!(
            "PunchPlay: Error while searching for movie {}",
            name
        ));
    }

    let json = search_response.json::<PunchPlaySearchResponse>()?;
    Ok(json.items)
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

    let details_response = send_punch_play_request(
        &client,
        &format!("https://punchplay.tv/api/platform/v1/title/movie/{tmdb_id}"),
        &headers,
        None,
        None,
    )?;
    if !details_response.status().is_success() {
        return Err(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("PunchPlay: Error while getting movie details");
    }

    details_response.json().map_err(Into::into)
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
    let download_image = move |client: Client, url: &str, path: PathBuf| -> anyhow::Result<()> {
        let image_bytes = client.get(url).send()?.bytes()?.into_iter().collect_vec();

        if let Ok(img) = image::load_from_memory(&image_bytes) {
            img.save(path)?;
        }

        Ok(())
    };

    let path = cache_dir
        .join("posters")
        .join(format!("{}.jpg", movie_details.tmdb_id));
    let poster_handle = {
        let client = client.clone();

        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_details.poster_url.is_empty() {
                download_image(client, movie_details.poster_url.as_str(), path)?;
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
                download_image(client, &movie_details.backdrop_url.as_str(), path)?;
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}
