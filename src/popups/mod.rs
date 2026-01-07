mod add_movie;
mod delete_movie;
mod edit_movie;

pub use add_movie::AddMoviePopup;
pub use add_movie::Phase as AddMoviePopupPhase;
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;

pub enum Popups {
    AddMovie(AddMoviePopup),
    EditMovie(EditMoviePopup),
    DeleteMovie(DeleteMoviePopup),
    // FetchArtwork,
    // TraktInit,
    // TMDBInit,
}
