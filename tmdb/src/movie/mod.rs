use std::{path::PathBuf, thread};

use anyhow::{Context, anyhow};
use reqwest::{blocking::ClientBuilder, header::HeaderMap};
use serde::Deserialize;

use crate::{
    download_image, send_tmdb_request,
    smo::{
        ConfigurationResponse, ImagesConfiguration, RequestResponseError, TMDBCollectionDetails,
        TMDBCredits, TMDBMovieDetails, TMDBMovieImagesResponse, TMDBSearchResult,
    },
};

pub(crate) mod smo;

pub fn find_movie(access_token: &str, name: &str) -> anyhow::Result<Vec<TMDBSearchResult>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let query = [("query", name)];
    let search_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/search/movie",
        &headers,
        None,
        Some(&query),
    )?;
    if !search_response.status().is_success() {
        return Err(match search_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context(format!("TMDB: Error while searching for movie {}", name));
    }

    #[derive(Deserialize)]
    struct TMDBSearchResponse {
        // page: u64,
        results: Vec<TMDBSearchResult>,
        // total_pages: u64,
        // total_results: u64,
    }
    let json = search_response.json::<TMDBSearchResponse>()?;
    Ok(json.results)
}

pub fn get_movie_details(access_token: &str, tmdb_id: u32) -> anyhow::Result<TMDBMovieDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}"),
        &headers,
        None,
        None,
    )?;
    if !details_response.status().is_success() {
        return Err(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("TMDB: Error while getting movie details");
    }

    details_response
        .json()
        .map(|mut x: TMDBMovieDetails| {
            x.credits = get_movie_credits(access_token, tmdb_id).ok();
            x.certificate = get_movie_certification(access_token, tmdb_id)
                .or_else(|_| -> anyhow::Result<String> {
                    Ok(if x.adult { "N" } else { "NR" }.into())
                })
                .ok();
            x.recommendations = get_movie_recommendations(access_token, tmdb_id).ok();

            x
        })
        .map_err(Into::into)
}

pub fn get_movie_credits(access_token: &str, tmdb_id: u32) -> anyhow::Result<TMDBCredits> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/credits"),
        &headers,
        None,
        None,
    )?;
    if !details_response.status().is_success() {
        return Err(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("TMDB: Error while getting credits");
    }

    details_response.json().map_err(Into::into)
}

pub fn get_movie_certification(access_token: &str, tmdb_id: u32) -> anyhow::Result<String> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/release_dates"),
        &headers,
        None,
        None,
    )?;
    if !details_response.status().is_success() {
        return Err(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("TMDB: Error while getting certification");
    }

    #[derive(Deserialize)]
    struct ReleaseDatesResponse {
        results: Vec<ReleaseDatesResult>,
    }
    #[derive(Deserialize)]
    struct ReleaseDatesResult {
        iso_3166_1:    String,
        release_dates: Vec<ReleaseDate>,
    }
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

    details_response
        .json::<ReleaseDatesResponse>()
        .map_err(Into::into)
        .map(|x| {
            x.results
                .iter()
                .filter(|y| y.iso_3166_1 == "US")
                .nth(0)
                .map(|y| {
                    y.release_dates
                        .iter()
                        .filter(|z| z.release_type == 3)
                        .nth(0)
                        .map(|z| z.certification.clone())
                })
                .flatten()
                .ok_or(anyhow!(""))
        })
        .flatten()
}

pub fn get_movie_recommendations(access_token: &str, tmdb_id: u32) -> anyhow::Result<Vec<u32>> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let recommendations_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/recommendations"),
        &headers,
        None,
        None,
    )?;
    if !recommendations_response.status().is_success() {
        return Err(
            match recommendations_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            },
        )
        .context("TMDB: Error while getting recommendations");
    }

    #[derive(Deserialize)]
    struct RecommendationsResponse {
        // page: u32,
        results: Vec<Recommendation>,
    }
    #[derive(Deserialize)]
    struct Recommendation {
        id: u32,
    }

    recommendations_response
        .json::<RecommendationsResponse>()
        .map_err(Into::into)
        .map(|x| x.results.into_iter().map(|y| y.id).take(5).collect())
}

pub(crate) fn get_movie_images(
    access_token: &str,
    tmdb_id: u32,
) -> anyhow::Result<TMDBMovieImagesResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let mut movie_images: TMDBMovieImagesResponse = get_movie_details(access_token, tmdb_id)
        .unwrap_or(TMDBMovieDetails::default())
        .into();

    if movie_images.backdrops.is_empty() || movie_images.posters.is_empty() {
        for query in [vec![("include_image_language", "en")], vec![]] {
            let response = send_tmdb_request(
                &client,
                &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/images"),
                &headers,
                None,
                Some(&query),
            );
            if response.is_err() {
                continue;
            }

            let images_response = response.unwrap();
            if !images_response.status().is_success() {
                continue;
            }

            let result = images_response.json::<TMDBMovieImagesResponse>();

            if let Ok(images) = result {
                if movie_images.backdrops.is_empty() && !images.backdrops.is_empty() {
                    movie_images.backdrops = images.backdrops;
                }
                if movie_images.posters.is_empty() && !images.posters.is_empty() {
                    movie_images.posters = images.posters;
                }
            }

            if !(movie_images.backdrops.is_empty() || movie_images.posters.is_empty()) {
                break;
            }
        }
    }

    Ok(movie_images)
}

pub fn get_movie_artworks(
    cache_dir: &PathBuf,
    access_token: &str,
    tmdb_id: u32,
) -> anyhow::Result<bool> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();

    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let movie_images = get_movie_images(access_token, tmdb_id)?;
    let configuration_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/configuration",
        &headers,
        None,
        None,
    )?;
    if !configuration_response.status().is_success() {
        return Err(
            match configuration_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            },
        )
        .context("TMDB: Error while while querying for configurations");
    }

    let images_configurations = configuration_response
        .json::<ConfigurationResponse>()?
        .images;
    let try_get_artwork = |images_configurations: &ImagesConfiguration,
                           movie_images: &TMDBMovieImagesResponse,
                           path: &PathBuf,
                           backdrop: bool,
                           id: usize|
     -> anyhow::Result<u8> {
        if (backdrop && id >= movie_images.posters.len()) || id >= movie_images.backdrops.len() {
            return Ok(2);
        }

        // let image_bytes = reqwest::blocking::get(format!(
        //     "{}{}{}",
        //     images_configurations.base_url,
        //     if backdrop {
        //         images_configurations.backdrop_sizes[1].clone()
        //     } else {
        //         images_configurations.poster_sizes[3].clone()
        //     },
        //     if backdrop {
        //         movie_images.backdrops[id].file_path.clone()
        //     } else {
        //         movie_images.posters[id].file_path.clone()
        //     }
        // ))?
        // .bytes()?
        // .into_iter()
        // .collect_vec();

        // if let Ok(img) = image::load_from_memory(&image_bytes) {
        //     img.save(path)?;
        // } else {
        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            if backdrop {
                images_configurations.backdrop_sizes.last().unwrap().clone()
            } else {
                images_configurations.poster_sizes.last().unwrap().clone()
            },
            if backdrop {
                movie_images.backdrops[id].file_path.clone()
            } else {
                movie_images.posters[id].file_path.clone()
            }
        ))?
        .bytes()?
        .into_iter()
        .collect();

        if let Ok(img) = image::load_from_memory(&image_bytes) {
            img.resize(
                if backdrop { 10000 } else { 600 },
                if backdrop { 720 } else { 10000 },
                image::imageops::FilterType::CatmullRom,
            )
            .save(path)?;
        } else {
            return Ok(1);
        }
        // }

        Ok(0)
    };

    let mut status = false;
    let poster_path = cache_dir.join("posters").join(format!("{}.jpg", tmdb_id));
    let poster_handle = {
        let images_configurations = images_configurations.clone();
        let movie_images = movie_images.clone();

        thread::spawn(move || -> anyhow::Result<bool> {
            if !movie_images.posters.is_empty() {
                for i in 0..5 {
                    let result = try_get_artwork(
                        &images_configurations,
                        &movie_images,
                        &poster_path,
                        false,
                        i,
                    )?;
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

    let backdrop_path = cache_dir.join("backdrops").join(format!("{}.jpg", tmdb_id));
    let backdrop_handle = {
        thread::spawn(move || -> anyhow::Result<bool> {
            if !movie_images.backdrops.is_empty() {
                for i in 0..5 {
                    let result = try_get_artwork(
                        &images_configurations,
                        &movie_images,
                        &backdrop_path,
                        true,
                        i,
                    )?;
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

pub fn get_person_artwork(
    cache_dir: &PathBuf,
    access_token: &str,
    id: u32,
) -> anyhow::Result<bool> {
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

    let person_details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/person/{id}"),
        &headers,
        None,
        None,
    )?;
    if !person_details_response.status().is_success() {
        return Err(
            match person_details_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(_) => anyhow!(""),
            },
        )
        .context("TMDB: Error while while querying for person details");
    }
    let profile_path = person_details_response
        .json::<PersonDetails>()?
        .profile_path;

    if let Some(profile_path) = profile_path {
        let configuration_response = send_tmdb_request(
            &client,
            "https://api.themoviedb.org/3/configuration",
            &headers,
            None,
            None,
        )?;
        if !configuration_response.status().is_success() {
            return Err(
                match configuration_response.json::<RequestResponseError>() {
                    Ok(err) => err.into(),
                    Err(_) => anyhow!(""),
                },
            )
            .context("TMDB: Error while while querying for configurations");
        }
        let images_configurations = configuration_response
            .json::<ConfigurationResponse>()?
            .images;

        let path = cache_dir.join("persons").join(format!("{}.jpg", id));
        download_image(
            client,
            &format!(
                "{}{}{}",
                images_configurations.base_url,
                images_configurations.poster_sizes[3].clone(),
                profile_path
            ),
            path,
        )?;

        return Ok(true);
    }

    Ok(false)
}

pub fn get_collection_details(
    access_token: &str,
    id: u32,
) -> anyhow::Result<TMDBCollectionDetails> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/collection/{id}"),
        &headers,
        None,
        None,
    )?;
    if !details_response.status().is_success() {
        return Err(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("TMDB: Error while getting collection details");
    }

    details_response.json().map_err(Into::into)
}

pub fn get_collection_artwork(
    cache_dir: &PathBuf,
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
        let configuration_response = send_tmdb_request(
            &client,
            "https://api.themoviedb.org/3/configuration",
            &headers,
            None,
            None,
        )?;
        if !configuration_response.status().is_success() {
            return Err(
                match configuration_response.json::<RequestResponseError>() {
                    Ok(err) => err.into(),
                    Err(_) => anyhow!(""),
                },
            )
            .context("TMDB: Error while while querying for configurations");
        }
        let images_configurations = configuration_response
            .json::<ConfigurationResponse>()?
            .images;

        let path = cache_dir.join("collections").join(format!("{}.jpg", id));
        download_image(
            client,
            &format!(
                "{}{}{}",
                images_configurations.base_url,
                images_configurations.poster_sizes[4].clone(),
                profile_path
            ),
            path,
        )?;

        return Ok(false);
    }

    Ok(false)
}
