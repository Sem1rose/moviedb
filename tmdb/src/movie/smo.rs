use serde::{self, Deserialize, Serialize};

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

#[derive(Deserialize, Clone)]
pub(crate) struct TMDBMovieImagesResponse {
    pub backdrops: Vec<TMDBMovieImage>,
    pub posters:   Vec<TMDBMovieImage>,
}
impl Into<TMDBMovieImagesResponse> for TMDBMovieDetails {
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

#[derive(Deserialize, Clone)]
pub(crate) struct TMDBMovieImage {
    // aspect_ratio: f32,
    // height: u32,
    // iso_639_1: String,
    pub file_path: String,
    // vote_average: f32,
    // vote_count: u32,
    // width: u32,
}

#[derive(Deserialize)]
pub struct TMDBCollectionDetails {
    pub id: u32,
    pub name: String,
    // pub original_language: String,
    // pub original_name: String,
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<TMDBSearchResult>
}

#[derive(Deserialize, Default, Debug)]
pub struct TMDBMovieDetails {
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
    pub certificate:           Option<String>,
    pub recommendations:       Option<Vec<u32>>,
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

#[derive(Deserialize, Debug)]
pub struct TMDBCredits {
    // pub id:                    u32,
    pub cast: Vec<Person>,
    pub crew: Vec<Person>,
}
#[derive(Deserialize, Debug)]
pub struct Person {
    pub id:           u32,
    // pub adult:        bool,
    pub gender:       usize,
    // pub known_for_department: String,
    pub name:         String,
    // pub original_name:        String,
    // pub popularity:           f64,
    pub profile_path: Option<String>,

    // pub credit_id:            String,

    // actors
    pub cast_id:    Option<usize>,
    pub character:  Option<String>,
    pub order:      Option<usize>,
    // crew
    pub job:        Option<String>,
    pub department: Option<String>,
}
