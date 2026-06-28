mod add_movie;
mod delete_movie;
mod edit_movie;
mod fetch_artworks;
mod omdb_init;
mod out_of_box;
mod tmdb_init;
mod trakt_init;

pub use add_movie::{AddMoviePopup, Phase as AddMoviePopupPhase};
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;
pub use fetch_artworks::FetchArtworksPopup;
pub use omdb_init::OMDBInitPopup;
pub use out_of_box::OutOfBoxPopup;
pub use tmdb_init::{Phase as TMDBInitPopupPhase, TMDBInitPopup};
pub use trakt_init::{Phase as TraktInitPopupPhase, TraktInitPopup};

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
