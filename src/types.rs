use crate::{
    tmdb::{RequestResponseError, TMDBDetailsResponse},
    trakt::{TokenResponseError, TraktDetailsResponse},
};
use serde::Deserialize;

pub type Result<T> = color_eyre::Result<T, Errors>;
pub type OptionalResult<T> = color_eyre::Result<T, Option<Errors>>;

#[allow(clippy::upper_case_acronyms)]
#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub enum Rating {
    Trakt(f64, u32),
    TMDB(f64, u32),
}

#[derive(Debug, thiserror::Error)]
pub enum Errors {
    #[error("TMDB request error: {0}")]
    TMDBRequest(#[from] RequestResponseError),

    #[error("Trakt request error: {0}")]
    TraktRequest(#[from] TokenResponseError),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("cocoon encryption error: {0}")]
    Encryption(#[from] cocoon::Error),

    #[error("image error: {0}")]
    Image(#[from] image::error::ImageError),

    #[error("ratatui_image error: {0}")]
    EncodeImage(#[from] ratatui_image::errors::Errors),

    #[error("{0}")]
    EyreReport(#[from] color_eyre::Report),

    #[error("error: {0}")]
    Other(String),
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub struct MovieID {
    pub tmdb: u32,
    pub imdb: String,
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub id: MovieID,
    pub name: String,
    pub year: String,
    pub user_rating: f64,
    pub language: String,
    pub ratings: [Rating; 2],
    pub genres: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<u32>,
    pub overview: String,
    pub runtime: u32,
    pub released: bool,
    pub tagline: String,
    pub trailer: Option<String>,
    // pub finished_at: String,
}

impl Movie {
    pub fn from(movie_details: TMDBDetailsResponse, user_rating: f64) -> Self {
        let mut collection: Option<String> = None;
        let mut collection_id: Option<u32> = None;
        if movie_details.belongs_to_collection.is_some() {
            collection = Some(movie_details.belongs_to_collection.clone().unwrap().name);
            collection_id = Some(movie_details.belongs_to_collection.clone().unwrap().id);
        }

        Self {
            name: movie_details.title,
            user_rating,
            ratings: [
                Rating::TMDB(movie_details.vote_average, movie_details.vote_count),
                Rating::Trakt(0.0, 0),
            ],
            year: movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
            language: movie_details.original_language,
            id: MovieID {
                tmdb: movie_details.id,
                imdb: movie_details.imdb_id,
            },
            genres: movie_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview: movie_details.overview,
            collection,
            collection_id,
            runtime: movie_details.runtime,
            released: movie_details.status == "Released",
            tagline: movie_details.tagline,
            trailer: None,
            // finished_at: "".into(),
        }
    }

    pub fn add_trakt_details(mut self, trakt_details: TraktDetailsResponse) -> Self {
        self.ratings[1] = Rating::Trakt(trakt_details.rating, trakt_details.votes);
        self.trailer = trakt_details.trailer;

        self
    }
}
