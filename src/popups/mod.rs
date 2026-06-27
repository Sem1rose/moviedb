mod add_movie;
mod delete_movie;
mod edit_movie;
mod trakt_init;
mod tmdb_init;
mod omdb_init;
mod fetch_artworks;
mod out_of_box;

pub use add_movie::{AddMoviePopup, Phase as AddMoviePopupPhase};
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;
pub use trakt_init::{TraktInitPopup, Phase as TraktInitPopupPhase};
pub use tmdb_init::{TMDBInitPopup, Phase as TMDBInitPopupPhase};
pub use omdb_init::OMDBInitPopup;
pub use fetch_artworks::FetchArtworksPopup;
pub use out_of_box::OutOfBoxPopup;

pub enum Popups {
    AddMovie(AddMoviePopup),
    EditMovie(EditMoviePopup),
    DeleteMovie(DeleteMoviePopup),
    TraktInit(TraktInitPopup),
    TMDBInit(TMDBInitPopup),
    OMDBInit(OMDBInitPopup),
    FetchArtworks(FetchArtworksPopup),
    // OutOfBox(OutOfBoxPopup),
}
