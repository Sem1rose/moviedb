use chrono::NaiveDate;
use serde::Deserialize;

use crate::smo::{Ids, Item, Ratings};

#[derive(Deserialize, Debug)]
pub struct MovieDetails {
    pub budget:                Option<u32>,
    pub certification:         Option<String>,
    pub country:               String,
    pub director:              String,
    pub fanart:                String,
    pub poster:                String,
    pub genres:                Vec<String>,
    pub ids:                   Ids,
    pub language:              String,
    pub overview:              String,
    pub ratings:               Ratings,
    pub runtime:               u32,
    pub released:              NaiveDate,
    pub revenue:               Option<u32>,
    pub title:                 String,
    pub year:                  u32,
    pub users_recommendations: Vec<Item>,
}
