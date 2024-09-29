// use oauth2::basic::BasicClient;
// use oauth2::reqwest::http_client;
// use oauth2::{
//     AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
//     Scope, TokenResponse, TokenUrl,
// };
use crate::{app::Movie, config_tmdb::Conf};
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

#[derive(Deserialize, Debug)]
struct SearchResponse {
    // page: u64,
    results: Vec<SearchResult>,
    // total_pages: u64,
    // total_results: u64,
}

#[derive(Deserialize, Debug)]
pub struct SearchResult {
    // pub adult: bool,
    // pub backdrop_path: Option<String>,
    // pub genre_ids: Vec<u64>,
    pub id: u64,
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

#[derive(Deserialize, Debug)]
pub struct DetailsResponse {
    // adult: bool,
    pub backdrop_path: Option<String>,
    pub belongs_to_collection: Option<Collection>,
    // budget: u32,
    pub genres: Vec<Genre>,
    pub homepage: Option<String>,
    pub id: u32,
    pub imdb_id: String,
    // original_language: String,
    // original_title: String,
    pub overview: String,
    // popularity: f32,
    pub poster_path: Option<String>,
    pub release_date: String,
    // revenue: u32,
    pub runtime: u32,
    pub status: String,
    pub tagline: String,
    pub title: String,
    // video: bool,
    pub vote_average: f32,
    pub vote_count: u32,
}
#[derive(Deserialize, Debug)]
pub struct Collection {
    pub id: u32,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Genre {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
struct RequestSessionIDResponse {
    // success: bool,
    session_id: String,
}

// const ALTERNATE_POSTER_FILE: String = String::from("placeholder.png");

pub fn populate_tokens(config: &mut Conf) -> Result<(), Box<dyn Error>> {
    if !config.has_session_id() {
        get_session_id(config)?
    }

    Ok(())
}

fn get_session_id(config: &mut Conf) -> Result<(), Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", config.access_token()).parse().unwrap(),
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
    config.set_session_id(session_id);
    config.save_creds()?;

    Ok(())
}

pub fn find_movie(config: &Conf, name: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", config.access_token()).parse().unwrap(),
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

    let json = search_response.json::<SearchResponse>()?;
    // println!("{:#?}", json);
    Ok(json.results)
}

pub fn get_movie_details(config: &Conf, id: u32) -> Result<DetailsResponse, Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("accept", "application/json".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(
        "Authorization",
        format!("Bearer {}", config.access_token()).parse().unwrap(),
    );

    let details_response = send_tmdb_request(
        &client,
        &format!("https://api.themoviedb.org/3/movie/{id}"),
        headers.clone(),
        None,
        None,
    )?;
    if details_response.status().as_u16() != 200 {
        return Err(Box::new(details_response.json::<RequestResponseError>()?));
    }

    Ok(details_response.json::<DetailsResponse>()?)
}

pub fn get_movie_poster_banner(config: &Conf, movie: &Movie) -> Result<(), Box<dyn Error>> {
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
        format!("Bearer {}", config.access_token()).parse().unwrap(),
    );

    let movie_details = get_movie_details(config, movie.id)?;

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
