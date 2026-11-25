use serde::Deserialize;

use super::super::smo::SearchResult;

#[derive(Deserialize, Debug, Clone)]
pub struct CollectionDetails {
    pub id:            u64,
    pub name:          String,
    // pub original_language: String,
    // pub original_name: String,
    pub overview:      String,
    pub poster_path:   Option<String>,
    pub backdrop_path: Option<String>,
    pub parts:         Vec<SearchResult>,
}
