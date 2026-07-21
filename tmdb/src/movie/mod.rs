use std::{path::PathBuf, thread};

use anyhow::Context;
use itertools::Itertools;
use reqwest::{blocking::ClientBuilder, header::HeaderMap};

use crate::{
    send_tmdb_request,
    smo::{
        ConfigurationResponse, ImagesConfiguration, RequestResponseError, TMDBDetailsResponse,
        TMDBMovieImagesResponse, TMDBSearchResponse, TMDBSearchResult,
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
    if search_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(match search_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context(format!("TMDB: Error while searching for movie {}", name));
    }

    let json = search_response.json::<TMDBSearchResponse>()?;
    Ok(json.results)
}

pub fn get_movie_details(access_token: &str, tmdb_id: u32) -> anyhow::Result<TMDBDetailsResponse> {
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
    if details_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(match details_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("TMDB: Error while getting movie details");
    }

    Ok(details_response.json::<TMDBDetailsResponse>()?)
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

    let query = [("include_image_language", "en")];

    let images_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/images"),
        &headers,
        None,
        Some(&query),
    )?;
    if images_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(match images_response.json::<RequestResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("TMDB: Error while while querying for movie images");
    }

    let mut movie_images = images_response.json::<TMDBMovieImagesResponse>()?;
    if movie_images.backdrops.is_empty() || movie_images.posters.is_empty() {
        let response = send_tmdb_request(
            &client,
            &format!("https://api.themoviedb.org/3/movie/{tmdb_id}/images"),
            &headers,
            None,
            None,
        );
        if response.is_err() {
            return Ok(movie_images);
        }

        let images_response = response.unwrap();
        if images_response.status().as_u16() != 200 {
            return Ok(movie_images);
        }

        let result = images_response.json::<TMDBMovieImagesResponse>();

        if let Ok(unfiltered_images) = result {
            if movie_images.backdrops.is_empty() && !unfiltered_images.backdrops.is_empty() {
                movie_images.backdrops = unfiltered_images.backdrops;
            }
            if movie_images.posters.is_empty() && !unfiltered_images.posters.is_empty() {
                movie_images.posters = unfiltered_images.posters;
            }
        }
    }

    Ok(movie_images)
}

pub fn get_movie_poster_banner(
    cache_dir: &PathBuf,
    access_token: &str,
    tmdb_id: u32,
) -> anyhow::Result<()> {
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
    if configuration_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(
            match configuration_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(err) => err.into(),
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

        let image_bytes = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            if backdrop {
                images_configurations.backdrop_sizes[1].clone()
            } else {
                images_configurations.poster_sizes[3].clone()
            },
            if backdrop {
                movie_images.backdrops[id].file_path.clone()
            } else {
                movie_images.posters[id].file_path.clone()
            }
        ))?
        .bytes()?
        .into_iter()
        .collect_vec();

        let img = image::load_from_memory(&image_bytes);
        if img.is_ok() {
            img.unwrap().save(path)?;
        } else if img.is_err() {
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

            let img = image::load_from_memory(&image_bytes);
            if img.is_ok() {
                img.unwrap()
                    .resize(
                        if backdrop { 780 } else { 342 },
                        10000,
                        image::imageops::FilterType::CatmullRom,
                    )
                    .save(path)?;
            } else if img.is_err() {
                return Ok(1);
            }
        }

        Ok(0)
    };

    let poster_path = cache_dir.join("posters").join(format!("{}.jpg", tmdb_id));
    let poster_handle = {
        let images_configurations = images_configurations.clone();
        let movie_images = movie_images.clone();

        thread::spawn(move || -> anyhow::Result<()> {
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
                        0 | 2 => {
                            break;
                        }
                        _ => (),
                    }
                }
            }

            Ok(())
        })
    };

    let backdrop_path = cache_dir.join("backdrops").join(format!("{}.jpg", tmdb_id));
    let backdrop_handle = {
        thread::spawn(move || -> anyhow::Result<()> {
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
                        0 | 2 => {
                            break;
                        }
                        _ => (),
                    }
                }
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}
