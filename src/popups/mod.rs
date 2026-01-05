mod add_movie;
mod edit_movie;
mod remove_movie;

pub use add_movie::AddMoviePopup;
pub use add_movie::Phase as AddMoviePopupPhase;
pub use edit_movie::EditMoviePopup;
pub use remove_movie::RemoveMoviePopup;

pub enum Popups {
    AddMovie(AddMoviePopup),
    EditMovie(EditMoviePopup),
    RemoveMovie(RemoveMoviePopup),
    // FetchArtwork,
    // TraktInit,
    // TMDBInit,
}
