mod add_movie;
mod delete_movie;
mod edit_movie;
mod tmdb_init;
mod trakt_init;
mod fetch_artworks;
mod omdb_init;

pub use add_movie::{AddMoviePopup, Phase as AddMoviePopupPhase};
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;
pub use tmdb_init::{TMDBInitPopup, Phase as TMDBInitPopupPhase};
pub use omdb_init::OMDBInitPopup;
pub use trakt_init::{TraktInitPopup, Phase as TraktInitPopupPhase};
pub use fetch_artworks::FetchArtworksPopup;

pub enum Popups {
    AddMovie(AddMoviePopup),
    EditMovie(EditMoviePopup),
    DeleteMovie(DeleteMoviePopup),
    TMDBInit(TMDBInitPopup),
    OMDBInit(OMDBInitPopup),
    TraktInit(TraktInitPopup),
    FetchArtworks(FetchArtworksPopup),
}
