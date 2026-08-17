use std::{path::Path, thread};

use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

use crate::{
    list::smo::ListItem,
    smo::{
        DetailsResponse, HistoryItem, ItemDetails, PaginatedResponse, RatingsResponse,
        SearchResponse,
    },
};

pub mod smo;

pub fn get_user_watchlist(access_token: &str) -> anyhow::Result<Vec<ListItem>> {
    if let Some(watchlist_id) = crate::list::get_user_lists(access_token)?
        .into_iter()
        .find(|x| x.is_watchlist)
        .map(|x| x.id)
    {
        crate::list::get_list_details(access_token, watchlist_id).map(|x| x.items.unwrap())
    } else {
        Ok(vec![])
    }
    // let client = ClientBuilder::new().build()?;
    // let mut headers = HeaderMap::new();
    // headers.insert("accept", "application/json".parse().unwrap());
    // headers.insert("content-type", "application/json".parse().unwrap());
    // headers.insert(
    //     "Authorization",
    //     format!("Bearer {}", access_token).parse().unwrap(),
    // );

    // crate::send_request_deserialized(
    //     &client,
    //     "https://punchplay.tv/api/platform/v1/me/watch-status",
    //     &headers,
    //     None,
    //     None,
    //     format!("PunchPlay: Error while getting user watchlist"),
    // )
}

pub fn find_movie(name: &str) -> anyhow::Result<Vec<ItemDetails>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let query = [("q", name), ("type", "movie")];
    crate::send_request_deserialized::<SearchResponse>(
        &client,
        "https://punchplay.tv/api/public/v1/catalog/search",
        &headers,
        None,
        Some(&query),
        format!("PunchPlay: Error while searching for movie {}", name),
    )
    .map(|x| x.items)
}

pub fn get_movie_details(access_token: &str, tmdb_id: u32) -> anyhow::Result<DetailsResponse> {
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

pub fn get_rated_movies(access_token: &str) -> anyhow::Result<Vec<HistoryItem>> {
    // pub fn get_rated_movies(access_token: &str) -> anyhow::Result<Vec<anyhow::Result<Vec<HistoryItem>>>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let first_page = crate::send_request_deserialized::<RatingsResponse>(
        &client,
        "https://punchplay.tv/api/platform/v1/me/ratings",
        &headers,
        None,
        None,
        "PunchPlay: Error while getting rated movies",
    )?;
    if !first_page.has_more {
        Ok(first_page.items)
        // Ok(vec![])
    } else {
        thread::scope(|s| {
            let mut items = Vec::with_capacity(first_page.total as usize);
            items.extend(first_page.items);
            // items.push(Ok(first_page.items));

            let client_ref = &client;
            let headers_ref = &headers;
            items.extend(
                (2..=first_page.total.div_ceil(first_page.page_size))
                    .map(|i| {
                        s.spawn(move || {
                            crate::send_request_deserialized::<RatingsResponse>(
                                client_ref,
                                "https://punchplay.tv/api/platform/v1/me/ratings",
                                headers_ref,
                                None,
                                Some(&[("page", i.to_string().as_str())]),
                                "PunchPlay: Error while getting rated movies",
                            )
                            .map(|x| x.items)
                        })
                    })
                    // .map(|x| x.join().map_err(|_| anyhow!("")).flatten())
                    .filter_map(|x| x.join().ok().map(|x| x.ok()).flatten())
                    .flatten(),
            );

            Ok(items)
        })
    }
}

pub fn get_watch_history(access_token: &str) -> anyhow::Result<Vec<HistoryItem>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let mut results = Vec::with_capacity(256);
    let mut response = crate::send_request_deserialized::<PaginatedResponse<HistoryItem>>(
        &client,
        "https://punchplay.tv/api/platform/v1/me/history",
        &headers,
        None,
        Some(&[("limit", "100")]),
        "PunchPlay: Error while getting watch history",
    )?;
    results.extend(response.items);
    while let Some(cursor) = response.next_cursor {
        let Ok(rspns) = crate::send_request_deserialized::<PaginatedResponse<HistoryItem>>(
            &client,
            "https://punchplay.tv/api/platform/v1/me/history",
            &headers,
            None,
            Some(&[("limit", "100"), ("cursor", &cursor)]),
            "PunchPlay: Error while getting watch history",
        ) else {
            break;
        };

        response = rspns;
        results.extend(response.items);
    }

    Ok(results)
}

pub fn get_ratings_stats(access_token: &str) -> anyhow::Result<RatingsResponse> {
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
        "https://punchplay.tv/api/platform/v1/me/ratings",
        &headers,
        None,
        None,
        "PunchPlay: Error while getting rated movies",
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
            if let Some(poster_url) = movie_details.poster_url {
                if !poster_url.is_empty() {
                    crate::download_image(client, poster_url.as_str(), path)?;
                }
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
            if let Some(backdrop_url) = movie_details.backdrop_url {
                if !backdrop_url.is_empty() {
                    crate::download_image(client, backdrop_url.as_str(), path)?;
                }
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}
