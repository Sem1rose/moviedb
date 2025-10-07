use crate::{
    app::App,
    custom::helpers::center_rect,
    image_backends::{ratatui_image::RatatuiImage, ImageBackend},
    popups::{
        add_movie::AddMoviePopup, edit_movie::EditMoviePopup, fetch_artworks::FetchArtworksPopup,
        remove_movie::RemoveMoviePopup, tmdb_init::TMDBInitPopup, trakt_init::TraktInitPopup,
        Popups,
    },
    screens::{init_screen::InitScreen, main_screen::MainScreen, Screens},
    types::*,
};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};

impl Default for Box<dyn ImageBackend> {
    fn default() -> Self {
        Box::new(RatatuiImage::new())
    }
}

#[derive(Default)]
pub struct Drawer {
    pub should_quit: bool,

    pub(crate) image_backend: Box<dyn ImageBackend>,

    pub(crate) throbber_state: throbber_widgets_tui::ThrobberState,

    pub(crate) previous_screen: Option<Screens>,
    pub(crate) current_screen: Screens,
    pub(crate) active_popup: Option<Popups>,

    pub(crate) main_screen: MainScreen,
    pub(crate) init_screen: InitScreen,

    pub(crate) add_movie_popup: AddMoviePopup,
    pub(crate) edit_movie_popup: EditMoviePopup,
    pub(crate) remove_movie_popup: RemoveMoviePopup,
    pub(crate) fetch_artwork_popup: FetchArtworksPopup,
    pub(crate) tmdb_init_popup: TMDBInitPopup,
    pub(crate) trakt_init_popup: TraktInitPopup,

    pub(crate) error_popup_error: String,
}

const MINTERMSIZE: [u32; 2] = [80, 22];
impl Drawer {
    pub fn render_app(&mut self, frame: &mut Frame, app: &mut App, frame_time: f64) -> Result<()> {
        self.check_term_size(frame);
        self.check_popups(app)?;
        self.image_backend.update();

        self.draw_current_screen(frame, app)?;

        if self.active_popup.is_some() {
            self.draw_popup(frame, app)?;
        }

        frame.render_widget(
            Paragraph::new(format!("{:.1}", 1.0 / frame_time)),
            frame.area(),
        );
        Ok(())
    }

    pub fn tick(&mut self) {
        self.throbber_state.calc_next();
    }

    pub fn draw_current_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        match self.current_screen {
            Screens::InitScreen => {
                self.render_init_screen(frame, app)?;
            }
            Screens::MainScreen => {
                self.render_main_screen(frame, app)?;
            }
            Screens::TermSizeWarn => {
                self.render_term_size_warning(frame)?;
            }
        }

        Ok(())
    }

    pub fn draw_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        match self.active_popup.as_ref().unwrap() {
            Popups::FetchArtwork => {
                self.draw_fetch_artworks_popup(frame, app)?;
            }
            Popups::AddMovie => {
                self.draw_add_movie_popup(frame, app)?;
            }
            Popups::EditMovie => {
                self.draw_edit_movie_popup(frame, app)?;
            }
            Popups::RemoveMovie => {
                self.draw_remove_movie_popup(frame, app)?;
            }
            Popups::Error => {
                self.draw_error_popup(frame)?;
            }
            Popups::TMDBInit => {
                self.draw_tmdb_init_popup(frame)?;
            }
            Popups::TraktInit => {
                self.draw_trakt_init_popup(frame, app)?;
            }
        }

        Ok(())
    }

    pub fn check_popups(&mut self, app: &App) -> Result<()> {
        if let Some(popup) = self.active_popup.as_ref() {
            match popup {
                Popups::FetchArtwork => {
                    if self.fetch_artwork_popup.check_done(app) {
                        self.close_popups();
                        self.image_backend.reload_images(
                            app,
                            self.main_screen.movies_list.scroll_pos,
                            Some(self.main_screen.movies_list.num_visible_movies),
                        );
                    }

                    self.fetch_artwork_popup.read_threads_responses()?;
                }
                Popups::AddMovie => (),
                Popups::EditMovie => (),
                Popups::RemoveMovie => (),
                Popups::Error => (),
                Popups::TraktInit => (),
                Popups::TMDBInit => (),
            }
        }

        Ok(())
    }

    pub fn close_popups(&mut self) {
        self.active_popup = None;

        // self.fetch_artwork_popup_options.reset();

        // self.main_screen_options.rehash_visible_images(app);
    }

    pub fn open_tmdb_init_popup(&mut self) {
        self.active_popup = Some(Popups::TMDBInit);

        self.tmdb_init_popup.begin();
    }

    pub fn open_trakt_init_popup(&mut self) {
        self.active_popup = Some(Popups::TraktInit);

        self.trakt_init_popup.begin();
    }

    pub fn open_fetch_artworks_popup(&mut self, app: &mut App) -> Result<()> {
        self.active_popup = Some(Popups::FetchArtwork);

        self.fetch_artwork_popup.begin(app)?;

        Ok(())
    }

    pub fn open_add_movie_popup(&mut self) {
        self.active_popup = Some(Popups::AddMovie);

        self.add_movie_popup.begin();
    }

    pub fn open_edit_movie_popup(&mut self, app: &mut App) {
        self.active_popup = Some(Popups::EditMovie);

        self.edit_movie_popup
            .begin(self.main_screen.current_movie().user_rating);
    }

    pub fn open_remove_movie_popup(&mut self) {
        self.active_popup = Some(Popups::RemoveMovie);

        self.remove_movie_popup.begin();
    }

    pub fn open_error_popup(&mut self, message: String) {
        self.active_popup = Some(Popups::Error);

        self.error_popup_error = message;
    }

    pub fn can_quit(&self) -> bool {
        self.should_quit && self.fetch_artwork_popup.done
    }

    fn check_term_size(&mut self, frame: &Frame) -> bool {
        if (frame.area().width as u32) < MINTERMSIZE[0]
            || (frame.area().height as u32) < MINTERMSIZE[1]
        {
            if self.current_screen != Screens::TermSizeWarn {
                self.previous_screen = Some(self.current_screen);
                self.current_screen = Screens::TermSizeWarn;
            }

            return false;
        } else if let Screens::TermSizeWarn = self.current_screen {
            self.current_screen = self.previous_screen.unwrap();
            self.previous_screen = None;
        }

        true
    }

    fn render_term_size_warning(&self, frame: &mut Frame) -> Result<()> {
        let frame_area = frame.area();
        let lines = vec![
            Line::from_iter([
                "Terminal is too small: ".into(),
                frame_area.width.to_string().red(),
                "x".into(),
                frame_area.height.to_string().red(),
            ]),
            Line::default(),
            Line::from_iter([
                "Minimum size is: ".into(),
                MINTERMSIZE[0].to_string().green(),
                "x".into(),
                MINTERMSIZE[1].to_string().green(),
            ]),
        ];
        let area = center_rect(
            frame_area,
            Constraint::Min(0),
            Constraint::Length(lines.len() as u16),
        );
        let text = Text::from(lines).centered();

        frame.render_widget(text, area);

        Ok(())
    }
}
