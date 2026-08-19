use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Rating {
    pub rating: f64,
    pub votes: usize,
}

#[derive(Deserialize, Debug)]
pub struct Ratings {
    pub imdb: Rating,
    // pub simkl: Rating,
}

#[derive(Deserialize, Debug)]
pub struct Ids {
    pub simkl: u32,
    pub slug: String,
    pub imdb: Option<String>,
    pub letterboxd: Option<String>,
    pub tmdb: Option<String>,
    pub traktslug: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub ids: Ids,
    pub poster: String,
    pub title: String,
    #[serde(alias = "type")]
    pub item_type: String,
    pub year: u32
}

#[derive(Deserialize, Debug)]
pub struct MovieDetails {
    pub budget: Option<u32>,
    pub certification: String,
    pub country: String,
    pub director: String,
    pub fanart: String,
    pub poster: String,
    pub genres: Vec<String>,
    pub ids: Ids,
    pub language: String,
    pub overview: String,
    pub ratings: Ratings,
    pub runtime: u32,
    pub released: NaiveDate,
    pub revenue: Option<u32>,
    pub title: String,
    pub year: u32,
    pub users_recommendations: Vec<Item>,
}