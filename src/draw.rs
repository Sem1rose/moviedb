use crate::{
    app::{App, Result},
    popups::{
        add_movie::AddMoviePopup, edit_movie::EditMoviePopup, fetch_artworks::FetchArtworksPopup,
        remove_movie::RemoveMoviePopup, Popups,
    },
    screens::{init_screen::InitScreen, main_screen::MainScreen},
};
use ratatui::{
    crossterm::event::{Event, KeyEvent},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use tui_input::backend::crossterm::EventHandler;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CurrentScreen {
    #[default]
    InitScreen,
    MainScreen,
    TermSizeWarn,
}

#[derive(Default)]
pub struct Drawer {
    pub(crate) throbber_state: throbber_widgets_tui::ThrobberState,

    pub(crate) update: bool,
    pub(crate) accepting_input: bool,

    pub(crate) previous_screen: Option<CurrentScreen>,
    pub(crate) current_screen: CurrentScreen,
    pub(crate) active_popup: Option<Popups>,

    pub(crate) main_screen_options: MainScreen,
    pub(crate) init_screen_options: InitScreen,
    pub(crate) add_movie_popup_options: AddMoviePopup,
    pub(crate) edit_movie_popup_options: EditMoviePopup,
    pub(crate) remove_movie_popup_options: RemoveMoviePopup,
    pub(crate) fetch_artwork_popup_options: FetchArtworksPopup,
    // pub(crate) clear_images: bool,
}

const MINTERMSIZE: [u32; 2] = [80, 22];
impl Drawer {
    // pub fn new(app: &App) -> Self {
    //     Self {
    //         current_screen: CurrentScreen::InitScreen,
    //         fetch_artwork_popup_options: FetchArtworksPopup::new(app),

    //         update: false,
    //         accepting_input: false,
    //         // clear_images: false,
    //         previous_screen: None,
    //         active_popup: None,

    //         throbber_state: Default::default(),
    //         main_screen_options: Default::default(),
    //         init_screen_options: Default::default(),
    //         add_movie_popup_options: Default::default(),
    //         edit_movie_popup_options: Default::default(),
    //         remove_movie_popup_options: Default::default(),
    //     }
    // }
    pub fn inc_selection(&mut self, app: &App) {
        if CurrentScreen::MainScreen == self.current_screen {
            if self.active_popup.is_none() && self.main_screen_options.num_visible_movies > 0 {
                if self
                    .main_screen_options
                    .inc_movie_selection(app.movies.len())
                {
                    // self.clear_images(false);
                }
            } else if !self.add_movie_popup_options.movie_selected {
                self.add_movie_popup_options.inc_movie_selection();
                self.queue_update();
            }
        }
    }

    pub fn dec_selection(&mut self) {
        if CurrentScreen::MainScreen == self.current_screen {
            if self.active_popup.is_none() {
                if self.main_screen_options.dec_movie_selection() {
                    // self.clear_images(false);
                }
            } else if !self.add_movie_popup_options.movie_selected {
                self.add_movie_popup_options.dec_movie_selection();
                self.queue_update();
            }
        }
    }

    pub fn inc_selection_horiz(&mut self) {
        if let Some(Popups::RemoveMovie) = self.active_popup {
            self.remove_movie_popup_options.selected += 1;
            if self.remove_movie_popup_options.selected >= RemoveMoviePopup::BUTTONS {
                self.remove_movie_popup_options.selected = 0;
            }
            self.queue_update();
        }
    }

    pub fn dec_selection_horiz(&mut self) {
        if let Some(Popups::RemoveMovie) = self.active_popup {
            self.remove_movie_popup_options.selected -= 1;
            if self.remove_movie_popup_options.selected < 0 {
                self.remove_movie_popup_options.selected = RemoveMoviePopup::BUTTONS - 1;
            }
            self.queue_update();
        }
    }

    pub fn handle_input(&mut self, event: KeyEvent) {
        self.queue_update();
        match self.active_popup {
            Some(Popups::AddMovie) => {
                self.add_movie_popup_options
                    .search_input
                    .handle_event(&Event::Key(event));
            }
            Some(Popups::EditMovie) => {
                self.edit_movie_popup_options
                    .user_rating_input
                    .handle_event(&Event::Key(event));
            }
            _ => {}
        }
    }

    pub fn queue_update(&mut self) {
        self.update = true;
    }

    pub fn render_app(&mut self, frame: &mut Frame, app: &mut App, frame_time: f64) -> Result<()> {
        self.check_term_size(frame);

        self.draw_current_screen(frame, app)?;

        if self.active_popup.is_some() {
            self.draw_popup(frame, app)?;
        }

        frame.render_widget(
            Paragraph::new(format!("{:.1}", 1.0 / frame_time)),
            // Paragraph::new(format!("{}", app.clear_images)),
            frame.area(),
        );
        Ok(())
    }

    pub fn tick(&mut self) {
        self.throbber_state.calc_next();
        self.main_screen_options.inc_tickets_age();
    }

    // pub fn clear_images(&mut self, clear_cache: bool) {
    //     self.clear_images = true;
    //     self.main_screen_options.images_displayed.clear();
    //     self.main_screen_options.backdrop_displayed = false;

    //     if clear_cache {
    //         self.movie_artwork.lock().unwrap().clear();
    //         self.movie_artworks_requested.clear();
    //     }
    // }

    pub fn draw_current_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        match self.current_screen {
            CurrentScreen::InitScreen => {
                self.render_init_screen(frame, app)?;
            }
            CurrentScreen::MainScreen => {
                self.render_movies_list(frame, app)?;
            }
            CurrentScreen::TermSizeWarn => {
                self.render_term_size_warning(frame)?;
            }
        }

        Ok(())
    }

    pub fn draw_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        match self.active_popup.unwrap() {
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
        }

        Ok(())
    }

    pub fn close_popups(&mut self) {
        self.active_popup = None;

        self.fetch_artwork_popup_options.finish();

        self.accepting_input = false;
    }

    pub fn open_fetch_artworks_popup(&mut self, app: &mut App) {
        self.active_popup = Some(Popups::FetchArtwork);
        self.fetch_artworks(app);
    }

    pub fn open_add_movie_popup(&mut self) {
        self.active_popup = Some(Popups::AddMovie);
        self.add_movie_popup_options.begin();
        self.accepting_input = true;
    }

    pub fn open_edit_movie_popup(&mut self) {
        self.active_popup = Some(Popups::EditMovie);
        self.edit_movie_popup_options.begin();
        self.accepting_input = true;
    }

    pub fn open_remove_movie_popup(&mut self) {
        self.active_popup = Some(Popups::RemoveMovie);
        self.remove_movie_popup_options.begin();
        self.accepting_input = false;
    }

    pub(crate) fn center(&self, area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
        let [area] = Layout::horizontal([horizontal])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
        area
    }

    fn check_term_size(&mut self, frame: &Frame) -> bool {
        if (frame.area().width as u32) < MINTERMSIZE[0]
            || (frame.area().height as u32) < MINTERMSIZE[1]
        {
            if self.current_screen != CurrentScreen::TermSizeWarn {
                self.previous_screen = Some(self.current_screen);
                self.current_screen = CurrentScreen::TermSizeWarn;
            }

            return false;
        } else if let CurrentScreen::TermSizeWarn = self.current_screen {
            self.current_screen = self.previous_screen.unwrap();
            self.previous_screen = None;
        }

        true
    }

    fn render_term_size_warning(&mut self, frame: &mut Frame) -> Result<()> {
        let frame_area = frame.area();
        let lines = vec![
            Line::from_iter([
                "Terminal is too small: ".into(),
                frame_area.width.to_string().red(),
                "X".into(),
                frame_area.height.to_string().red(),
            ]),
            Line::default(),
            Line::from_iter([
                "Minimum size is: ".into(),
                MINTERMSIZE[0].to_string().green(),
                "X".into(),
                MINTERMSIZE[1].to_string().green(),
            ]),
        ];
        let area = self.center(
            frame_area,
            Constraint::Min(0),
            Constraint::Length(lines.len() as u16),
        );
        let text = Text::from(lines).centered();

        frame.render_widget(text, area);

        Ok(())
    }
}
