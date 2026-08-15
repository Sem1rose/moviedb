mod add_movie;
mod advanced_filter;
mod delete_movie;
mod edit_movie;
mod fetch_movies;
mod omdb_init;
mod out_of_box;
mod punch_play_init;
mod tmdb_init;
mod trakt_init;

pub use add_movie::{AddMoviePopup, Phase as AddMoviePopupPhase};
pub use advanced_filter::AdvancedFilterPopup;
pub use delete_movie::DeleteMoviePopup;
pub use edit_movie::EditMoviePopup;
pub use fetch_movies::FetchMoviesPopup;
pub use omdb_init::OMDBInitPopup;
pub use out_of_box::OutOfBoxPopup;
pub use punch_play_init::{Phase as PunchPlayInitPopupPhase, PunchPlayInitPopup};
pub use tmdb_init::{Phase as TMDBInitPopupPhase, TMDBInitPopup};
pub use trakt_init::{Phase as TraktInitPopupPhase, TraktInitPopup};

pub enum Popups {
    AddMovie(Box<AddMoviePopup>),
    EditMovie(Box<EditMoviePopup>),
    DeleteMovie(Box<DeleteMoviePopup>),
    TraktInit(Box<TraktInitPopup>),
    PunchPlayInit(Box<PunchPlayInitPopup>),
    TMDBInit(Box<TMDBInitPopup>),
    OMDBInit(Box<OMDBInitPopup>),
    OutOfBox(Box<OutOfBoxPopup>),
    AdvancedFilter(Box<AdvancedFilterPopup>),
    FetchMovies(Box<FetchMoviesPopup>),
}

impl Popups {
    fn as_trait(&self) -> &dyn PopupTrait {
        match self {
            Popups::AddMovie(add_movie_popup) => &**add_movie_popup,
            Popups::EditMovie(edit_movie_popup) => &**edit_movie_popup,
            Popups::DeleteMovie(delete_movie_popup) => &**delete_movie_popup,
            Popups::TraktInit(trakt_init_popup) => &**trakt_init_popup,
            Popups::PunchPlayInit(punch_play_init_popup) => &**punch_play_init_popup,
            Popups::TMDBInit(tmdbinit_popup) => &**tmdbinit_popup,
            Popups::OMDBInit(omdbinit_popup) => &**omdbinit_popup,
            Popups::OutOfBox(out_of_box_popup) => &**out_of_box_popup,
            Popups::AdvancedFilter(advanced_filter_popup) => &**advanced_filter_popup,
            Popups::FetchMovies(fetch_movies_popup) => &**fetch_movies_popup,
        }
    }

    fn as_trait_mut(&mut self) -> &mut dyn PopupTrait {
        match self {
            Popups::AddMovie(add_movie_popup) => &mut **add_movie_popup,
            Popups::EditMovie(edit_movie_popup) => &mut **edit_movie_popup,
            Popups::DeleteMovie(delete_movie_popup) => &mut **delete_movie_popup,
            Popups::TraktInit(trakt_init_popup) => &mut **trakt_init_popup,
            Popups::PunchPlayInit(punch_play_init_popup) => &mut **punch_play_init_popup,
            Popups::TMDBInit(tmdbinit_popup) => &mut **tmdbinit_popup,
            Popups::OMDBInit(omdbinit_popup) => &mut **omdbinit_popup,
            Popups::OutOfBox(out_of_box_popup) => &mut **out_of_box_popup,
            Popups::AdvancedFilter(advanced_filter_popup) => &mut **advanced_filter_popup,
            Popups::FetchMovies(fetch_movies_popup) => &mut **fetch_movies_popup,
        }
    }

    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        self.as_trait().get_state()
    }

    pub fn update_next_frame(&self) -> bool {
        self.as_trait().update_next_frame()
    }

    pub fn update(&mut self) {
        self.as_trait_mut().update()
    }

    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        key_event_handler: &mut crate::key_event_handler::KeyEventHandler,
    ) {
        self.as_trait_mut().render(frame, key_event_handler)
    }
}

pub trait PopupTrait {
    fn get_state(&self) -> (Option<usize>, Option<usize>);
    fn update_next_frame(&self) -> bool;
    fn update(&mut self);
    fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        key_event_handler: &mut crate::key_event_handler::KeyEventHandler,
    );
}
