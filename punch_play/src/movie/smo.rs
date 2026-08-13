use serde::Deserialize;

#[derive(PartialEq, Deserialize, Debug, Default)]
pub(crate) struct PunchPlaySearchResponse {
    pub items: Vec<PunchPlaySearchResult>,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PunchPlaySearchResult {
    // pub id:                String,
    pub tmdb_id:           u32,
    // pub type:              String,
    // pub is_anime:             bool,
    pub year:              usize,
    pub category:          String,
    pub name:              String,
    pub overview:          String,
    pub poster_url:        String,
    pub backdrop_url:      String,
    pub community_rating:  f64,
    pub release_date:      String,
    // pub poster_path:          Option<String>,
    // pub backdrop_path:        Option<String>,
    pub popularity:        Option<f64>,
    pub runtime_minutes:   Option<u32>,
    pub genres:            Vec<String>,
    pub age_rating:        Option<String>,
    pub original_language: String,
    // pub poster_color:         String,
    // pub poster_color_is_dark: bool,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PunchPlayDetailsResponse {
    pub title:            PunchPlayDetailsResponseTitle,
    pub interaction:      Option<Interaction>,
    pub watch_history:    Vec<WatchHistory>,
    pub community_rating: CommunityRating,
    pub external_ratings: Option<ExternalRatings>,
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PunchPlayDetailsResponseTitle {
    pub tmdb_id:           u32,
    // pub type:              String,
    // pub is_anime:             bool,
    pub year:              usize,
    pub category:          String,
    pub name:              String,
    pub overview:          String,
    pub poster_url:        String,
    pub backdrop_url:      String,
    pub directors:         Option<Vec<Person>>,
    pub cast:              Option<Vec<Person>>,
    // pub writers:           Option<Vec<Person>>,
    pub community_rating:  f64,
    pub genres:            Vec<String>,
    pub tagline:           String,
    pub release_date:      String,
    pub original_language: String,
    pub status:            String,
    pub age_rating:        Option<String>,
    pub runtime_minutes:   u32,
    // pub poster_path:          Option<String>,
    // pub backdrop_path:        Option<String>,
    pub popularity:        Option<f64>,
    // pub poster_color:         String,
    // pub poster_color_is_dark: bool,
    pub recommendations:   Option<Vec<Recommendation>>,
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id:          usize,
    pub name:        String,
    pub character:   Option<String>,
    pub profile_url: Option<String>, // `/placeholder.svg` must be interpreted as None
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub tmdb_id:          u32,
    pub name:             String,
    pub year:             usize,
    pub overview:         String,
    pub poster_url:       String,
    pub backdrop_url:     String,
    pub community_rating: f64,
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub watched_at:   Option<String>,
    pub rating:       Option<f64>,
    pub is_favourite: bool,
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatchHistory {
    pub id:         usize,
    pub watched_at: String,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct CommunityRating {
    pub average: Option<f64>,
    pub count:   usize,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct ExternalRatings {
    pub ratings: Vec<ExternalRating>,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct ExternalRating {
    // pub url: Option<String|usize>,
    // pub score:  Option<u32>,
    pub value:  Option<f64>,
    pub votes:  Option<u32>,
    pub source: Option<String>,
}
