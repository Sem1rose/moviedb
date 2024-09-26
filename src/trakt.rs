// use oauth2::basic::BasicClient;
// use oauth2::reqwest::http_client;
// use oauth2::{
//     AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
//     Scope, TokenResponse, TokenUrl,
// };
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    header::HeaderMap,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::io::{stdin, stdout, Write};
// use trakt_rs::{Request, Response as Rewponse};

// use url::Url;

pub fn new(client_id: String) -> Result<(), Box<dyn Error>> {
    // let ctx = trakt_rs::Context {
    //     base_url: "https://api.trakt.tv",
    //     client_id: "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
    //     oauth_token: None,
    // };

    // let req = trakt_rs::api::movies::summary::Request {
    //     id: trakt_rs::smo::Id::Imdb("tt1877830".into()),
    // };
    // let http_req: http::Request<Vec<u8>> = req.try_into_http_request(ctx).unwrap();

    // let client = BasicClient::new(
    //     ClientId::new(
    //         "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82".to_string(),
    //     ),
    //     Some(ClientSecret::new(
    //         "ec4af5a8cb34d39055239adb5993aeadf7a7f22d941b6da7c574f0a8eabcb336".to_string(),
    //     )),
    //     AuthUrl::new("https://api.trakt.tv/oauth/authorize".to_string())?,
    //     // Some(TokenUrl::new("http://token".to_string())?)
    //     None,
    // )
    // .set_redirect_uri(RedirectUrl::new("urn:ietf:wg:oauth:2.0:oob".to_string())?);
    // let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // let (auth_url, csrf_token) = client
    //     .authorize_url(CsrfToken::new_random)
    //     // Set the desired scopes.
    //     .add_scope(Scope::new("read".to_string()))
    //     .add_scope(Scope::new("write".to_string()))
    //     // Set the PKCE code challenge.
    //     .set_pkce_challenge(pkce_challenge)
    //     .url();

    // println!("Browse to: {}", auth_url);

    let client = ClientBuilder::new().build()?;
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    // let authorization_url = client
    //     .get("https://api.trakt.tv/oauth/authorize")
    //     .query(&[
    //         ("response_type", "code"),
    //         (
    //             "client_id",
    //             "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
    //         ),
    //         ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
    //     ])
    //     .headers(headers.clone())
    //     .build()?
    //     .url()
    //     .to_string();

    // println!(
    //     "\nPlease visit the following url to authorize the application.\n{}\n",
    //     authorization_url
    // );

    // let mut auth_code = String::new();
    // print!("Please enter the auth code from the url: ");
    // let _ = stdout().flush();
    // stdin()
    //     .read_line(&mut auth_code)
    //     .expect("Did not enter a correct string");
    // if let Some('\n') = auth_code.chars().next_back() {
    //     auth_code.pop();
    // }
    // if let Some('\r') = auth_code.chars().next_back() {
    //     auth_code.pop();
    // }

    // let mut body = HashMap::new();
    // body.insert("code", auth_code.as_str());
    // body.insert(
    //     "client_id",
    //     "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
    // );
    // body.insert(
    //     "client_secret",
    //     "ec4af5a8cb34d39055239adb5993aeadf7a7f22d941b6da7c574f0a8eabcb336",
    // );
    // body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    // body.insert("grant_type", "authorization_code");

    // // // let get_token_request = client
    // // //     .post()
    // // //     .header("Content-Type", "application/json")
    // // //     .json(&body);

    // // // println!("{:#?}\n", get_token_request);

    // #[derive(Deserialize, Debug)]
    // struct TokenResponse {
    //     pub access_token: String,
    //     pub token_type: String,
    //     pub expires_in: u32,
    //     pub refresh_token: String,
    //     pub scope: String,
    //     pub created_at: u32,
    // }
    // // // let token_response = get_token_request.send()?.json::<TokenResponse>()?;
    // let token_response = send_trakt_request(
    //     &client,
    //     "https://api.trakt.tv/oauth/token",
    //     headers.clone(),
    //     Some(body),
    //     None,
    // )?
    // .json::<TokenResponse>()?;

    // println!("{:#?}\n", token_response);

    // let authorization_response = authorization_request.send().await?;
    // println!("{:#?}", authorization_response);

    // let request = client
    //     .request(Method::GET, Url::parse(&http_req.uri().to_string())?)
    //     .headers(http_req.headers().clone())
    //     .version(Version::HTTP_11);

    // println!("{:#?}\n", http_req);
    // println!("{:#?}\n", request);

    // Convert the HTTP response into a Trakt responss
    // let trakt_response =
    //     trakt_rs::api::movies::summary::Response::try_from_http_response(response).unwrap();

    // let ctx = trakt_rs::Context {
    //     base_url: "https://api.trakt.tv",
    //     client_id: "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
    //     oauth_token: Some("1ed5f14f365478d65dc630de28a38a9a355400f2c1936f24dd811c4001322069"),
    // };
    // let req = trakt_rs::api::search::text_query::Request {
    //     tp: trakt_rs::api::search::SearchType::MOVIE,
    //     query: "American".into(),
    //     pagination: trakt_rs::Pagination { page: 0, limit: 3 },
    // };
    // let http_req: http::Request<Vec<u8>> = req.try_into_http_request(ctx).unwrap();
    // println!("{:#?}\n", http_req);

    let mut search_headers = headers.clone();
    search_headers.insert(
        "trakt-api-key",
        "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82"
            .parse()
            .unwrap(),
    );
    search_headers.insert("trakt-api-version", "2".parse().unwrap());
    // search_headers.insert(
    //     "Authorization",
    //     "sfd270fcd07b301595070fbe6409c3f88af4f55c10313b603ab238f27f950638"
    //         .parse()
    //         .unwrap(),
    //     // format!("Bearer {}", token_response.access_token)
    //     //     .parse()
    //     //     .unwrap(),
    // );
    let query = vec![
        ("limit", "3"),
        ("page", "1"),
        ("query", "Allied"),
        ("extended", "full"),
        ("ratings", "70-100"),
        ("imdb_ratings", "7.0-10.0"),
    ];
    let search_response = send_trakt_request(
        &client,
        "https://api.trakt.tv/search/movie",
        search_headers,
        None,
        Some(&query),
    )?;

    let movies = search_response.text()?;
    println!("{:#}", json::parse(movies.as_str()).unwrap());

    // let trakt_response = trakt_rs::api::search::text_query::Response::try_from_http_response(search_response).unwrap();
    // trakt_rs::smo::Movie

    // println!("Movie: {:?}", trakt_response.0);
    // let response = request.send().await?;
    // println!("{:#?}", response);
    Ok(())
}

#[derive(Debug)]
struct MyError(String);
impl Error for MyError {}
impl Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}

fn send_trakt_request(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: Option<HashMap<&str, &str>>,
    query: Option<&[(&str, &str)]>,
) -> Result<Response, Box<dyn Error>> {
    let mut retry_attempts = 0;
    let mut retry_delay = 1;

    while retry_attempts < 2 {
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

        println!("{:#?}", request.try_clone().unwrap().build().unwrap());
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
        if response.status().as_u16() <= 204 {
            return Ok(response);
        } else if response.status().as_u16() <= 522 {
            retry_attempts += 1;
            println!("{retry_attempts}Retrying {}", response.status());
            std::thread::sleep(std::time::Duration::from_secs(retry_delay));
            retry_delay *= 2;
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Unrecognized status code {} when requesting from {}",
                    response.status(),
                    url
                ),
            )));
        }
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Maximum retries reached while requesting {}", url),
    )))
}
