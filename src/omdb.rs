use crate::config::config_omdb::OMDBConfig;
// use log::{debug, error, trace};
use anyhow::Context;
use reqwest::blocking::{ClientBuilder, RequestBuilder};
use serde::Deserialize;
use std::{error::Error, fmt::Display};

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

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Default, Clone)]
pub struct OMDBDetailsResponse {
    // pub Title: String,
    // pub Year: String,
    // pub Rated: String,
    // pub Released: String,
    // pub Runtime: String,
    // pub Genre: String,
    pub Director: String,
    // pub Writer: String,
    pub Actors: String,
    // pub Plot: String,
    // pub Language: String,
    // pub Country: String,
    // pub Awards: String,
    // pub Metascore: String,
    pub imdbRating: String,
    pub imdbVotes: String,
    // pub imdbID: String,
    // pub Type: String,
    // pub Result: String,
}

pub fn get_movie_details(
    omdb_config: &OMDBConfig,
    imdb_id: &str,
) -> anyhow::Result<OMDBDetailsResponse> {
    let client = ClientBuilder::new().build()?;

    let query = [
        ("apikey", omdb_config.key()),
        ("i", imdb_id),
        ("type", "movie"),
    ];
    let mut request: RequestBuilder;
    request = client.get("http://www.omdbapi.com");
    request = request.query(&query);

    let response = request.send()?;
    if response.status().as_u16() != 200 {
        return Err::<_, anyhow::Error>(match response.json::<DetailsResponseError>() {
            Ok(err) => err.into(),
            Err(err) => err.into(),
        })
        .context("Error while requesting from the omdb API");
    }

    response
        .json::<OMDBDetailsResponse>()
        .context("Couldn't parse response")
}
