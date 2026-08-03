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
impl Into<TMDBMovieImagesResponse> for TMDBDetails {
    fn into(self) -> TMDBMovieImagesResponse {
        TMDBMovieImagesResponse {
            posters:   self
                .poster_path
                .and_then(|x| if x.is_empty() { None } else { Some(x) })
                .map(|x| vec![TMDBMovieImage { file_path: x }])
                .unwrap_or(vec![]),
            backdrops: self
                .backdrop_path
                .and_then(|x| if x.is_empty() { None } else { Some(x) })
                .map(|x| vec![TMDBMovieImage { file_path: x }])
                .unwrap_or(vec![]),
        }
    }
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

pub struct TMDBMovieDetails {}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBDetails {
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
    pub popularity:            f64,
    pub revenue:               u32,
    pub video:                 bool,
    pub origin_country:        Option<Vec<String>>,
    pub poster_path:           Option<String>,
    pub backdrop_path:         Option<String>,
    pub credits:               Option<TMDBCredits>,
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

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TMDBCredits {
    // pub id:                    u32,
    pub cast: Vec<Actor>,
    pub crew: Vec<Crew>,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct Actor {
    pub adult:                bool,
    pub gender:               usize,
    pub id:                   usize,
    pub known_for_department: String,
    pub name:                 String,
    pub original_name:        String,
    pub popularity:           f64,
    pub profile_path:         Option<String>,
    pub cast_id:              usize,
    pub character:            String,
    pub credit_id:            String,
    pub order:                usize,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct Crew {
    pub adult:                bool,
    pub gender:               usize,
    pub id:                   usize,
    pub known_for_department: String,
    pub name:                 String,
    pub original_name:        String,
    pub popularity:           f64,
    pub profile_path:         Option<String>,
    pub credit_id:            String,
    pub department:           String,
    pub job:                  String,
}
