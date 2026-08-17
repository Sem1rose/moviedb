use chrono::NaiveDate;
use serde::{self, Deserialize, Deserializer, Serialize};

use crate::collection::smo::CollectionDetails;

#[derive(Deserialize, Clone, Default)]
pub(crate) struct MovieImagesResponse {
    pub backdrops: Vec<MovieImage>,
    pub posters:   Vec<MovieImage>,
}
impl Into<MovieImagesResponse> for &MovieDetails {
    fn into(self) -> MovieImagesResponse {
        MovieImagesResponse {
            posters:   self
                .poster_path
                .clone()
                .and_then(|x| if x.is_empty() { None } else { Some(x) })
                .map(|x| {
                    vec![MovieImage {
                        file_path:    x,
                        vote_average: 10.0,
                        vote_count:   1000,
                    }]
                })
                .unwrap_or(vec![]),
            backdrops: self
                .backdrop_path
                .clone()
                .and_then(|x| if x.is_empty() { None } else { Some(x) })
                .map(|x| {
                    vec![MovieImage {
                        file_path:    x,
                        vote_average: 10.0,
                        vote_count:   1000,
                    }]
                })
                .unwrap_or(vec![]),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub(crate) struct MovieImage {
    // aspect_ratio: f32,
    // height: u32,
    // iso_639_1: String,
    pub file_path:    String,
    pub vote_average: f32,
    pub vote_count:   u32,
    // width: u32,
}

#[derive(Deserialize, Debug, PartialEq, Default)]
pub struct SearchResult {
    pub id:                u32,
    #[serde(alias = "name")]
    pub title:             String,
    #[serde(alias = "original_name")]
    pub original_title:    String,
    pub adult:             bool,
    pub genre_ids:         Vec<u32>,
    pub original_language: String,
    pub overview:          String,
    pub popularity:        f64,
    pub rating:            Option<f64>,
    pub poster_path:       Option<String>,
    pub backdrop_path:     Option<String>,
    #[serde(deserialize_with = "custom_deserialize")]
    pub release_date:      NaiveDate,
    // pub video:             bool,
    pub vote_average:      Option<f64>,
    pub vote_count:        u32,
    pub media_type:        Option<String>,
}
fn custom_deserialize<'de, D>(d: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d)
        .map(|x: &str| NaiveDate::parse_from_str(x, "%Y-%m-%d").unwrap_or_default())
}

#[derive(Deserialize, Default, Debug)]
pub struct MovieDetails {
    pub id:                    u32,
    pub imdb_id:               String,
    pub release_date:          String,
    pub title:                 String,
    pub original_title:        String,
    pub tagline:               String,
    pub overview:              String,
    pub genres:                Vec<Genre>,
    pub vote_count:            u32,
    pub vote_average:          f64,
    pub original_language:     String,
    pub runtime:               u32,
    pub homepage:              Option<String>,
    pub status:                String,
    pub adult:                 bool,
    pub belongs_to_collection: Option<Collection>,
    pub budget:                u32,
    pub popularity:            f64,
    pub revenue:               u32,
    // pub video:                 bool,
    pub origin_country:        Option<Vec<String>>,
    pub poster_path:           Option<String>,
    pub backdrop_path:         Option<String>,
    pub credits:               Option<Credits>,
    pub certificate:           Option<String>,
    pub recommendations:       Option<Vec<u32>>,
    pub collection_details:    Option<CollectionDetails>,
    pub user_interaction:      Option<UserInteraction>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Collection {
    pub id:            u32,
    pub name:          String,
    pub poster_path:   Option<String>,
    pub backdrop_path: Option<String>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Genre {
    pub id:   u32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct UserInteraction {
    pub favorite:  bool,
    pub watchlist: bool,
    pub rating:    Option<u64>,
}
#[derive(Deserialize, Debug)]
pub struct Credits {
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
