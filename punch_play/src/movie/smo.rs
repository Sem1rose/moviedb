use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};

fn naive_date_deserializer<'de, D>(d: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).and_then(|value: Option<&str>| {
        value.map_or(Ok(Default::default()), |value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(serde::de::Error::custom)
        })
    })
}

fn date_time_deserializer<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).and_then(|value: Option<&str>| {
        value.map_or(Ok(Default::default()), |value| {
            DateTime::parse_from_rfc3339(value)
                .map(|x| x.with_timezone(&Utc))
                .map_err(serde::de::Error::custom)
        })
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaginatedResponse<T> {
    pub items:       Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct SearchResponse {
    pub items: Vec<ItemDetails>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetails {
    pub title:            ItemDetails,
    pub interaction:      Option<Interaction>,
    pub watch_history:    Vec<WatchHistory>,
    pub community_rating: CommunityRating,
    pub external_ratings: Option<ExternalRatings>,
}
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetails {
    pub tmdb_id:           u32,
    pub year:              usize,
    pub category:          String,
    #[serde(alias = "type")]
    pub item_type:         String,
    pub name:              String,
    pub overview:          String,
    pub poster_url:        Option<String>,
    pub backdrop_url:      Option<String>,
    pub community_rating:  f64,
    #[serde(deserialize_with = "naive_date_deserializer", default)]
    pub release_date:      NaiveDate,
    pub popularity:        Option<f64>,
    pub runtime_minutes:   u32,
    pub genres:            Vec<String>,
    pub age_rating:        Option<String>,
    pub original_language: String,

    pub tagline: Option<String>,
    pub status:  Option<String>,

    // pub is_anime:             bool,
    // pub writers:           Option<Vec<Person>>,

    pub poster_path:          Option<String>,
    pub backdrop_path:        Option<String>,
    // pub poster_color:         String,
    // pub poster_color_is_dark: bool,
    pub cast:            Option<Vec<Person>>,
    pub directors:       Option<Vec<Person>>,
    pub recommendations: Option<Vec<Recommendation>>,
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
    pub id:         u32,
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub watched_at: DateTime<Utc>,
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

// #[derive(Deserialize, Clone)]
// struct RatingsRatingStats {
//     1: usize,
//     2: usize,
//     3: usize,
//     4: usize,
//     5: usize,
//     6: usize,
//     7: usize,
//     8: usize,
//     9: usize,
//     10: usize,
// }

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub tmdb_id:      u32,
    // pub poster_path:   Option<String>,
    // pub backdrop_path: Option<String>,
    // pub season:        Option<usize>,
    // pub show_tmdb_id:  Option<u32>,
    // pub episode:       Option<usize>,
    // pub episode_title: Option<String>,
    // pub id: u32,
    pub is_favourite: Option<bool>,
    #[serde(alias = "type")]
    pub kind:         String,
    // pub mediaSource: Option<String>,
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub rated_at:     DateTime<Utc>,
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub watched_at:   DateTime<Utc>,
    pub rating:       Option<usize>,
    // pub scope:        Option<String>,
    // pub source_id: Option<usize>,
    // pub still_path:    Option<String>,
    pub title:        Option<String>,
    pub year:         Option<u32>,
}
#[derive(Deserialize, Clone)]
pub struct RatingsScopes {
    pub episode: usize,
    pub season:  usize,
    pub title:   usize,
}
#[derive(Deserialize, Clone)]
pub struct RatingsCounts {
    pub all:    usize,
    pub anime:  usize,
    pub movies: usize,
    // pub ratings: RatingsRatingStats,
    pub scopes: RatingsScopes,
    pub shows:  usize,
}
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RatingsResponse {
    pub counts:    RatingsCounts,
    pub has_more:  bool,
    pub items:     Vec<HistoryItem>,
    pub page:      usize,
    pub page_size: usize,
    pub total:     usize,
}
