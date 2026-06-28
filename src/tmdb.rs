use std::{
    collections::HashMap, error::Error, fmt::Display, path::PathBuf, sync::mpsc::Sender, thread,
};

use anyhow::{Context, bail};
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct RequestTokenResponse {
    // success: bool,
    // expires_at: String,
    request_token: String,
}

#[derive(Deserialize, Debug)]
pub struct RequestResponseError {
    status_code:    i32,
    status_message: String,
    // success: bool,
}
impl Error for RequestResponseError {}
impl Display for RequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.status_code, self.status_message)
    }
}

#[derive(Deserialize, Debug, Clone)]
struct ConfigurationResponse {
    // change_keys: Vec<String>,
    images: ImagesConfiguration,
}
#[derive(Deserialize, Debug, Clone)]
struct ImagesConfiguration {
    base_url:       String,
    backdrop_sizes: Vec<String>, // w92 w154 w185 w342 w500 w780 original
    poster_sizes:   Vec<String>, // w92 w154 w185 w342 w500 w780 original
}

#[derive(PartialEq, Deserialize, Debug, Default)]
pub struct TMDBSearchResponse {
    // page: u64,
    pub results: Vec<TMDBSearchResult>,
    // total_pages: u64,
    // total_results: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct TMDBSearchResult {
    // adult: bool,
    // backdrop_path: Option<String>,
    // genre_ids: Vec<u64>,
    pub id:           u32,
    // original_language: String,
    // original_title: String,
    // overview: String,
    // popularity: f64,
    // poster_path: Option<String>,
    pub release_date: Option<String>,
    pub title:        String,
    // video: bool,
    pub vote_average: Option<f64>,
    pub vote_count:   u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBMovieImagesResponse {
    pub backdrops: Vec<TMDBMovieImage>,
    pub posters:   Vec<TMDBMovieImage>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBMovieImage {
    // aspect_ratio: f32,
    // height: u32,
    // iso_639_1: String,
    pub file_path: String,
    // vote_average: f32,
    // vote_count: u32,
    // width: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBDetailsResponse {
    // adult: bool,
    // backdrop_path: Option<String>,
    pub belongs_to_collection: Option<TMDBCollection>,
    // budget: u32,
    pub genres:                Vec<TMDBGenre>,
    // homepage: Option<String>,
    pub id:                    u32,
    pub imdb_id:               String,
    pub original_language:     String,
    // original_title: String,
    pub overview:              String,
    // popularity: f32,
    // poster_path: Option<String>,
    pub release_date:          String,
    // revenue: u32,
    pub runtime:               u32,
    pub status:                String,
    pub tagline:               String,
    pub title:                 String,
    // video: bool,
    pub vote_average:          f64,
    pub vote_count:            u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TMDBCollection {
    pub id:   u32,
    pub name: String,
    // poster_path: Option<String>,
    // backdrop_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TMDBGenre {
    // id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
struct RequestSessionIDResponse {
    // success: bool,
    session_id: String,
}

// https://developer.themoviedb.org/docs/authentication-user
pub fn get_session_id(
    access_token: &str,
    tx_authorization_url: Sender<String>,
) -> anyhow::Result<String> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", access_token).parse().unwrap(),
    );

    // Step 1: create a request token
    let request_token_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/token/new",
        &headers,
        None,
        None,
    )?;

    if request_token_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(
            match request_token_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(err) => err.into(),
            },
        )
        .context("TMDB: Error while getting a request token");
    }

    let request_token = request_token_response
        .json::<RequestTokenResponse>()?
        .request_token;

    // Step 2: ask the user for permission
    let authorization_url = format!("https://www.themoviedb.org/authenticate/{}", request_token);
    _ = tx_authorization_url.send(authorization_url.clone());

    // Step 3: wait for user permission
    let mut request_token_response = send_tmdb_request(
        &client,
        &format!(
            "https://www.themoviedb.org/authenticate/{}/allow",
            request_token
        ),
        &headers,
        None,
        None,
    )?;
    let mut retries = 0;
    while request_token_response.status().as_u16() >= 400 {
        retries += 1;
        if retries > 50 {
            bail!("TMDB: couldn't authenticate request token, max retries reached");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
        request_token_response = send_tmdb_request(
            &client,
            &format!(
                "https://www.themoviedb.org/authenticate/{}/allow",
                request_token
            ),
            &headers,
            None,
            None,
        )?;
    }
    drop(tx_authorization_url);

    // The request token has been approved by the user
    // Step 4: finally create a new session ID
    let mut body = HashMap::new();
    body.insert("request_token", request_token.as_str());
    let create_session_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/session/new",
        &headers,
        Some(body),
        None,
    )?;

    if create_session_response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(
            match create_session_response.json::<RequestResponseError>() {
                Ok(err) => err.into(),
                Err(err) => err.into(),
            },
        )
        .context("TMDB: Error while creating a new session ID");
    }

    let session_id = create_session_response
        .json::<RequestSessionIDResponse>()?
        .session_id;
    Ok(session_id)
}

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

pub fn get_movie_images(
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

        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
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
        .collect();

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
                        ratatui_image::FilterType::CatmullRom,
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
                let mut success = false;
                for i in 0..5 {
                    let result = try_get_artwork(
                        &images_configurations,
                        &movie_images,
                        &poster_path,
                        false,
                        i,
                    )?;
                    match result {
                        0 => {
                            success = true;
                            break;
                        }
                        2 => {
                            break;
                        }
                        _ => (),
                    }
                }
                if !success {
                    std::fs::copy("poster_placeholder.jpg", poster_path)?;
                }
            } else {
                std::fs::copy("poster_placeholder.jpg", poster_path)?;
            }

            Ok(())
        })
    };

    let backdrop_path = cache_dir.join("backdrops").join(format!("{}.jpg", tmdb_id));
    let backdrop_handle = {
        thread::spawn(move || -> anyhow::Result<()> {
            if !movie_images.backdrops.is_empty() {
                let mut success = false;
                for i in 0..5 {
                    let result = try_get_artwork(
                        &images_configurations,
                        &movie_images,
                        &backdrop_path,
                        true,
                        i,
                    )?;
                    match result {
                        0 => {
                            success = true;
                            break;
                        }
                        2 => {
                            break;
                        }
                        _ => (),
                    }
                }
                if !success {
                    std::fs::copy("backdrop_placeholder.jpg", backdrop_path)?;
                }
            } else {
                std::fs::copy("backdrop_placeholder.jpg", backdrop_path)?;
            }

            Ok(())
        })
    };

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(())
}

fn send_tmdb_request(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> anyhow::Result<Response> {
    let mut request: RequestBuilder;
    if body.is_none() {
        request = client.get(url).headers(headers.clone());
        if query.is_some() {
            request = request.query(&query.unwrap());
        }
    } else {
        request = client
            .post(url)
            .headers(headers.clone())
            .json(&body.clone().unwrap());
    }

    let response = request.send()?;
    Ok(response)
}
