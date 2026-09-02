use chrono::NaiveDate;
use itertools::Itertools;
use serde::{self, Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{collection::smo::CollectionDetails, movie::PaginatedResponse};

#[derive(Deserialize, Clone, Debug)]
pub struct MovieImage {
    // aspect_ratio: f32,
    // height: u32,
    // iso_639_1: String,
    pub file_path:    String,
    pub vote_average: f32,
    pub vote_count:   u32,
    // width: u32,
}
#[derive(Deserialize, Clone, Default, Debug)]
pub struct MovieImagesResponse {
    pub backdrops: Vec<MovieImage>,
    pub posters:   Vec<MovieImage>,
}
impl Into<MovieImagesResponse> for MovieDetails {
    fn into(mut self) -> MovieImagesResponse {
        let images = self.images.take().unwrap_or_default();
        MovieImagesResponse {
            posters:   match self.poster_path.clone() {
                Some(x) if x.len() > 0 => vec![MovieImage {
                    file_path:    x,
                    vote_average: 100.0,
                    vote_count:   10000,
                }],
                _ => vec![],
            }
            .into_iter()
            .chain(images.posters)
            .collect_vec(),
            backdrops: match self.backdrop_path.clone() {
                Some(x) if x.len() > 0 => vec![MovieImage {
                    file_path:    x,
                    vote_average: 100.0,
                    vote_count:   10000,
                }],
                _ => vec![],
            }
            .into_iter()
            .chain(images.backdrops)
            .collect_vec(),
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Default, Clone)]
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

#[derive(Deserialize, Default, Debug, Clone)]
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
    pub images:                Option<MovieImagesResponse>,
    #[serde(
        deserialize_with = "certificate_deserializer",
        alias = "release_dates",
        default
    )]
    pub certificate:           Option<String>,
    #[serde(deserialize_with = "recommendations_deserializer", default)]
    pub recommendations:       Option<Vec<u32>>,
    pub collection_details:    Option<CollectionDetails>,
    #[serde(
        deserialize_with = "user_interaction_deserializer",
        alias = "account_states",
        default
    )]
    pub user_interaction:      Option<UserInteraction>,
}
fn user_interaction_deserializer<'de, D>(d: D) -> Result<Option<UserInteraction>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).map(|value: Option<Value>| {
        value.map(|x| UserInteraction {
            favorite:  x["favorite"].as_bool().unwrap(),
            watchlist: x["watchlist"].as_bool().unwrap(),
            rating:    x["rated"]
                .as_object()
                .map(|y| y["value"].as_number().unwrap().as_u64())
                .flatten(),
        })
    })
}
fn certificate_deserializer<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ReleaseDate {
        certification: String,
        // descriptors: [],
        // iso_639_1:     String,
        // note: String,
        // release_date:  String,
        #[serde(alias = "type")]
        release_type:  usize,
    }
    #[derive(Deserialize)]
    struct ReleaseDatesResult {
        iso_3166_1:    String,
        release_dates: Vec<ReleaseDate>,
    }
    #[derive(Deserialize)]
    struct ReleaseDatesResponse {
        results: Vec<ReleaseDatesResult>,
    }

    Deserialize::deserialize(d).map(|value: Option<ReleaseDatesResponse>| {
        value
            .map(|x| {
                x.results
                    .into_iter()
                    .filter(|y| y.iso_3166_1 == "US")
                    .nth(0)
                    .map(|y| {
                        y.release_dates
                            .into_iter()
                            .filter(|z| z.release_type == 3)
                            .nth(0)
                            .map(|z| z.certification)
                    })
            })
            .flatten()
            .flatten()
    })
}
fn recommendations_deserializer<'de, D>(d: D) -> Result<Option<Vec<u32>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Recommendation {
        id: u32,
    }

    Deserialize::deserialize(d).map(|value: Option<PaginatedResponse<Recommendation>>| {
        value.map(|x| x.results.into_iter().map(|y| y.id).take(5).collect())
    })
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

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct UserInteraction {
    pub favorite:  bool,
    pub watchlist: bool,
    pub rating:    Option<u64>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Credits {
    // pub id:                    u32,
    pub cast: Vec<Person>,
    pub crew: Vec<Person>,
}
#[derive(Deserialize, Debug, Clone)]
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
