use std::{path::Path, thread};

use anyhow::{Context, anyhow};
use itertools::Itertools;
use reqwest::{
    blocking::{Client, ClientBuilder},
    header::HeaderMap,
};
use serde::Deserialize;
use serde_json::Value;

use crate::smo::{
    Credits, MovieDetails, MovieImagesResponse, RequestResponseError, SearchResult, UserInteraction,
};

pub(crate) mod smo;

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

    #[derive(Deserialize)]
    struct TMDBSearchResponse {
        // page: u64,
        results: Vec<SearchResult>,
        // total_pages: u64,
        // total_results: u64,
    }

    let results = crate::send_request_deserialized::<TMDBSearchResponse>(
        &client,
        "https://api.themoviedb.org/3/search/movie",
        &headers,
        None,
        Some(&query),
        &format!("TMDB: Error while searching for movie {}", name),
    )?
    .results;
    Ok(results)
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
    #[derive(Deserialize)]
    struct RecommendationsResponse {
        // page: u32,
        results: Vec<Recommendation>,
    }

    crate::send_request_deserialized::<RecommendationsResponse>(
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

pub fn get_movie_details(
    access_token: &str,
    movie_id: u32,
    extra_details: bool,
) -> anyhow::Result<MovieDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let client_ref = &client;
    let headers_ref = &headers;
    if extra_details {
        let (
            details_response,
            user_interaction_response,
            credits_response,
            certification_response,
            recommendations_response,
        ) = thread::scope(move |s| {
            let details_handle = s.spawn(move || {
                crate::send_tmdb_request(
                    client_ref,
                    &format!("https://api.themoviedb.org/3/movie/{movie_id}"),
                    headers_ref,
                    None,
                    None,
                )
            });
            let user_interaction_handle =
                s.spawn(move || get_movie_user_interaction(client_ref, headers_ref, movie_id));
            let credits_handle =
                s.spawn(move || get_movie_credits(client_ref, headers_ref, movie_id));
            let certification_handle =
                s.spawn(move || get_movie_certification(client_ref, headers_ref, movie_id));
            let recommendations_handle =
                s.spawn(move || get_movie_recommendations(client_ref, headers_ref, movie_id));

            let details_response = details_handle
                .join()
                .map_err(|x| anyhow!("TMDB: Error while joining the thread: {x:?}"))??;
            if !details_response.status().is_success() {
                return Err(match details_response.json::<RequestResponseError>() {
                    Ok(err) => err.into(),
                    Err(_) => anyhow!(""),
                })
                .context("TMDB: Error while getting movie details");
            }

            Ok((
                details_response,
                user_interaction_handle
                    .join()
                    .map(Result::ok)
                    .ok()
                    .flatten(),
                credits_handle.join().map(Result::ok).ok().flatten(),
                certification_handle.join().map(Result::ok).ok().flatten(),
                recommendations_handle.join().map(Result::ok).ok().flatten(),
            ))
        })?;

        details_response
            .json()
            .map(|mut x: MovieDetails| {
                x.user_interaction = user_interaction_response;
                x.credits = credits_response;
                x.certificate =
                    certification_response.or(Some(if x.adult { "N" } else { "NR" }.into()));
                x.recommendations = recommendations_response;

                if let Some(collection_id) = x.belongs_to_collection.as_ref().map(|x| x.id) {
                    x.collection_details =
                        crate::collection::get_collection_details(access_token, collection_id).ok();
                }

                x
            })
            .map_err(Into::into)
    } else {
        crate::send_request_deserialized(
            client_ref,
            &format!("https://api.themoviedb.org/3/movie/{movie_id}"),
            headers_ref,
            None,
            None,
            "TMDB: Error while getting movie details",
        )
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
        get_movie_details(access_token, movie_id, false)
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
