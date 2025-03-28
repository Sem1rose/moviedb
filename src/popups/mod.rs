pub mod add_movie;
pub mod edit_movie;
pub mod error;
pub mod fetch_artworks;
pub mod remove_movie;
pub mod tmdb_init;
pub mod trakt_init;

#[derive(Clone, PartialEq)]
pub enum Popups {
    FetchArtwork,
    AddMovie,
    EditMovie,
    RemoveMovie,
    Error,
    TraktInit,
    TMDBInit,
}
