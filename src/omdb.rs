use crate::{config::config_omdb::OMDBConfig, types::*};
// use log::{debug, error, trace};
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
    pub Metascore: String,
    pub imdbRating: String,
    pub imdbVotes: String,
    // pub imdbID: String,
    // pub Type: String,
    // pub Result: String,
}

/*
"Title": "El Camino",
  "Year": "2019",
  "Rated": "TV-MA",
  "Released": "11 Oct 2019",
  "Runtime": "122 min",
  "Genre": "Crime, Drama, Thriller",
  "Director": "Vince Gilligan",
  "Writer": "Vince Gilligan",
  "Actors": "Aaron Paul, Jonathan Banks, Matt Jones",
  "Plot": "Finally free from torture and slavery at the hands of Tod's uncle Jack, and from Mr. White, Jesse must escape demons from his past. He's on the run from a police manhunt, with his only hope of escape being Saul Goodman's hoover guy, Ed Galbraith. A man who for the right price, can give you a new identity and a fresh start. Jesse is racing against the clock, with help from his crew, avoiding capture to get enough money together to buy a 'new dust filter for his Hoover MaxExtract PressurePro model', a new life.",
  "Language": "English, Spanish",
  "Country": "United States",
  "Awards": "Nominated for 4 Primetime Emmys. 4 wins & 24 nominations total",
  "Poster": "https://m.media-amazon.com/images/M/MV5BYTYxMjI2YzUtODQ5Mi00M2JmLTlmNzItOTlkM2MyM2ExM2RlXkEyXkFqcGc@._V1_SX300.jpg",
  "Ratings": [
    {
      "Source": "Internet Movie Database",
      "Value": "7.3/10"
    },
    {
      "Source": "Rotten Tomatoes",
      "Value": "92%"
    },
    {
      "Source": "Metacritic",
      "Value": "72/100"
    }
  ],
  "Metascore": "72",
  "imdbRating": "7.3",
  "imdbVotes": "324,161",
  "imdbID": "tt9243946",
  "Type": "movie",
  "DVD": "N/A",
  "BoxOffice": "N/A",
  "Production": "N/A",
  "Website": "N/A",
  "Response": "True"
   */

pub fn get_movie_details(omdb_config: &OMDBConfig, imdb_id: &str) -> Result<OMDBDetailsResponse> {
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
        let result = response.json::<DetailsResponseError>();
        if let Ok(error) = result {
            return Err(Errors::OMDBRequest(error));
        } else {
            return Err(Errors::Reqwest(result.unwrap_err()));
        }
    }

    response
        .json::<OMDBDetailsResponse>()
        .map_err(Errors::Reqwest)
}
