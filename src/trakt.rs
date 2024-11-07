// use oauth2::basic::BasicClient;
// use oauth2::reqwest::http_client;
// use oauth2::{
//     AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
//     Scope, TokenResponse, TokenUrl,
// };
use crate::{app::Config, config_trakt::TraktConfig};
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{stdin, stdout, Write},
};
// use trakt_rs::smo::*;
// use trakt_rs::{Request, Response as Rewponse};

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
    refresh_token: String,
    scope: String,
    created_at: i32,
}

#[derive(Deserialize, Debug)]
struct TokenResponseError {
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
    // pub ids: IDs,
    pub tagline: String,
    pub overview: String,
    pub released: String,
    pub runtime: u32,
    // pub country: String,
    pub trailer: Option<String>,
    // pub homepage: String,
    pub status: String,
    pub rating: f64,
    pub votes: u32,
    // pub comment_count: u32,
    // pub updated_at: String,
    // pub language: String,
    // pub languages: Vec<String>,
    // pub available_translations: Vec<String>,
    pub genres: Vec<String>,
    pub certification: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct IDs {
    trakt: u32,
    slug: String,
    imdb: String,
    tmdb: u32,
}

pub fn populate_tokens(
    config: &Config,
    trakt_config: &mut TraktConfig,
) -> Result<(), Box<dyn Error>> {
    if !trakt_config.has_tokens() {
        get_tokens(config, trakt_config)?
    } else if unix_ts::Timestamp::from(trakt_config.tokens_expiration_data() - 86_400)
        < unix_ts::Timestamp::now()
    {
        println!("Refreshing tokens");
        refresh_tokens(trakt_config)?
    }

    Ok(())
}

fn get_tokens(config: &Config, trakt_config: &mut TraktConfig) -> Result<(), Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let authorization_url = client
        .get("https://api.trakt.tv/oauth/authorize")
        .query(&[
            ("response_type", "code"),
            ("client_id", trakt_config.client_id()),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
        ])
        .headers(headers.clone())
        .build()?
        .url()
        .to_string();

    println!(
        "\nPlease visit the following url to authorize the application.\n{}\n",
        authorization_url
    );

    let mut auth_code = String::new();
    print!("Please enter the auth code from the url: ");
    let _ = stdout().flush();
    stdin()
        .read_line(&mut auth_code)
        .expect("Did not enter a correct string");
    if let Some('\n') = auth_code.chars().next_back() {
        auth_code.pop();
    }
    if let Some('\r') = auth_code.chars().next_back() {
        auth_code.pop();
    }

    let mut body = HashMap::new();
    body.insert("code", auth_code.as_str());
    body.insert("client_id", trakt_config.client_id());
    body.insert("client_secret", trakt_config.client_secret());
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "authorization_code");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        headers.clone(),
        Some(body),
        None,
    )?;

    if token_response.status().as_u16() >= 400 {
        return Err(Box::new(
            //     std::io::Error::new(
            //     std::io::ErrorKind::Other,
            //     "Error {}: Invalid authorization code!",
            // )
            token_response.json::<TokenResponseError>()?,
        ));
    }
    let token_response = token_response.json::<TokenResponse>()?;

    trakt_config.set_trakt_tokens(
        token_response.access_token,
        token_response.refresh_token,
        token_response.created_at,
        token_response.expires_in,
    );
    trakt_config.save_creds(config)?;

    Ok(())
}

fn refresh_tokens(trakt_config: &mut TraktConfig) -> Result<(), Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let mut body = HashMap::new();
    body.insert("refresh_token", trakt_config.refresh_token());
    body.insert("client_id", trakt_config.client_id());
    body.insert("client_secret", trakt_config.client_secret());
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "refresh_token");

    let token_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/oauth/token",
        headers.clone(),
        Some(body),
        None,
    )?;

    if token_response.status().as_u16() >= 400 {
        return Err(Box::new(
            //     std::io::Error::new(
            //     std::io::ErrorKind::Other,
            //     "Error {}: Invalid authorization code!",
            // )
            token_response.json::<TokenResponseError>()?,
        ));
    }
    let token_response = token_response.json::<TokenResponse>()?;

    trakt_config.set_trakt_tokens(
        token_response.access_token,
        token_response.refresh_token,
        token_response.created_at,
        token_response.expires_in,
    );

    Ok(())
}

pub fn get_movie_details(
    trakt_config: &TraktConfig,
    imdb_id: &str,
) -> Result<TraktDetailsResponse, Box<dyn Error>> {
    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();

    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("trakt-api-key", trakt_config.client_id().parse().unwrap());
    headers.insert("trakt-api-version", "2".parse().unwrap());
    let query = [("type", "movie"), ("extended", "full")];

    let details_response = send_trakt_request(
        &client,
        &format!("https://api.trakt.tv/movies/{imdb_id}"),
        headers,
        None,
        Some(&query),
    )?;

    if details_response.status().as_u16() != 200 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Couldn't get movie details with trakt!",
        )));
    }

    // let movies = details_response.text()?;
    // println!("{:#}", json::parse(movies.as_str()).unwrap());
    let json = details_response.json::<TraktDetailsResponse>()?;
    Ok(json)
}

// #[derive(Debug)]
// struct MyError(String);
// impl Error for MyError {}
// impl Display for MyError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "There is an error: {}", self.0)
//     }
// }

fn send_trakt_request(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: Option<HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> Result<Response, Box<dyn Error>> {
    // let mut retry_attempts = 0;
    // let mut retry_delay = 1;

    // while retry_attempts < 2 {
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

    // println!("{:#?}", request.try_clone().unwrap().build().unwrap());
    // println!(
    //     "{:#?}",
    //     request
    //         .try_clone()
    //         .unwrap()
    //         .build()
    //         .unwrap()
    //         .body()
    //         .unwrap()
    // );

    let response = request.send()?;
    Ok(response)
    // if response.status().as_u16() <= 204 {
    // return Ok(response);
    // } else if response.status().as_u16() <= 522 {
    //     retry_attempts += 1;
    //     println!("{retry_attempts}Retrying {}", response.status());
    //     std::thread::sleep(std::time::Duration::from_secs(retry_delay));
    //     retry_delay *= 2;
    // }
    // else {
    //     return Err(Box::new(std::io::Error::new(
    //         std::io::ErrorKind::Other,
    //         format!(
    //             "Unrecognized status code {} when requesting from {}",
    //             response.status(),
    //             url
    //         ),
    //     )));
    // }
    // }

    // Err(Box::new(std::io::Error::new(
    //     std::io::ErrorKind::Other,
    //     format!("Maximum retries reached while requesting {}", url),
    // )))
}
