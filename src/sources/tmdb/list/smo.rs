use serde::Deserialize;

use super::super::smo::SearchResult;

#[derive(Deserialize, Debug)]
pub struct ListDetails {
    pub created_by:     Option<String>,
    pub description:    String,
    pub favorite_count: usize,
    pub id:             u64,
    #[serde(default)]
    pub items:          Vec<SearchResult>,
    pub item_count:     usize,
    pub iso_639_1:      String,
    pub name:           String,
    pub poster_path:    Option<String>,
}
