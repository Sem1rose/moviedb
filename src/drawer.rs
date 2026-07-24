use std::{cell::RefCell, path::PathBuf, rc::Rc};

use ratatui::{
    Frame,
    macros::{constraint, line, span},
    style::{Stylize, palette::tailwind},
    text::Text,
    widgets::Block,
};

use crate::{
    KeyEventHandler,
    config::Config,
    popups::*,
    screens::{Screens, main_screen::MainScreen},
    tokens::{OMDBTokens, TMDBTokens, TraktTokens},
};

pub struct Drawer {
    pub refresh_immediate:  u8,
    show_term_size_warning: bool,
    pub active_popup:       Option<Popups>,
    pub current_screen:     Option<Screens>,
    pub popup_queue:        Vec<Popups>,
    pub screen_queue:       Vec<Screens>,
    pub config:             Rc<RefCell<Config>>,

    home_dir:  PathBuf,
    cache_dir: PathBuf,
}

const MINTERMSIZE: [u32; 2] = [100, 30];
impl Drawer {
    pub fn new(home_dir: &PathBuf, cache_dir: &PathBuf, config: Rc<RefCell<Config>>) -> Self {
        let popup_queue = if config.borrow().options.oob_done {
            let mut popups = vec![];
            if config.borrow_mut().options.trakt_enabled {
                popups.push(Popups::TraktInit(TraktInitPopup::new(home_dir, false)));
            }
            if config.borrow_mut().options.tmdb_enabled {
                popups.push(Popups::TMDBInit(TMDBInitPopup::new(home_dir, false)));
            }
            if config.borrow_mut().options.omdb_enabled {
                popups.push(Popups::OMDBInit(OMDBInitPopup::new(home_dir, false)));
            }

            popups.push(Popups::FetchArtworks(FetchArtworksPopup::new(cache_dir)));

            popups
        } else {
            vec![Popups::OutOfBox(OutOfBoxPopup::new())]
        };

        Drawer {
            refresh_immediate: 0,
            home_dir: home_dir.clone(),
            cache_dir: cache_dir.clone(),
            show_term_size_warning: false,

            active_popup: None,
            current_screen: None,
            screen_queue: vec![Screens::MainScreen(MainScreen::new(
                cache_dir,
                config.clone(),
            ))],
            popup_queue,

            config,
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
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame.area());

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
            popup.update();

            match popup {
                Popups::EditMovie(_) => {}
                Popups::DeleteMovie(_) => {}
                Popups::AddMovie(add_movie_popup) => {
                    if let AddMoviePopupPhase::Done = add_movie_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.add_movie();
                        });
                    }
                }
                Popups::TMDBInit(tmdb_init_popup) => {
                    if let TMDBInitPopupPhase::Done = tmdb_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_tmdb_user_tokens();
                        });
                    }
                }
                Popups::OMDBInit(omdb_init_popup) =>
                    if omdb_init_popup.done {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_omdb_user_tokens();
                        });
                    },
                Popups::TraktInit(trakt_init_popup) => {
                    if let TraktInitPopupPhase::Done = trakt_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_trakt_user_tokens();
                        });
                    }
                }
                Popups::FetchArtworks(fetch_artworks_popup) =>
                    if fetch_artworks_popup.done {
                        self.close_popup();
                    },
                Popups::OutOfBox(_) => {}
                Popups::AdvancedFilter(_) => {}
            }
        }
    }

    fn draw_popup(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        if let Some(active_popup) = self.active_popup.as_mut() {
            active_popup.render(frame, key_event_handler);
        }
    }

    fn try_pop_queues(&mut self, key_event_handler: &mut KeyEventHandler) {
        if self.active_popup.is_none() {
            if !self.popup_queue.is_empty() {
                self.active_popup = Some(self.popup_queue.remove(0));

                if matches!(self.active_popup, Some(Popups::FetchArtworks(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Popups::FetchArtworks(fetch_artworks_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            fetch_artworks_popup.initialize(
                                &app.movies,
                                &app.trakt_tokens,
                                &app.tmdb_tokens,
                            );
                        }
                    });
                } else if matches!(self.active_popup, Some(Popups::AdvancedFilter(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.initialize(&app.movies);
                        }
                    });
                }
            } else if !self.screen_queue.is_empty() {
                self.current_screen = self.screen_queue.pop();

                if matches!(self.current_screen, Some(Screens::MainScreen(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.set_movies(&app.movies);
                        }
                    });
                }
            }
        }
    }

    pub fn open_trakt_init_popup(&mut self) {
        self.popup_queue
            .push(Popups::TraktInit(TraktInitPopup::new(&self.home_dir, true)));
    }

    pub fn open_tmdb_init_popup(&mut self) {
        self.popup_queue
            .push(Popups::TMDBInit(TMDBInitPopup::new(&self.home_dir, true)));
    }

    pub fn open_omdb_init_popup(&mut self) {
        self.popup_queue
            .push(Popups::OMDBInit(OMDBInitPopup::new(&self.home_dir, true)));
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
        self.popup_queue
            .push(Popups::EditMovie(EditMoviePopup::new(true, 0.0)));
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
            self.popup_queue
                .push(Popups::DeleteMovie(DeleteMoviePopup::new(
                    &main_screen.current_movie().unwrap().name,
                )));
        }
    }

    pub fn open_advanced_filter_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.popup_queue
                .push(Popups::AdvancedFilter(AdvancedFilterPopup::new(
                    &main_screen.filter_criteria,
                )));
        }
    }

    pub fn close_popup(&mut self) {
        self.active_popup = None;

        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            main_screen.redraw_images = 1;
        }
        self.refresh_immediate += 2;
    }

    pub fn check_refresh_immediate(&mut self) -> bool {
        self.refresh_immediate > 0
    }

    pub fn check_refresh_delayed(&mut self) -> bool {
        if let Some(active_popup) = self.active_popup.as_ref() {
            return active_popup.update_next_frame();
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
            line![
                span!("Terminal is too small: "),
                frame_area.width.to_string().red(),
                span!("x"),
                frame_area.height.to_string().red(),
            ],
            line!(),
            line![
                span!("Minimum size is: "),
                MINTERMSIZE[0].to_string().green(),
                span!("x"),
                MINTERMSIZE[1].to_string().green(),
            ],
        ];
        let area = frame_area.centered(constraint!(>= 0), constraint!(== lines.len() as u16));
        let text = Text::from_iter(lines).centered();

        frame.render_widget(text, area);
    }
}
