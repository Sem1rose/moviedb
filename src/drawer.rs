use std::path::PathBuf;

use crate::{
    popups::*,
    screens::*,
    tokens::{OMDBTokens, TMDBTokens, TraktTokens},
    KeyEventHandler,
};
use ratatui::{
    layout::Constraint,
    style::{palette::tailwind::*, Stylize},
    text::{Line, Text},
    widgets::Block,
    Frame,
};

pub struct Drawer {
    pub refresh_immediate: u8,
    show_term_size_warning: bool,
    pub active_popup: Option<Popups>,
    pub current_screen: Option<Screens>,
    pub popup_queue: Vec<Popups>,
    pub screen_queue: Vec<Screens>,

    home_dir: PathBuf,
    cache_dir: PathBuf,
}

const MINTERMSIZE: [u32; 2] = [100, 30];
impl Drawer {
    pub fn new(home_dir: &PathBuf, cache_dir: &PathBuf) -> Self {
        Drawer {
            refresh_immediate: 0,
            home_dir: home_dir.clone(),
            cache_dir: cache_dir.clone(),
            show_term_size_warning: false,

            active_popup: None,
            current_screen: None,
            screen_queue: vec![Screens::MainScreen(MainScreen::new(cache_dir))],
            popup_queue: vec![
                Popups::FetchArtworks(FetchArtworksPopup::new(cache_dir)),
                Popups::OMDBInit(OMDBInitPopup::new(home_dir)),
                Popups::TMDBInit(TMDBInitPopup::new(home_dir)),
                Popups::TraktInit(TraktInitPopup::new(home_dir, false)),
            ],
        }
    }

    pub fn render_app(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        self.refresh_immediate = self.refresh_immediate.saturating_sub(1);

        self.check_term_size(frame);
        self.update_image_renderers();

        self.draw_current_screen(frame, key_event_handler);

        self.try_pop_queues(key_event_handler);
        self.check_popups(key_event_handler);
        if !self.show_term_size_warning && self.active_popup.is_some() {
            self.draw_popup(frame, key_event_handler);
        }
    }

    fn update_image_renderers(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            main_screen.image_renderer.update();
        }
    }

    fn draw_current_screen(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        frame.render_widget(Block::new().bg(SLATE.c900), frame.area());

        if self.show_term_size_warning {
            self.render_term_size_warning(frame);
        } else if let Some(current_screen) = self.current_screen.as_mut() {
            match current_screen {
                Screens::MainScreen(main_screen) => {
                    main_screen.render(frame, key_event_handler);
                }
            }
        }
    }

    fn check_popups(&mut self, key_event_handler: &mut KeyEventHandler) {
        if let Some(popup) = self.active_popup.as_mut() {
            match popup {
                Popups::EditMovie(_) => {}
                Popups::DeleteMovie(_) => {}
                Popups::AddMovie(add_movie_popup) => {
                    add_movie_popup.update();

                    if let AddMoviePopupPhase::Done = add_movie_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.add_movie();
                        });
                    }
                }
                Popups::TMDBInit(tmdb_init_popup) => {
                    tmdb_init_popup.update();

                    if let TMDBInitPopupPhase::Done = tmdb_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_tmdb_user_tokens();
                        });
                    }
                }
                Popups::OMDBInit(omdb_init_popup) => {
                    omdb_init_popup.update();

                    if omdb_init_popup.done {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_omdb_user_tokens();
                        });
                    }
                }
                Popups::TraktInit(trakt_init_popup) => {
                    trakt_init_popup.update();

                    if let TraktInitPopupPhase::Done = trakt_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_trakt_user_tokens();
                        });
                    }
                }
                Popups::FetchArtworks(fetch_artworks_popup) => {
                    fetch_artworks_popup.update();

                    if fetch_artworks_popup.done {
                        self.close_popups();
                    }
                }
            }
        }
    }

    fn draw_popup(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        if let Some(active_popup) = self.active_popup.as_mut() {
            match active_popup {
                Popups::EditMovie(edit_movie_popup) => {
                    edit_movie_popup.render(frame, key_event_handler);
                }
                Popups::DeleteMovie(delete_movie_popup) => {
                    delete_movie_popup.render(frame, key_event_handler);
                }
                Popups::AddMovie(add_movie_popup) => {
                    add_movie_popup.render(frame, key_event_handler);
                }
                Popups::TMDBInit(tmdb_init_popup) => {
                    tmdb_init_popup.render(frame, key_event_handler);
                }
                Popups::OMDBInit(omdb_init_popup) => {
                    omdb_init_popup.render(frame, key_event_handler);
                }
                Popups::TraktInit(trakt_init_popup) => {
                    trakt_init_popup.render(frame, key_event_handler);
                }
                Popups::FetchArtworks(fetch_artworks_popup) => {
                    fetch_artworks_popup.render(frame, key_event_handler);
                }
            }
        }
    }

    fn try_pop_queues(&mut self, key_event_handler: &mut KeyEventHandler) {
        if self.active_popup.is_none() {
            if !self.popup_queue.is_empty() {
                self.active_popup = self.popup_queue.pop();

                if matches!(self.active_popup, Some(Popups::FetchArtworks(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Popups::FetchArtworks(fetch_artworks_popup)) = app.drawer.active_popup.as_mut() {
                            fetch_artworks_popup.set_movies(&app.movies, app.trakt_tokens.client_id(), app.tmdb_tokens.access_token());
                        }
                    });
                }
            } else if !self.screen_queue.is_empty() {
                self.current_screen = self.screen_queue.pop();

                if matches!(self.current_screen, Some(Screens::MainScreen(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                            main_screen.set_movies(&app.movies);
                        }
                    });
                }
            }
        }
    }
    pub fn open_add_movie_popup(
        &mut self,
        trakt_tokens: TraktTokens,
        tmdb_tokens: TMDBTokens,
        omdb_tokens: OMDBTokens,
    ) {
        self.popup_queue.push(Popups::AddMovie(AddMoviePopup::new(
            trakt_tokens,
            tmdb_tokens,
            omdb_tokens,
            &self.cache_dir,
        )));
    }
    pub fn open_add_play_popup(&mut self) {
        self.popup_queue.push(Popups::EditMovie(EditMoviePopup::new(true, 0.0)));
    }
    pub fn open_edit_movie_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.popup_queue.push(Popups::EditMovie(EditMoviePopup::new(
                false,
                main_screen.current_movie().unwrap().get_user_rating(),
            )));
        }
    }
    pub fn open_delete_movie_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.popup_queue.push(Popups::DeleteMovie(DeleteMoviePopup::new(
                &main_screen.current_movie().unwrap().name,
            )));
        }
    }
    pub fn close_popups(&mut self) {
        self.active_popup = None;

        self.refresh_immediate += 2;
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            main_screen.redraw_images = 1;
        }
    }

    pub fn check_refresh_immediate(&mut self) -> bool {
        self.refresh_immediate > 0
    }
    pub fn check_refresh_delayed(&mut self) -> bool {
        match self.active_popup.as_ref() {
            Some(Popups::AddMovie(add_movie_popup)) => {
                return add_movie_popup.update_next_frame();
            }
            Some(Popups::TMDBInit(tmdb_init_popup)) => {
                return tmdb_init_popup.update_next_frame();
            }
            Some(Popups::OMDBInit(omdb_init_popup)) => {
                return omdb_init_popup.update_next_frame();
            }
            Some(Popups::TraktInit(trakt_init_popup)) => {
                return trakt_init_popup.update_next_frame();
            }
            Some(Popups::FetchArtworks(fetch_artworks_popup)) => {
                return fetch_artworks_popup.update_next_frame();
            }
            _ => {}
        }
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_ref() {
            return main_screen.drawing_images;
        }

        false
    }

    fn check_term_size(&mut self, frame: &Frame) {
        self.show_term_size_warning = (frame.area().width as u32) < MINTERMSIZE[0]
            || (frame.area().height as u32) < MINTERMSIZE[1];
    }

    fn render_term_size_warning(&mut self, frame: &mut Frame) {
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
        let area = crate::helpers::center_rect(
            frame_area,
            Constraint::Min(0),
            Constraint::Length(lines.len() as u16),
        );
        let text = Text::from(lines).centered();

        frame.render_widget(text, area);
    }
}
