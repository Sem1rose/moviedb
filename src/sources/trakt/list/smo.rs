use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::sources::trakt::smo::ListItem;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ListPrivacy {
    Private,
    Link,
    Friends,
    Public,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ListDetails {
    // allow_comments: bool,
    // comment_count: u32,
    // converted_to: Null,
    pub created_at:  DateTime<Utc>,
    pub description: String,
    // display_numbers: bool,
    pub ids:         ListIDs,
    pub images:      ListImages,
    pub item_count:  u32,
    // likes: u32,
    pub name:        String,
    pub privacy:     ListPrivacy,
    pub share_link:  String,
    // sort_by: String,
    // sort_how: String,
    // stale_origin: Null,
    pub total_count: u32,
    // #[serde(alias = "type")]
    // list_type: String,
    pub updated_at:  DateTime<Utc>,
    #[serde(default)]
    pub items:       Vec<ListItem>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct ListIDs {
    pub slug:  String,
    pub trakt: u64,
}
#[derive(Deserialize, Debug, Clone)]
pub struct ListImages {
    pub posters: Vec<String>,
}
