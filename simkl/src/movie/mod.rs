use chrono::{DateTime, TimeDelta, Utc};
use itertools::Itertools;
use reqwest::{
    blocking::ClientBuilder,
    header::{AUTHORIZATION, HeaderMap, USER_AGENT},
};
use serde::Serialize;
use serde_json::{Value, json};

mod smo;

use smo::MovieDetails;

use crate::smo::{Item, WatchlistBucket};

pub fn search_movies(
    client_id: &str,
    app_name: &str,
    app_version: &str,
    name: &str,
) -> anyhow::Result<Vec<Item>> {
    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version);
    let query = query!(app_name, app_version, ("q", name), ("extended", "full"));

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/search/movie"),
        Some(&headers),
        None,
        Some(&query),
        "Simkl: Error while getting movie details",
    )
}

pub fn get_movie_details(
    client_id: &str,
    app_name: &str,
    app_version: &str,
    simkl_id: u32,
) -> anyhow::Result<MovieDetails> {
    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version);
    let query = query!(app_name, app_version);

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/movies/{simkl_id}"),
        Some(&headers),
        None,
        Some(&query),
        "Simkl: Error while getting movie details",
    )
}

pub fn get_user_watchlist(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
) -> anyhow::Result<WatchlistBucket> {
    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version, ("extended", "full"), ("memos", "no"));

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/all-items/movies/plantowatch"),
        Some(&headers),
        None,
        Some(&query),
        "Simkl: Error while getting movie details",
    )
}

pub fn add_movies_to_watchlist(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
    ids_added_at: &[(u32, DateTime<Utc>)],
) -> anyhow::Result<Value> {
    #[derive(Serialize)]
    struct Id {
        tmdb: u32,
    }
    #[derive(Serialize)]
    struct WatchlistItem {
        ids:      Id,
        added_at: DateTime<Utc>,
        to:       String,
    }

    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version);
    let body = json!({
        "movies": ids_added_at.into_iter().map(|&(id, added_at)| WatchlistItem {
            ids: Id { tmdb: id },
            added_at,
            to: "plantowatch".to_string()
        }).collect_vec(),
    });

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/add-to-list"),
        Some(&headers),
        Some(&body),
        Some(&query),
        "Simkl: Error while adding movies to watchlist",
    )
}
pub fn remove_movies_history_or_from_watchlist(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
    ids: &[u32],
) -> anyhow::Result<Value> {
    #[derive(Serialize)]
    struct Id {
        tmdb: u32,
    }
    #[derive(Serialize)]
    struct WatchlistItem {
        ids: Id,
    }

    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version);
    let body = json!({
        "movies": ids.into_iter().map(|&id| WatchlistItem {
            ids: Id { tmdb: id },
        }).collect_vec(),
    });

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/history/remove"),
        Some(&headers),
        Some(&body),
        Some(&query),
        "Simkl: Error while removing movies history/from watchlist",
    )
}

pub fn log_watched(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
    items: &[(u32, usize, DateTime<Utc>)],
) -> anyhow::Result<Value> {
    #[derive(Serialize)]
    struct Id {
        tmdb: u32,
    }
    #[derive(Serialize)]
    struct WatchlistItem {
        ids:        Id,
        watched_at: DateTime<Utc>,
        status:     String,
        rating:     usize,
    }

    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version);
    let body = json!({
        "movies": items.into_iter().map(|&(id, rating, watched_at)| WatchlistItem {
            ids: Id { tmdb: id },
            watched_at: watched_at.to_utc() + TimeDelta::seconds(1),
            status: "completed".to_string(),
            rating,
        }).collect_vec(),
    });

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/history"),
        Some(&headers),
        Some(&body),
        Some(&query),
        "Simkl: Error while logging movies watch times",
    )
}

pub fn edit_watched(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
    items: &[(u32, usize, DateTime<Utc>)],
) -> anyhow::Result<Value> {
    _ = remove_movies_history_or_from_watchlist(access_token, client_id, app_name, app_version, &items.iter().map(|&(x, _, _)| x).collect_vec())?;

    #[derive(Serialize)]
    struct Id {
        tmdb: u32,
    }
    #[derive(Serialize)]
    struct WatchlistItem {
        ids:        Id,
        watched_at: DateTime<Utc>,
        status:     String,
        rating:     usize,
    }

    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version);
    let body = json!({
        "movies": items.into_iter().map(|&(id, rating, watched_at)| WatchlistItem {
            ids: Id { tmdb: id },
            watched_at: watched_at.to_utc() + TimeDelta::seconds(1),
            status: "completed".to_string(),
            rating,
        }).collect_vec(),
    });

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/history"),
        Some(&headers),
        Some(&body),
        Some(&query),
        "Simkl: Error while editing movies watch times",
    )
}

pub fn edit_rating(
    access_token: &str,
    client_id: &str,
    app_name: &str,
    app_version: &str,
    items: &[(u32, usize)],
) -> anyhow::Result<Value> {
    #[derive(Serialize)]
    struct Id {
        tmdb: u32,
    }
    #[derive(Serialize)]
    struct WatchlistItem {
        ids:    Id,
        rating: usize,
    }

    let client = ClientBuilder::new().build()?;
    let headers = headers!(client_id, app_name, app_version, access_token: access_token);
    let query = query!(app_name, app_version);
    let body = json!({
        "movies": items.into_iter().map(|&(id, rating)| WatchlistItem {
            ids: Id { tmdb: id },
            rating,
        }).collect_vec(),
    });

    crate::send_request_deserialized(
        &client,
        &format!("https://api.simkl.com/sync/history"),
        Some(&headers),
        Some(&body),
        Some(&query),
        "Simkl: Error while editing movies ratings",
    )
}
