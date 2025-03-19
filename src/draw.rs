use crate::{
    app::{App, Result},
    popups::{
        add_movie::AddMoviePopup, edit_movie::EditMoviePopup, fetch_artworks::FetchArtworksPopup,
        remove_movie::RemoveMoviePopup, Popups,
    },
    screens::{init_screen::InitScreen, main_screen::MainScreen, Screens},
};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};

#[derive(Default)]
pub struct Drawer {
    pub(crate) throbber_state: throbber_widgets_tui::ThrobberState,

    pub(crate) accepting_input: bool,

    pub(crate) previous_screen: Option<Screens>,
    pub(crate) current_screen: Screens,
    pub(crate) active_popup: Option<Popups>,

    pub(crate) main_screen_options: MainScreen,
    pub(crate) init_screen_options: InitScreen,
    pub(crate) add_movie_popup_options: AddMoviePopup,
    pub(crate) edit_movie_popup_options: EditMoviePopup,
    pub(crate) remove_movie_popup_options: RemoveMoviePopup,
    pub(crate) fetch_artwork_popup_options: FetchArtworksPopup,
}

const MINTERMSIZE: [u32; 2] = [80, 22];
impl Drawer {
    pub fn render_app(&mut self, frame: &mut Frame, app: &mut App, frame_time: f64) -> Result<()> {
        self.check_term_size(frame);

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
                self.render_movies_list(frame, app)?;
            }
            Screens::TermSizeWarn => {
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

    pub fn open_fetch_artworks_popup(&mut self, app: &mut App) -> Result<()> {
        self.active_popup = Some(Popups::FetchArtwork);
        self.fetch_artworks(app)?;
        Ok(())
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
