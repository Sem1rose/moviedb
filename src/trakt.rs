// use oauth2::basic::BasicClient;
// use oauth2::reqwest::http_client;
// use oauth2::{
//     AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
//     Scope, TokenResponse, TokenUrl,
// };
use reqwest::blocking::ClientBuilder;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::io::{stdin, stdout, Write};
use trakt_rs::Request;

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
    let authorization_url = client
        .get("https://api.trakt.tv/oauth/authorize")
        .query(&[
            ("response_type", "code"),
            (
                "client_id",
                "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
            ),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
        ])
        .header("Content-Type", "application/json")
        .build()?
        .url()
        .to_string();

    println!(
        "Please visit the following url to authorize the application.\n{}\n",
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
    body.insert(
        "client_id",
        "ca62495662f16852d7e6da10004b6f301928cb900b5bb20773628456054ebe82",
    );
    body.insert(
        "client_secret",
        "ec4af5a8cb34d39055239adb5993aeadf7a7f22d941b6da7c574f0a8eabcb336",
    );
    body.insert("redirect_uri", "urn:ietf:wg:oauth:2.0:oob");
    body.insert("grant_type", "authorization_code");

    let get_token_request = client
        .post("https://api.trakt.tv/oauth/token")
        .header("Content-Type", "application/json")
        .json(&body);

    println!("{:#?}\n", get_token_request);

    #[derive(Deserialize, Debug)]
    struct TokenResponse {
        pub access_token: String,
        pub token_type: String,
        pub expires_in: u32,
        pub refresh_token: String,
        pub scope: String,
        pub created_at: u32,
    }
    let token_response = get_token_request.send()?.json::<TokenResponse>()?;

    println!("{:#?}\n", token_response);

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

    // println!("Movie: {:?}", trakt_response.0);
    // let response = request.send().await?;
    // println!("{:#?}", response);
    Ok(())
}
