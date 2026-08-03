use std::{error::Error, fmt::Display};

use anyhow::{Context, anyhow};
use reqwest::blocking::{ClientBuilder, RequestBuilder};
use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct DetailsResponseError {
    // Result: String,
    Error: String,
}
impl Error for DetailsResponseError {}
impl Display for DetailsResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.Error)
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OMDBDetailsResponse {
    pub title:       String,
    pub year:        String,
    pub rated:       String,
    pub released:    String,
    pub runtime:     String,
    pub genre:       String,
    pub director:    String,
    pub writer:      String,
    pub actors:      String,
    pub plot:        String,
    pub language:    String,
    pub country:     String,
    pub awards:      String,
    // pub metascore: String,
    #[serde(rename = "imdbRating")]
    pub imdb_rating: String,
    #[serde(rename = "imdbVotes")]
    pub imdb_votes:  String,
    // pub imdb_iD: String,
    // pub type: String,
}

pub fn get_movie_details(omdb_key: &str, imdb_id: &str) -> anyhow::Result<OMDBDetailsResponse> {
    let client = ClientBuilder::new().build()?;

    let query = [("apikey", omdb_key), ("i", imdb_id), ("type", "movie")];
    let mut request: RequestBuilder;
    request = client.get("http://www.omdbapi.com");
    request = request.query(&query);

    let response = request.send()?;
    if !response.status().is_success() {
        return Err(match response.json::<DetailsResponseError>() {
            Ok(err) => err.into(),
            Err(_) => anyhow!(""),
        })
        .context("Error while requesting from the omdb API");
    }

    response
        .json::<OMDBDetailsResponse>()
        .context("Couldn't parse response")
}
