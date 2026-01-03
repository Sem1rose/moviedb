pub mod omdb_tokens;
pub mod tmdb_tokens;
pub mod trakt_tokens;
use serde::Deserialize;

pub use omdb_tokens::OMDBTokens;
pub use tmdb_tokens::TMDBTokens;
pub use trakt_tokens::TraktTokens;

#[derive(Deserialize)]
pub struct Credentials {
    pub trakt_client_id: String,
    pub trakt_client_secret: String,
    pub tmdb_access_token: String,
    pub omdb_key: String,
}
