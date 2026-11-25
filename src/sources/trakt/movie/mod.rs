use anyhow::{Context, anyhow};
use chrono::{DateTime, TimeDelta, Utc};
use itertools::Itertools;
use reqwest::{
    Method,
    blocking::{ClientBuilder, Response},
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, USER_AGENT},
};
use serde::Deserialize;
use serde_json::json;

use super::smo::MovieDetails;
use crate::sources::trakt::{
    fetch_paginated_source, send_request_deserialized, send_trakt_request,
    smo::{HistoryItem, ListItem, RequestResponseError},
};

pub(super) mod smo;

fn get_trakt_id_from_tmdb(client_id: &str, tmdb_id: u64) -> anyhow::Result<u64> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    #[derive(Deserialize)]
    struct SearchItem {
        // score: String,
        // type: String,
        pub movie: MovieDetails,
    }

    let query = [("extended", "full"), ("type", "movie"), ("limit", "1")];
    send_request_deserialized::<Vec<SearchItem>>(
        &client,
        &format!("https://api.trakt.tv/search/tmdb/{tmdb_id}"),
        &headers,
        None,
        Some(&query),
        format!("Trakt: Error getting ids for: {}", tmdb_id),
    )
    .map(|x| {
        x.first()
            .map(|y| y.movie.ids.trakt)
            .ok_or_else(|| anyhow!("Trakt: Error getting ids for: {}", tmdb_id))
    })
    .flatten()
}

pub fn find_movie(client_id: &str, name: &str) -> anyhow::Result<Vec<MovieDetails>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    #[derive(Deserialize)]
    struct SearchResponse {
        // score: String,
        // type: String,
        pub movie: MovieDetails,
    }

    let query = [("query", name)];
    send_request_deserialized::<Vec<SearchResponse>>(
        &client,
        "https://api.trakt.tv/search/movie",
        &headers,
        None,
        Some(&query),
        format!("Trakt: Error while searching for movie: {}", name),
    )
    .map(|x| x.into_iter().map(|y| y.movie).collect_vec())
}

pub fn get_movie_details(client_id: &str, imdb_id: &str) -> anyhow::Result<MovieDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let query = [("type", "movie"), ("extended", "full,images")];

    send_request_deserialized(
        &client,
        &format!("https://api.trakt.tv/movies/{imdb_id}"),
        &headers,
        None,
        Some(&query),
        format!("Trakt: Error getting details for: {}", imdb_id),
    )
}

pub fn get_user_ratings(client_id: &str, access_token: &str) -> anyhow::Result<Vec<ListItem>> {
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
        "https://api.trakt.tv/sync/ratings/movies",
        &headers,
        None,
        Some(&query),
        None,
    )
    .map(|x| x.into_iter().map(|y| y.1).flatten().collect_vec())
}

pub fn get_watch_history(client_id: &str, access_token: &str) -> anyhow::Result<Vec<HistoryItem>> {
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
        "https://api.trakt.tv/sync/history/movies",
        &headers,
        None,
        Some(&query),
        None,
    )
    .map(|x| x.into_iter().map(|y| y.1).flatten().collect_vec())
}

pub fn get_movie_watches(
    client_id: &str,
    access_token: &str,
    tmdb_id: u64,
) -> anyhow::Result<Vec<HistoryItem>> {
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

    let trakt_id = get_trakt_id_from_tmdb(client_id, tmdb_id)?;

    let query = [("extended", "full")];
    send_request_deserialized(
        &client,
        &format!("https://api.trakt.tv/sync/history/movies/{trakt_id}"),
        &headers,
        None,
        Some(&query),
        format!("Trakt: Error getting history for: {}", tmdb_id),
    )
}

pub fn log_watches(
    client_id: &str,
    access_token: &str,
    items: Vec<(u64, Option<usize>, DateTime<Utc>)>,
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
        "movies": items.iter().map(|x| json!({
            "ids": {
                "tmdb": x.0
            },
            "watched_at": if x.2 == DateTime::<Utc>::default() {"unknown".to_string()} else { x.2.to_rfc3339()}
        })).collect_vec()
    });
    let history_result = send_trakt_request(
        &client,
        "https://api.trakt.tv/sync/history",
        &headers,
        Some(&body),
        None,
        Method::POST,
    );
    if history_result.is_err()
        || history_result
            .as_ref()
            .map(|x| !x.status().is_success())
            .unwrap()
    {
        return history_result;
    }

    let body = json!({
        "movies": &items
            .into_iter()
            .filter(|x| x.1.is_some())
            .chunk_by(|x| x.0)
            .into_iter()
            .map(|x| x.1.sorted_by_key(|y| y.2).last().unwrap())
            .map(|x| json!({
                "ids": {
                    "tmdb": x.0
                },
                "rating": x.1,
                "rated_at": x.2.to_rfc3339(),
            })).collect_vec()
    });
    send_trakt_request(
        &client,
        "https://api.trakt.tv/sync/ratings",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn modify_rating(
    client_id: &str,
    access_token: &str,
    tmdb_id: u64,
    rating: usize,
    rated_at: DateTime<Utc>,
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
        "movies": [
            {
                "ids": {
                    "tmdb": tmdb_id
                },
                "rating": rating,
                "rated_at": rated_at.to_rfc3339(),
            }
        ]
    });

    send_trakt_request(
        &client,
        "https://api.trakt.tv/sync/ratings",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn delete_movie_watch(
    client_id: &str,
    access_token: &str,
    tmdb_id: u64,
    watch_time: DateTime<Utc>,
    // ) -> anyhow::Result<Option<Response>> {
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

    let watches_to_delete = get_movie_watches(client_id, access_token, tmdb_id)?
        .into_iter()
        .filter_map(|x| {
            if (x.watched_at - watch_time).abs() < TimeDelta::hours(1) {
                Some(x.id)
            } else {
                None
            }
        })
        .collect_vec();
    // if watches_to_delete.is_empty() {
    //     return Ok(None);
    // }

    let body = json!({
        "ids": watches_to_delete
    });

    send_trakt_request(
        &client,
        "https://api.trakt.tv/sync/history/remove",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
    // .map(Option::Some)
}

pub fn modify_watch_date(
    client_id: &str,
    access_token: &str,
    tmdb_id: u64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    // ) -> anyhow::Result<Option<Response>> {
) -> anyhow::Result<Response> {
    let response = delete_movie_watch(client_id, access_token, tmdb_id, from)?;
    // if let Some(response) = response {
    if !response.status().is_success() {
        return Err(match response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(err) => anyhow!(format!("{:?}", err)),
        })
        .context(format!("Trakt: Unable to modify movie {tmdb_id} watch"));
    }
    log_watches(client_id, access_token, vec![(tmdb_id, None, to)])
    // .map(Option::Some)
    // } else {
    // Ok(None)
    // }
}

pub fn remove_movies_from_watched(
    client_id: &str,
    access_token: &str,
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
        "https://api.trakt.tv/sync/history/remove",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn get_user_watchlist(client_id: &str, access_token: &str) -> anyhow::Result<Vec<ListItem>> {
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
        "https://api.trakt.tv/sync/watchlist/movies",
        &headers,
        None,
        Some(&query),
        None,
    )
    .map(|x| x.into_iter().map(|y| y.1).flatten().collect_vec())
}

pub fn add_movies_to_watchlist(
    client_id: &str,
    access_token: &str,
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
        "https://api.trakt.tv/sync/watchlist",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

pub fn remove_movies_from_watchlist(
    client_id: &str,
    access_token: &str,
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
        "https://api.trakt.tv/sync/watchlist/remove",
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}

// pub fn get_movie_artworks(cache_dir: &Path, client_id: &str, imdb_id: &str) -> anyhow::Result<()> {
//     let mut headers = HeaderMap::new();
//     headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
//     headers.insert(USER_AGENT, "moviedb/0.1.0".parse().unwrap());
//     headers.insert("trakt-api-version", "2".parse().unwrap());
//     headers.insert("trakt-api-key", client_id.parse().unwrap());

//     let client = Client::builder().default_headers(headers).build()?;

//     let movie_details = get_movie_details(client_id, &imdb_id)?;

//     let path = cache_dir
//         .join("posters")
//         .join(format!("{}.jpg", movie_details.ids.tmdb));
//     let poster_handle = {
//         let client = client.clone();

//         thread::spawn(move || -> anyhow::Result<()> {
//             if !movie_details.images.poster.is_empty() {
//                 let image_url = movie_details.images.poster[0].as_str();
//                 download_image(client, image_url, path)?;
//             }

//             Ok(())
//         })
//     };

//     let path = cache_dir
//         .join("backdrops")
//         .join(format!("{}.jpg", movie_details.ids.tmdb));
//     let backdrop_handle = {
//         let client = client.clone();

//         thread::spawn(move || -> anyhow::Result<()> {
//             if !movie_details.images.fanart.is_empty() {
//                 let image_url = movie_details.images.fanart[0].as_str();
//                 download_image(client, image_url, path)?;
//             } else if !movie_details.images.banner.is_empty() {
//                 let image_url = movie_details.images.banner[0].as_str();
//                 download_image(client, image_url, path)?;
//             }

//             Ok(())
//         })
//     };

//     poster_handle.join().unwrap()?;
//     backdrop_handle.join().unwrap()?;

//     Ok(())
// }
