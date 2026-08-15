use chrono::NaiveDateTime;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPoster {
    pub poster_path: String,
    pub tmdb_id:     u32,
    #[serde(alias = "type")]
    pub poster_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    // pub id: String,
    pub tmdb_id:      u32,
    #[serde(alias = "type")]
    pub item_type:    String,
    // pub is_anime: bool,
    pub title:        String,
    pub poster_path:  String,
    pub added_at:     NaiveDateTime,
    pub runtime:      usize,
    pub popularity:   usize,
    pub release_date: NaiveDateTime,
    pub watched:      bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDetails {
    pub id:              u32,
    pub name:            String,
    pub description:     Option<String>,
    // pub is_public: bool,
    pub is_watchlist:    bool,
    // pub is_dynamic_list: bool,
    // pub external_source: Option<String>,
    // pub is_collaborative: bool,
    // pub owner_username: Option<String>,
    pub item_count:      usize,
    pub preview_posters: Option<Vec<PreviewPoster>>,
    pub created_at:      Option<NaiveDateTime>,
    pub items:           Option<Vec<ListItem>>,
}

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct FullListDetails {
//     pub id: u32,
//     pub name: String,
//     pub description: String,
//     // pub is_public: bool,
//     pub is_watchlist: bool,
//     // pub external_source: String,
//     // pub external_url: String,
//     // pub is_dynamic_list: bool,
//     pub item_count: usize,
//     pub created_at: NaiveDateTime,
//     // pub is_owner: bool,
//     // pub owner_id: String,
//     // pub owner_username: String,
//     // pub is_collaborative: bool,
//     // pub is_collaborator: bool,
//     // pub is_featured: bool,
//     // pub like_count: usize,
//     // pub is_liked: bool,
//     // pub comment_count: usize,
//     pub items: Vec<ListItem>,
//     // pub last_synced_at: NaiveDateTime,
//     // pub dynamic_list_last_full_synced_at: NaiveDateTime,
//     // pub last_sync_error: String,
//     // pub dynamic_list_sync_state: String
//     // "collaborators": [
//     //   {
//     //     "userId": String,
//     //     "username": String,
//     //     "avatarUrl": String
//     //   }
//     // ],
//     // "dynamicListFilters": {},
// }
