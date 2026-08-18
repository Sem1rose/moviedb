use std::{path::Path, thread};

use anyhow::anyhow;
use itertools::Itertools;
use reqwest::{
    Method,
    blocking::{Client, ClientBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::smo::{Credits, MovieDetails, MovieImagesResponse, SearchResult, UserInteraction};

pub(crate) mod smo;

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    // page:          u32,
    results:       Vec<T>,
    total_pages:   u32,
    total_results: u32,
}

pub fn get_user_watchlist(
    access_token: &str,
    account_id: u32,
) -> anyhow::Result<Vec<SearchResult>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let first_page = crate::send_request_deserialized::<PaginatedResponse<SearchResult>>(
        &client,
        &format!("https://api.themoviedb.org/3/account/{account_id}/watchlist/movies"),
        &headers,
        None,
        None,
        "TMDB: Error while getting user watchlist",
    )?;
    if first_page.total_pages > 1 {
        thread::scope(|s| -> anyhow::Result<Vec<SearchResult>> {
            let mut items = Vec::with_capacity(first_page.total_results as usize);
            items.extend(first_page.results);

            let client_ref = &client;
            let headers_ref = &headers;
            let results = (2..=first_page.total_pages)
                .map(|i| {
                    s.spawn(move || {
                        crate::send_request_deserialized::<PaginatedResponse<SearchResult>>(
                            client_ref,
                            &format!(
                                "https://api.themoviedb.org/3/account/{account_id}/watchlist/movies"
                            ),
                            headers_ref,
                            None,
                            Some(&[("page", i.to_string().as_str())]),
                            "",
                        )
                        .map(|x| x.results)
                    })
                })
                .map(|x| {
                    x.join()
                        .map_err(|_| anyhow!("Error joining thread."))
                        .flatten()
                })
                .collect_vec();

            if results.iter().any(|x| x.is_err()) {
                results.into_iter().find(|x| x.is_err()).unwrap()
            } else {
                items.extend(results.into_iter().map(|x| x.unwrap()).flatten());

                Ok(items)
            }
        })
    } else {
        Ok(first_page.results)
    }
}
pub fn find_movie(access_token: &str, name: &str) -> anyhow::Result<Vec<SearchResult>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );
    let query = [("query", name)];

    crate::send_request_deserialized::<PaginatedResponse<_>>(
        &client,
        "https://api.themoviedb.org/3/search/movie",
        &headers,
        None,
        Some(&query),
        &format!("TMDB: Error while searching for movie {}", name),
    )
    .map(|x| x.results)
}

pub fn get_movie_user_interaction(
    client: &Client,
    headers: &HeaderMap,
    movie_id: u32,
) -> anyhow::Result<UserInteraction> {
    crate::send_request_deserialized::<Value>(
        client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/account_states"),
        headers,
        None,
        None,
        "TMDB: Error while getting movie credits",
    )
    .map(|x| UserInteraction {
        favorite:  x["favorite"].as_bool().unwrap(),
        watchlist: x["watchlist"].as_bool().unwrap(),
        rating:    x["rated"]
            .as_object()
            .map(|y| y["value"].as_number().unwrap().as_u64())
            .flatten(),
    })
}

pub fn get_movie_credits(
    client: &Client,
    headers: &HeaderMap,
    movie_id: u32,
) -> anyhow::Result<Credits> {
    crate::send_request_deserialized(
        client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/credits"),
        headers,
        None,
        None,
        "TMDB: Error while getting movie credits",
    )
}

pub fn get_movie_certification(
    client: &Client,
    headers: &HeaderMap,
    movie_id: u32,
) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct ReleaseDate {
        certification: String,
        // descriptors: [],
        // iso_639_1:     String,
        // note: String,
        // release_date:  String,
        #[serde(alias = "type")]
        release_type:  usize,
    }
    #[derive(Deserialize)]
    struct ReleaseDatesResult {
        iso_3166_1:    String,
        release_dates: Vec<ReleaseDate>,
    }
    #[derive(Deserialize)]
    struct ReleaseDatesResponse {
        results: Vec<ReleaseDatesResult>,
    }

    crate::send_request_deserialized::<ReleaseDatesResponse>(
        client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/release_dates"),
        headers,
        None,
        None,
        "TMDB: Error while getting certification",
    )
    .map_err(Into::into)
    .map(|x| {
        x.results
            .into_iter()
            .filter(|y| y.iso_3166_1 == "US")
            .nth(0)
            .map(|y| {
                y.release_dates
                    .into_iter()
                    .filter(|z| z.release_type == 3)
                    .nth(0)
                    .map(|z| z.certification)
            })
            .flatten()
            .ok_or(anyhow!(""))
    })
    .flatten()
}

pub fn get_movie_recommendations(
    client: &Client,
    headers: &HeaderMap,
    movie_id: u32,
) -> anyhow::Result<Vec<u32>> {
    #[derive(Deserialize)]
    struct Recommendation {
        id: u32,
    }

    crate::send_request_deserialized::<PaginatedResponse<Recommendation>>(
        client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/recommendations"),
        headers,
        None,
        None,
        "TMDB: Error while getting movie recommendations",
    )
    .map_err(Into::into)
    .map(|x| x.results.into_iter().map(|y| y.id).take(5).collect())
}

pub fn get_movie_details(access_token: &str, movie_id: u32) -> anyhow::Result<MovieDetails> {
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
        &format!(
            "https://api.themoviedb.org/3/movie/{movie_id}?append_to_response=account_states,credits,release_dates,recommendations,images"
        ),
        &headers,
        None,
        None,
        "TMDB: Error while getting movie details",
    )
}

pub fn get_rated_movies(access_token: &str, account_id: u32) -> anyhow::Result<Vec<SearchResult>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let first_page = crate::send_request_deserialized::<PaginatedResponse<SearchResult>>(
        &client,
        &format!("https://api.themoviedb.org/3/account/{account_id}/rated/movies"),
        &headers,
        None,
        None,
        "TMDB: Error while getting rated movies",
    )?;
    if first_page.total_pages <= 1 {
        Ok(first_page.results)
    } else {
        thread::scope(|s| -> anyhow::Result<Vec<SearchResult>> {
            let mut items = Vec::with_capacity(first_page.total_results as usize);
            items.extend(first_page.results);

            let client_ref = &client;
            let headers_ref = &headers;
            let results = (2..=first_page.total_pages)
                .map(|i| {
                    s.spawn(move || {
                        crate::send_request_deserialized::<PaginatedResponse<SearchResult>>(
                            client_ref,
                            &format!(
                                "https://api.themoviedb.org/3/account/{account_id}/rated/movies"
                            ),
                            headers_ref,
                            None,
                            Some(&[("page", i.to_string().as_str())]),
                            "TMDB: Error while getting rated movies",
                        )
                        .map(|x| x.results)
                    })
                })
                .map(|x| {
                    x.join()
                        .map_err(|_| anyhow!("Error joining thread."))
                        .flatten()
                })
                .collect_vec();

            if results.iter().any(|x| x.is_err()) {
                results.into_iter().find(|x| x.is_err()).unwrap()
            } else {
                items.extend(results.into_iter().map(|x| x.unwrap()).flatten());

                Ok(items)
            }
        })
    }
}

pub(crate) fn get_movie_images(
    access_token: &str,
    movie_id: u32,
) -> anyhow::Result<MovieImagesResponse> {
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
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/images"),
        &headers,
        None,
        None,
        "",
    )
}

pub fn get_movie_artworks(
    cache_dir: &Path,
    access_token: &str,
    tmdb_details: Option<&MovieDetails>,
    movie_id: u32,
) -> anyhow::Result<bool> {
    let mut movie_images: MovieImagesResponse = tmdb_details.map(Into::into).unwrap_or_else(|| {
        get_movie_details(access_token, movie_id)
            .as_ref()
            .map(Into::into)
            .unwrap_or_default()
    });
    if movie_images.backdrops.is_empty() || movie_images.posters.is_empty() {
        if let Ok(images) = get_movie_images(access_token, movie_id) {
            if movie_images.backdrops.is_empty() && !images.backdrops.is_empty() {
                movie_images.backdrops = images
                    .backdrops
                    .into_iter()
                    .sorted_by(|a, b| {
                        b.vote_average
                            .partial_cmp(&a.vote_average)
                            .map(|x| -> Option<std::cmp::Ordering> {
                                matches!(x, std::cmp::Ordering::Equal)
                                    .then_some(b.vote_count.cmp(&a.vote_count))
                                    .or_else(|| Some(x))
                            })
                            .flatten()
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .collect();
            }
            if movie_images.posters.is_empty() && !images.posters.is_empty() {
                movie_images.posters = images
                    .posters
                    .into_iter()
                    .sorted_by(|a, b| {
                        b.vote_average
                            .partial_cmp(&a.vote_average)
                            .map(|x| {
                                matches!(x, std::cmp::Ordering::Equal)
                                    .then_some(b.vote_count.cmp(&a.vote_count))
                                    .or_else(|| Some(x))
                            })
                            .flatten()
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .collect();
            }
        }
    }

    let try_get_artwork = |id: usize,
                           backdrop: bool,
                           path: &Path,
                           movie_images: &MovieImagesResponse|
     -> anyhow::Result<u8> {
        if id
            >= if backdrop {
                movie_images.backdrops.len()
            } else {
                movie_images.posters.len()
            }
        {
            return Ok(2);
        }

        let image_bytes = reqwest::blocking::get(format!(
            "https://image.tmdb.org/t/p/{}/{}",
            if backdrop { "w780" } else { "w500" },
            if backdrop {
                movie_images.backdrops[id].file_path.clone()
            } else {
                movie_images.posters[id].file_path.clone()
            }
        ))?
        .bytes()?
        .into_iter()
        .collect_vec();

        if let Ok(img) = image::load_from_memory(&image_bytes) {
            img.save(path)?;
        } else {
            return Ok(1);
        }

        Ok(0)
    };

    let mut status = false;
    let poster_path = cache_dir.join("posters").join(format!("{}.jpg", movie_id));
    let poster_handle = {
        let movie_images = movie_images.clone();

        thread::spawn(move || -> anyhow::Result<bool> {
            if !movie_images.posters.is_empty() {
                for i in 0..5 {
                    let result = try_get_artwork(i, false, &poster_path, &movie_images)?;
                    match result {
                        0 => return Ok(true),
                        2 => return Ok(false),
                        _ => (),
                    }
                }
            }
            Ok(false)
        })
    };

    let backdrop_path = cache_dir
        .join("backdrops")
        .join(format!("{}.jpg", movie_id));
    let backdrop_handle = {
        thread::spawn(move || -> anyhow::Result<bool> {
            if !movie_images.backdrops.is_empty() {
                for i in 0..5 {
                    let result = try_get_artwork(i, true, &backdrop_path, &movie_images)?;
                    match result {
                        0 => return Ok(true),
                        2 => return Ok(false),
                        _ => (),
                    }
                }
            }
            Ok(false)
        })
    };

    status |= poster_handle.join().unwrap()?;
    status |= backdrop_handle.join().unwrap()?;

    Ok(status)
}

pub fn get_person_artwork(cache_dir: &Path, access_token: &str, id: u32) -> anyhow::Result<bool> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    #[derive(Deserialize)]
    struct PersonDetails {
        profile_path: Option<String>,
    }

    let profile_path = crate::send_request_deserialized::<PersonDetails>(
        &client,
        &format!("https://api.themoviedb.org/3/person/{id}"),
        &headers,
        None,
        None,
        "TMDB: Error while while querying for person details",
    )?
    .profile_path;

    if let Some(profile_path) = profile_path {
        let path = cache_dir.join("persons").join(format!("{}.jpg", id));
        crate::download_image(
            client,
            &format!("https://image.tmdb.org/t/p/{}/{}", "w342", profile_path),
            path,
        )?;

        return Ok(true);
    }

    Ok(false)
}

pub fn add_or_edit_rating(
    access_token: &str,
    movie_id: u32,
    rating: usize,
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
        "value": rating
    });

    crate::send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/rating"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
    .map_err(Into::into)
}

pub fn delete_rating(access_token: &str, movie_id: u32) -> anyhow::Result<Response> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    crate::send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{movie_id}/rating"),
        &headers,
        None,
        None,
        Method::DELETE,
    )
}

pub fn add_or_remove_watchlist(
    access_token: &str,
    account_id: u32,
    movie_id: u32,
    watchlist: bool,
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
        "media_type": "movie",
        "media_id": movie_id,
        "watchlist": watchlist
    });

    crate::send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/account/{account_id}/watchlist"),
        &headers,
        Some(&body),
        None,
        Method::POST,
    )
}
