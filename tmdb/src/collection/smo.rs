use serde::Deserialize;

use crate::smo::SearchResult;

#[derive(Deserialize, Debug)]
pub struct CollectionDetails {
    pub id:            u32,
    pub name:          String,
    // pub original_language: String,
    // pub original_name: String,
    pub overview:      String,
    pub poster_path:   Option<String>,
    pub backdrop_path: Option<String>,
    pub parts:         Vec<SearchResult>,
}
