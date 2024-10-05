// use oauth2::basic::BasicClient;
// use oauth2::reqwest::http_client;
// use oauth2::{
//     AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
//     Scope, TokenResponse, TokenUrl,
// };
use crate::{app::Config, config_tmdb::TMDBConfig};
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
struct RequestResponseError {
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

// const ALTERNATE_POSTER_FILE: String = String::from("placeholder.png");

pub fn populate_tokens(
    config: &Config,
    tmdb_config: &mut TMDBConfig,
) -> Result<(), Box<dyn Error>> {
    if !tmdb_config.has_session_id() {
        get_session_id(config, tmdb_config)?
    }

    Ok(())
}

fn get_session_id(config: &Config, tmdb_config: &mut TMDBConfig) -> Result<(), Box<dyn Error>> {
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

    let validate_key_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/token/new",
        headers.clone(),
        None,
        None,
    )?;
    if validate_key_response.status().as_u16() != 200 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid access token!",
        )));
    }

    let request_token_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/token/new",
        headers.clone(),
        None,
        None,
    )?;

    if request_token_response.status().as_u16() != 200 {
        return Err(Box::new(
            request_token_response.json::<RequestResponseError>()?,
        ));
    }

    let request_token = request_token_response
        .json::<RequestTokenResponse>()?
        .request_token;

    println!(
        "\nPlease visit the following url to authorize the application.\nhttps://www.themoviedb.org/authenticate/{}\n",
        request_token
    );

    let mut request_token_response = send_tmdb_request(
        &client,
        &format!(
            "https://www.themoviedb.org/authenticate/{}/allow",
            request_token
        ),
        headers.clone(),
        None,
        None,
    )?;

    let mut retries = 0;
    while request_token_response.status().as_u16() >= 400 {
        println!(
            "{:#?} {}",
            request_token_response.status(),
            request_token_response.url()
        );
        retries += 1;
        if retries > 20 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Couldn't authenticate request token!",
            )));
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
        request_token_response = send_tmdb_request(
            &client,
            &format!(
                "https://www.themoviedb.org/authenticate/{}/allow",
                request_token
            ),
            headers.clone(),
            None,
            None,
        )?;
    }

    let mut body = HashMap::new();
    body.insert("request_token", request_token.as_str());

    let create_session_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/authentication/session/new",
        headers.clone(),
        Some(body),
        None,
    )?;

    if create_session_response.status().as_u16() != 200 {
        return Err(Box::new(
            create_session_response.json::<RequestResponseError>()?,
        ));
    }

    let session_id = create_session_response
        .json::<RequestSessionIDResponse>()?
        .session_id;

    println!("{session_id}");
    tmdb_config.set_session_id(session_id);
    tmdb_config.save_creds(config)?;

    Ok(())
}

pub fn find_movie(
    tmdb_config: &TMDBConfig,
    name: &str,
) -> Result<TMDBSearchResponse, Box<dyn Error>> {
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

    let query = [("query", name), ("language", "en-US")];
    let search_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/search/movie",
        headers.clone(),
        None,
        Some(&query),
    )?;
    if search_response.status().as_u16() != 200 {
        return Err(Box::new(search_response.json::<RequestResponseError>()?));
    }
    // println!(
    //     "{}",
    //     json::parse(&search_response.text()?).unwrap().pretty(2)
    // );

    let json = search_response.json::<TMDBSearchResponse>()?;
    // println!("{:#?}", json);
    Ok(json)
    // Err(Box::new(std::io::Error::new(
    //     std::io::ErrorKind::Other,
    //     "ass",
    // )))
}

pub fn get_movie_details(
    tmdb_config: &TMDBConfig,
    tmdb_id: u32,
) -> Result<TMDBDetailsResponse, Box<dyn Error>> {
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
        headers.clone(),
        None,
        None,
    )?;
    if details_response.status().as_u16() != 200 {
        return Err(Box::new(details_response.json::<RequestResponseError>()?));
    }

    Ok(details_response.json::<TMDBDetailsResponse>()?)
}

pub fn get_movie_poster_banner(
    config: &Config,
    tmdb_config: &TMDBConfig,
    id: u32,
) -> Result<(), Box<dyn Error>> {
    let poster_cache = config.cache.join("posters");
    let backdrop_cache = config.cache.join("backdrops");
    std::fs::create_dir_all(&poster_cache)?;
    std::fs::create_dir_all(&backdrop_cache)?;

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

    let movie_details = get_movie_details(tmdb_config, id)?;

    // println!(
    //     "{} {}\n{}",
    //     movie_details.title, movie_details.release_date, movie_details.id
    // );

    let configuration_response = send_tmdb_request(
        &client,
        "https://api.themoviedb.org/3/configuration",
        headers.clone(),
        None,
        None,
    )?;
    if configuration_response.status().as_u16() != 200 {
        return Err(Box::new(
            configuration_response.json::<RequestResponseError>()?,
        ));
    }

    let images_configurations = configuration_response
        .json::<ConfigurationResponse>()?
        .images;

    if movie_details.poster_path.is_some() {
        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            images_configurations.poster_sizes[1],
            movie_details.poster_path.as_ref().unwrap()
        ))
        .expect("requesting movie poster failed!")
        .bytes()?
        .iter()
        .copied()
        .collect();
        let mut out = File::create(poster_cache.join(format!("{}.jpg", movie_details.id)))
            .expect("failed to create file");
        std::io::copy(&mut image_bytes.as_slice(), &mut out).expect("failed to copy content");
    } else {
        std::fs::copy(
            "poster_placeholder.jpg",
            poster_cache.join(format!("{}.jpg", movie_details.id)),
        )
        .expect("failed to copy placeholder poster!");
    }

    if movie_details.backdrop_path.is_some() {
        let image_bytes: Vec<_> = reqwest::blocking::get(format!(
            "{}{}{}",
            images_configurations.base_url,
            images_configurations.backdrop_sizes[1],
            movie_details.backdrop_path.as_ref().unwrap()
        ))
        .expect("requesting movie backdrop failed!")
        .bytes()?
        .iter()
        .copied()
        .collect();
        let mut out = File::create(backdrop_cache.join(format!("{}.jpg", movie_details.id)))
            .expect("failed to create file");
        std::io::copy::<&[u8], File>(&mut image_bytes.as_slice(), &mut out)
            .expect("failed to copy content");
    } else {
        std::fs::copy(
            "backdrop_placeholder.jpg",
            poster_cache.join(format!("{}.jpg", movie_details.id)),
        )
        .expect("failed to copy placeholder backdrop!");
    }

    Ok(())
}

fn send_tmdb_request(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: Option<HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> Result<Response, Box<dyn Error>> {
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
