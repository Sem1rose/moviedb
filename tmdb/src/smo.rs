use std::{error::Error, fmt::Display};

use serde::Deserialize;

pub use crate::movie::smo::*;

#[derive(Deserialize, Debug)]
pub struct RequestResponseError {
    pub status_code:    i32,
    pub status_message: String,
}
impl Error for RequestResponseError {}
impl Display for RequestResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.status_code, self.status_message)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct AccountDetails {
    // pub avatar: {
    //     gravatar: {
    //         pub hash: "00000000000000000"
    //     },
    //     tmdb: {
    //         pub avatar_path: null
    //     }
    // },
    pub id: u32,
    // pub iso_639_1:     String,
    // pub iso_3166_1:    String,
    // pub name:          String,
    // pub include_adult: bool,
    // pub username:      String,
}

// #[derive(Deserialize, Debug, Clone)]
// pub(crate) struct ConfigurationResponse {
//     // change_keys: Vec<String>,
//     pub images: ImagesConfiguration,
// }
// #[derive(Deserialize, Debug, Clone)]
// pub(crate) struct ImagesConfiguration {
//     pub base_url:       String,
//     pub poster_sizes:   Vec<String>, // w92 w154 w185 w342 w500 w780 original
//     pub backdrop_sizes: Vec<String>, // w300 w780 w1280 original
// }
