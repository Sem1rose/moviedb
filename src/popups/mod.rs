mod add_movie;
mod delete_movie;
mod edit_movie;
mod tmdb_init;

pub use add_movie::{AddMoviePopup, Phase as AddMoviePopupPhase};
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;
pub use tmdb_init::{Phase as TMDBInitPopupPhase, TMDBInitPopup};

pub enum Popups {
    AddMovie(AddMoviePopup),
    EditMovie(EditMoviePopup),
    DeleteMovie(DeleteMoviePopup),
    TMDBInit(TMDBInitPopup),
    // FetchArtwork,
    // TraktInit,
}
