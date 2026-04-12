use crate::tokens::TraktUserTokens;
use anyhow::{bail, Context};
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    header::{HeaderMap, CONTENT_TYPE, USER_AGENT},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    thread,
};

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    // token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    // scope: String,
    pub created_at: i64,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponseError {
    error: String,
    error_description: String,
}
impl Error for TokenResponseError {}
impl Display for TokenResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.error_description)
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TraktDetailsResponse {
    pub title: String,
    pub year: u32,
    pub ids: IDs,
    pub tagline: String,
    pub overview: String,
    pub released: String,
    pub runtime: u32,
    // country: String,
    pub trailer: Option<String>,
    // homepage: String,
    pub status: String,
    pub rating: f64,
    pub votes: u32,
    // comment_count: u32,
    // updated_at: String,
    // language: String,
    // languages: Vec<String>,
    // available_translations: Vec<String>,
    pub genres: Vec<String>,
    pub certification: Option<String>,
    pub images: TraktMovieImages,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TraktMovieImages {
    fanart: Vec<String>,
    poster: Vec<String>,
    // logo: Vec<String>,
    // clearart: Vec<String>,
    banner: Vec<String>,
    // thumb: Vec<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct IDs {
    // trakt: u32,
    // slug: String,
    // imdb: String,
    tmdb: u32,
}

pub fn should_refresh_tokens(trakt_tokens: &TraktUserTokens) -> bool {
    unix_ts::Timestamp::now().seconds() - trakt_tokens.expires_on > 0
}

// https://trakt.docs.apiary.io/#reference/authentication-oauth/authorize/authorize-application
pub fn get_tokens(
    client_id: &str,
    client_secret: &str,
    tx_auth_url: Sender<String>,
    rx_auth_code: Receiver<String>,
) -> anyhow::Result<TokenResponse> {
    let client = reqwest::blocking::Client::new();

    // Step 1: ask the user for an authorization code
    let authorization_url = client
        .get("https://trakt.tv/oauth/authorize")
        .query(&[
            ("client_id", client_id),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
            ("response_type", "code"),
        ])
        .header(CONTENT_TYPE, "application/json")
        .build()?
        .url()
        .to_string();

    _ = tx_auth_url.send(authorization_url);

    let auth_code = rx_auth_code
        .recv_timeout(std::time::Duration::from_secs(120))
        .unwrap_or_default();
    if auth_code.is_empty() {
        bail!("Trakt: no auth code received");
    }

    // Step 2: exchange authorization code for access token
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let mut body = HashMap::new();
    body.insert("code", auth_code.as_str());
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "authorization_code");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        &headers,
        Some(body),
        None,
    )?;

    if token_response.status().as_u16() >= 400 {
        return Err::<_, anyhow::Error>(match token_response.json::<TokenResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("Trakt: Error while while exchanging auth code for access token");
    }

    Ok(token_response.json::<TokenResponse>()?)
}

pub fn refresh_tokens(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> anyhow::Result<TokenResponse> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let mut body = HashMap::new();
    body.insert("refresh_token", refresh_token);
    body.insert("client_id", client_id);
    body.insert("client_secret", client_secret);
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "refresh_token");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        &headers,
        Some(body),
        None,
    )?;

    // debug!("{:#?}", token_response);

    if token_response.status().as_u16() >= 400 {
        return Err::<_, anyhow::Error>(match token_response.json::<TokenResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("Trakt: Error while while refreshing access token");
    }

    Ok(token_response.json::<TokenResponse>()?)
}

pub fn get_movie_details(client_id: &str, imdb_id: &str) -> anyhow::Result<TraktDetailsResponse> {
    let client = ClientBuilder::new().build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let query = [("type", "movie"), ("extended", "full,images")];

    let details_response = send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/movies/{imdb_id}"),
        &headers,
        None,
        Some(&query),
    )?;

    if details_response.status().as_u16() != 200 {
        bail!("couldn't get movie details with trakt");
    }

    // let movies = details_response.text()?;
    // panic!("{:#}", json::parse(movies.as_str()).unwrap());
    Ok(details_response.json()?)
}

pub fn get_movie_poster_banner(
    cache_dir: &PathBuf,
    client_id: &str,
    id: String,
    add_placeholder: bool,
) -> anyhow::Result<bool> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(USER_AGENT, "reqwest/0.12.8".parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    headers.insert("trakt-api-key", client_id.parse().unwrap());

    let movie_details = get_movie_details(client_id, &id)?;

    if !add_placeholder
        && ((movie_details.images.banner.is_empty() && movie_details.images.fanart.is_empty())
            || movie_details.images.poster.is_empty())
    {
        return Ok(false);
    }

    let client = Client::builder().default_headers(headers).build()?;

    let _client = client.clone();
    let path = cache_dir
        .join("posters")
        .join(format!("{}.jpg", movie_details.ids.tmdb));
    let poster_handle = thread::spawn(move || -> anyhow::Result<()> {
        if !movie_details.images.poster.is_empty() {
            let mut image_url = movie_details.images.poster[0].as_str();
            if let Some(stripped) = image_url.strip_suffix(".webp") {
                image_url = stripped;
            }

            let image_bytes: Vec<_> = _client
                .get(format!("https://{image_url}"))
                .send()?
                .bytes()?
                .iter()
                .copied()
                .collect();

            if let Ok(img) = image::load_from_memory(&image_bytes) {
                img.save(path)?;
            } else if add_placeholder {
                fs::copy("poster_placeholder.jpg", path)?;
            }
        } else if add_placeholder {
            fs::copy("poster_placeholder.jpg", path)?;
        }
        Ok(())
    });

    let path = cache_dir
        .join("backdrops")
        .join(format!("{}.jpg", movie_details.ids.tmdb));
    let backdrop_handle = thread::spawn(move || -> anyhow::Result<()> {
        if !movie_details.images.fanart.is_empty() {
            let mut image_url = movie_details.images.fanart[0].as_str();
            if let Some(stripped) = image_url.strip_suffix(".webp") {
                image_url = stripped;
            }

            let image_bytes: Vec<_> = client
                .get(format!("https://{image_url}"))
                .send()?
                .bytes()?
                .iter()
                .copied()
                .collect();

            if let Ok(img) = image::load_from_memory(&image_bytes) {
                img.save(path)?;
            } else if add_placeholder {
                fs::copy("backdrop_placeholder.jpg", path)?;
            }
        } else if !movie_details.images.banner.is_empty() {
            let mut image_url = movie_details.images.banner[0].as_str();
            if let Some(stripped) = image_url.strip_suffix(".webp") {
                image_url = stripped;
            }

            let image_bytes: Vec<_> = client
                .get(format!("https://{image_url}"))
                .send()?
                .bytes()?
                .iter()
                .copied()
                .collect();

            if let Ok(img) = image::load_from_memory(&image_bytes) {
                img.save(path)?;
            } else if add_placeholder {
                fs::copy("backdrop_placeholder.jpg", path)?;
            }
        } else if add_placeholder {
            fs::copy("backdrop_placeholder.jpg", path)?;
        }

        Ok(())
    });

    poster_handle.join().unwrap()?;
    backdrop_handle.join().unwrap()?;

    Ok(true)
}

fn send_trakt_request(
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

    // error while sending the request????
    let response = request.send()?;
    Ok(response)
}
