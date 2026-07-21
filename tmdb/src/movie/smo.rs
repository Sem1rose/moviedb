use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Debug, Default)]
pub(crate) struct TMDBSearchResponse {
    // page: u64,
    pub results: Vec<TMDBSearchResult>,
    // total_pages: u64,
    // total_results: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct TMDBSearchResult {
    pub adult:             bool,
    pub backdrop_path:     Option<String>,
    pub genre_ids:         Vec<u64>,
    pub id:                u32,
    pub original_language: String,
    pub original_title:    String,
    pub overview:          String,
    pub popularity:        f64,
    pub poster_path:       Option<String>,
    pub release_date:      Option<String>,
    pub title:             String,
    pub video:             bool,
    pub vote_average:      Option<f64>,
    pub vote_count:        u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub(crate) struct TMDBMovieImagesResponse {
    pub backdrops: Vec<TMDBMovieImage>,
    pub posters:   Vec<TMDBMovieImage>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub(crate) struct TMDBMovieImage {
    // aspect_ratio: f32,
    // height: u32,
    // iso_639_1: String,
    pub file_path: String,
    // vote_average: f32,
    // vote_count: u32,
    // width: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBDetailsResponse {
    pub id:                    u32,
    pub imdb_id:               String,
    pub release_date:          String,
    pub title:                 String,
    pub original_title:        String,
    pub tagline:               String,
    pub overview:              String,
    pub genres:                Vec<TMDBGenre>,
    pub vote_count:            u32,
    pub vote_average:          f64,
    pub original_language:     String,
    pub runtime:               u32,
    pub homepage:              Option<String>,
    pub status:                String,
    pub adult:                 bool,
    pub belongs_to_collection: Option<TMDBCollection>,
    pub budget:                u32,
    pub popularity:            f32,
    pub revenue:               u32,
    pub video:                 bool,
    pub poster_path:           Option<String>,
    pub backdrop_path:         Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TMDBCollection {
    pub id:            u32,
    pub name:          String,
    pub poster_path:   Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TMDBGenre {
    pub id:   u32,
    pub name: String,
}
