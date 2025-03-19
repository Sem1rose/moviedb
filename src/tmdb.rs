use crate::{
    app::{Config, Errors, Result},
    config_tmdb::TMDBConfig,
};
use log::{debug, trace};
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use std::{collections::HashMap, error::Error, fmt::Display, fs::File};

#[derive(Deserialize, Debug)]
struct RequestTokenResponse {
    // success: bool,
    // expires_at: String,
    request_token: String,
}

#[derive(Deserialize, Debug)]
pub struct RequestResponseError {
    status_code: i32,
    status_message: String,
    // success: bool,
}
impl Error for RequestResponseError {}
impl Display for RequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error code {}: {}",
            self.status_code, self.status_message
        )
    }
}

#[derive(Deserialize, Debug)]
struct ConfigurationResponse {
    // pub change_keys: Vec<String>,
    images: ImagesConfiguration,
}
#[derive(Deserialize, Debug)]
struct ImagesConfiguration {
    base_url: String,
    backdrop_sizes: Vec<String>,
    poster_sizes: Vec<String>,
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
    // pub adult: bool,
    // pub backdrop_path: Option<String>,
    // pub genre_ids: Vec<u64>,
    pub id: u32,
    // pub original_language: String,
    // pub original_title: String,
    // pub overview: String,
    // pub popularity: f64,
    // pub poster_path: Option<String>,
    pub release_date: String,
    pub title: String,
    // pub video: bool,
    pub vote_average: f64,
    // pub vote_count: u64,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBMovieImagesResponse {
    pub backdrops: Vec<TMDBMovieImage>,
    pub posters: Vec<TMDBMovieImage>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBMovieImage {
    pub aspect_ratio: f32,
    pub height: u32,
    pub iso_639_1: String,
    pub file_path: String,
    pub vote_average: f32,
    pub vote_count: u32,
    pub width: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBDetailsResponse {
    // pub adult: bool,
    pub backdrop_path: Option<String>,
    pub belongs_to_collection: Option<TMDBCollection>,
    // pub budget: u32,
    pub genres: Vec<TMDBGenre>,
    pub homepage: Option<String>,
    pub id: u32,
    pub imdb_id: String,
    pub original_language: String,
    // pub original_title: String,
    pub overview: String,
    // pub popularity: f32,
    pub poster_path: Option<String>,
    pub release_date: String,
    // pub revenue: u32,
    pub runtime: u32,
    pub status: String,
    pub tagline: String,
    pub title: String,
    // pub video: bool,
    pub vote_average: f64,
    pub vote_count: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TMDBCollection {
    pub id: u32,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TMDBGenre {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
struct RequestSessionIDResponse {
    // success: bool,
    session_id: String,
}

pub fn populate_tokens(config: &Config, tmdb_config: &mut TMDBConfig) -> Result<()> {
    if !tmdb_config.has_session_id() {
        debug!("No TMDB session ID found, fetching a new one...");

        get_session_id(config, tmdb_config)?
    }

    Ok(())
}

// https://developer.themoviedb.org/docs/authentication-user
fn get_session_id(config: &Config, tmdb_config: &mut TMDBConfig) -> Result<()> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", tmdb_config.access_token())
            .parse()
            .unwrap(),
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
        let result = request_token_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }

    let request_token = request_token_response
        .json::<RequestTokenResponse>()?
        .request_token;

    // Step 2: ask the user for permission
    println!(
        "\nPlease visit the following url to authorize the application.\nhttps://www.themoviedb.org/authenticate/{}\n",
        request_token
    );

    // Step 4: finally get the request token
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

    // once the user gives permission of course...
    let mut retries = 0;
    while request_token_response.status().as_u16() >= 400 {
        trace!(
            "{:#?} {}",
            request_token_response.status(),
            request_token_response.url()
        );
        retries += 1;
        if retries > 50 {
            return Err(Errors::Other(
                "couldn't authenticate request token, max retries reached".to_string(),
            ));
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

    let mut body = HashMap::new();
    body.insert("request_token", request_token.as_str());

    // The request token has been approved by the user
    // Step 5: create a new session ID
    let create_session_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/session/new",
        &headers,
        Some(body),
        None,
    )?;

    if create_session_response.status().as_u16() != 200 {
        let result = create_session_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }

    let session_id = create_session_response
        .json::<RequestSessionIDResponse>()?
        .session_id;

    tmdb_config.set_session_id(session_id);
    tmdb_config.save_creds(config)?;

    Ok(())
}

pub fn find_movie(tmdb_config: &TMDBConfig, name: &str) -> Result<TMDBSearchResponse> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", tmdb_config.access_token())
            .parse()
            .unwrap(),
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
        let result = search_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }
    // println!(
    //     "{}",
    //     json::parse(&search_response.text()?).unwrap().pretty(2)
    // );

    let json = search_response.json::<TMDBSearchResponse>()?;
    // println!("{:#?}", json);
    Ok(json)
}

pub fn get_movie_details(tmdb_config: &TMDBConfig, tmdb_id: u32) -> Result<TMDBDetailsResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", tmdb_config.access_token())
            .parse()
            .unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{tmdb_id}"),
        &headers,
        None,
        None,
    )?;
    if details_response.status().as_u16() != 200 {
        let result = details_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }

    Ok(details_response.json::<TMDBDetailsResponse>()?)
}

pub fn get_movie_images(tmdb_config: &TMDBConfig, tmdb_id: u32) -> Result<TMDBMovieImagesResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", tmdb_config.access_token())
            .parse()
            .unwrap(),
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
        let result = images_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
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
    config: &Config,
    tmdb_config: &TMDBConfig,
    id: u32,
    add_placeholder: bool,
) -> Result<bool> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();

    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", tmdb_config.access_token())
            .parse()
            .unwrap(),
    );

    let movie_images = get_movie_images(tmdb_config, id)?;

    let configuration_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/configuration",
        &headers,
        None,
        None,
    )?;
    if configuration_response.status().as_u16() != 200 {
        let result = configuration_response.json::<RequestResponseError>();
        if let Ok(error) = result {
            return Err(Errors::TMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }

    let images_configurations = configuration_response
        .json::<ConfigurationResponse>()?
        .images;

    if !add_placeholder && (movie_images.posters.is_empty() || movie_images.backdrops.is_empty()) {
        return Ok(false);
    }

    if !movie_images.posters.is_empty() {
        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            images_configurations.poster_sizes[4], // w92 w154 w185 w342 w500 w780 original
            movie_images.posters[0].file_path
        ))?
        // .expect("requesting movie poster failed!")
        .bytes()?
        .iter()
        .copied()
        .collect();

        let mut out = File::create(config.dirs.poster_cache.join(format!("{}.jpg", id)))?;
        // .expect("failed to create file");
        std::io::copy(&mut image_bytes.as_slice(), &mut out)?; //.expect("failed to copy content");
    } else if add_placeholder {
        std::fs::copy(
            "poster_placeholder.jpg",
            config.dirs.poster_cache.join(format!("{}.jpg", id)),
        )?;
        // .expect("failed to copy placeholder poster!");
    }

    if !movie_images.backdrops.is_empty() {
        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            images_configurations.backdrop_sizes[2], // w300 w780 w1280 original
            movie_images.backdrops[0].file_path
        ))?
        // .expect("requesting movie backdrop failed!")
        .bytes()?
        .iter()
        .copied()
        .collect();

        let mut out = File::create(config.dirs.backdrop_cache.join(format!("{}.jpg", id)))?;
        // .expect("failed to create file");
        std::io::copy::<&[u8], File>(&mut image_bytes.as_slice(), &mut out)?;
        // .expect("failed to copy content");
    } else if add_placeholder {
        std::fs::copy(
            "backdrop_placeholder.jpg",
            config.dirs.poster_cache.join(format!("{}.jpg", id)),
        )?;
        // .expect("failed to copy placeholder backdrop!");
    }

    Ok(true)
}

fn send_tmdb_request(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: Option<HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> Result<Response> {
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
