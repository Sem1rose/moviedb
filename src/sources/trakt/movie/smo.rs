use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};

fn date_serializer<'de, D>(d: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).map(|x: Option<&str>| {
        x.map(|y| NaiveDate::parse_from_str(y, "%Y-%m-%d").ok())
            .flatten()
    })
}

#[derive(Deserialize, Debug, Clone)]
pub struct ListItem {
    // id: u64,
    pub movie:     MovieDetails,
    #[serde(alias = "listed_at", alias = "rated_at")]
    pub added_at:  DateTime<Utc>,
    #[serde(alias = "rating")]
    pub my_rating: Option<f64>,
    pub notes:     Option<String>,
    #[serde(default)]
    pub rank:      u64,
    // #[serde(alias = "type")]
    // item_type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HistoryItem {
    // action: String,
    pub id:         u64,
    pub movie:      MovieDetails,
    // #[serde(alias = "type")]
    // item_type: String,
    pub watched_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MovieDetails {
    pub title:         String,
    // pub year: Option<usize>,
    pub ids:           IDs,
    pub tagline:       String,
    pub overview:      String,
    #[serde(deserialize_with = "date_serializer")]
    pub released:      Option<NaiveDate>,
    pub runtime:       Option<u64>,
    // pub country: String,
    // pub trailer: Option<String>,
    // pub homepage: String,
    pub status:        String,
    pub rating:        Option<f64>,
    pub votes:         Option<u64>,
    // pub comment_count: u64,
    // #[serde(deserialize_with = "date_serializer")]
    // pub updated_at: Option<NaiveDate>,
    pub language:      String,
    // pub languages: Vec<String>,
    // pub available_translations: Vec<String>,
    pub genres:        Vec<String>,
    pub certification: Option<String>,
    pub images:        MovieImages,
    // pub subgenres: Vec<String>,
    // pub after_credits: bool,
    // pub during_credits: bool,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub(crate) struct MovieImages {
    pub fanart: Vec<String>,
    pub poster: Vec<String>,
    pub banner: Vec<String>,
    // logo: Vec<String>,
    // clearart: Vec<String>,
    // thumb: Vec<String>,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct IDs {
    pub slug:  String,
    pub trakt: u64,
    pub imdb:  String,
    pub tmdb:  u64,
}
