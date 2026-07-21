use std::{path::PathBuf, thread};

use anyhow::bail;
use itertools::Itertools;
use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

use crate::{
    send_trakt_request,
    smo::{TraktDetailsResponse, TraktSearchResponse, TraktSearchResponseMovie},
};

pub(crate) mod smo;

pub fn find_movie(client_id: &str, name: &str) -> anyhow::Result<Vec<TraktSearchResponseMovie>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    // let escaped_name = url_escape::encode_fragment(name).to_string();
    let query = [("query", name)];
    let search_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/search/movie",
        &headers,
        None,
        Some(&query),
    )?;
    if search_response.status().as_u16() != 200 {
        bail!(format!("Trakt: Error while searching for movie: {}", name));
    }

    let json = search_response.json::<Vec<TraktSearchResponse>>()?;
    Ok(json.into_iter().map(|x| x.movie).collect_vec())
}

pub fn get_movie_details(client_id: &str, imdb_id: &str) -> anyhow::Result<TraktDetailsResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let query = [("type", "movie"), ("extended", "full,images")];

    let details_response = send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/movies/{imdb_id}"),
        &headers,
        None,
        Some(&query),
    )?;

    if details_response.status().as_u16() != 200 {
        bail!("couldn't get movie details with trakt");
    }

    Ok(details_response.json()?)
}

pub fn get_movie_poster_banner(
    cache_dir: &PathBuf,
    client_id: &str,
    imdb_id: &str,
) -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());
    let client = Client::builder().default_headers(headers).build()?;

    let movie_details = get_movie_details(client_id, &imdb_id)?;
    let download_image = move |client: Client, url: &str, path: PathBuf| -> anyhow::Result<()> {
        let image_url = url.strip_suffix(".webp").unwrap_or(url);

        let image_bytes: Vec<_> = client
            .get(format!("https://{image_url}"))
            .send()?
            .bytes()?
            .into_iter()
            .collect();

        if let Ok(img) = image::load_from_memory(&image_bytes) {
            img.save(path)?;
        }

        Ok(())
    };

    let path = cache_dir
        .join("posters")
        .join(format!("{}.jpg", movie_details.ids.tmdb));
    let poster_handle = {
        let client = client.clone();

        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_details.images.poster.is_empty() {
                let image_url = movie_details.images.poster[0].as_str();
                download_image(client, image_url, path)?;
            }

            Ok(())
        })
    };

    let path = cache_dir
        .join("backdrops")
        .join(format!("{}.jpg", movie_details.ids.tmdb));
    let backdrop_handle = {
        let client = client.clone();

        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_details.images.fanart.is_empty() {
                let image_url = movie_details.images.fanart[0].as_str();
                download_image(client, image_url, path)?;
            } else if !movie_details.images.banner.is_empty() {
                let image_url = movie_details.images.banner[0].as_str();
                download_image(client, image_url, path)?;
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}
