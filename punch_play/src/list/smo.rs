use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPoster {
    pub poster_path: String,
    pub tmdb_id:     u32,
    #[serde(alias = "type")]
    pub poster_type: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id:           u32,
    pub tmdb_id:      u32,
    #[serde(alias = "type")]
    pub item_type:    String,
    // pub is_anime: bool,
    pub title:        String,
    pub poster_path:  Option<String>,
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub added_at:     DateTime<Utc>,
    // pub runtime:      Option<usize>,
    // pub popularity:   Option<f64>,
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub release_date: DateTime<Utc>,
    pub watched:      bool,
}

#[derive(Deserialize, Debug)]
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
    #[serde(deserialize_with = "date_time_deserializer", default)]
    pub created_at:      DateTime<Utc>,
    pub items:           Option<Vec<ListItem>>,
}

// fn custom_deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     struct CustomVisitor;

//     impl<'de> serde::de::Visitor<'de> for CustomVisitor {
//         type Value = Option<NaiveDateTime>;
//         fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//             formatter.write_str("a datetime in the format %Y-%m-%dT%H:%M:%S%.fZ")
//         }

//         fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
//         where
//             E: serde::de::Error,
//         {
//             if value == "null" {
//                 Ok(None)
//             } else {
//                 NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ").map(Option::Some).map_err(E::custom)
//             }
//         }
//     }

//     deserializer.deser(CustomVisitor)
// }
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
