use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Layout, Offset, Size},
    macros::{constraint, line, span},
    style::{Stylize, palette::tailwind},
    text::Text,
    widgets::{Block, Clear},
};

use crate::{
    config::Config,
    helpers::ellipsize_string,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::*,
    screens::{Screens, main_screen::MainScreen},
    tokens::{OMDBTokens, PunchPlayTokens, TMDBTokens, TraktTokens},
    types::{Entry, FxIndexMap, Movie},
};

pub struct Drawer {
    pub refresh_immediate:  u8,
    show_term_size_warning: bool,
    pub active_popup:       Option<Popup>,
    pub current_screen:     Option<Screens>,
    pub popup_queue:        Vec<Popup>,
    pub screen_queue:       Vec<Screens>,
    pub _config:            Rc<RefCell<Config>>,
    pub image_renderer:     RatatuiImage,

    // home_dir:  PathBuf,
    cache_dir: PathBuf,
}

#[macro_export]
macro_rules! new_popup {
    ($popup_enum:ident, $T:expr) => {
        Popup::$popup_enum(Box::new($T))
    };
    ($popup_enum:ident) => {
        Popup::$popup_enum(Box::default())
    };
}

const MINTERMSIZE: [u32; 2] = [100, 30];
impl Drawer {
    pub fn new(home_dir: &Path, cache_dir: &Path, _config: Rc<RefCell<Config>>) -> Self {
        let popup_queue = if _config.borrow().options.oob_done {
            let mut popups = Vec::with_capacity(5);
            if _config.borrow_mut().options.tmdb_enabled {
                popups.push(new_popup!(TMDBInit, TMDBInitPopup::new(home_dir, false)));
            }
            if _config.borrow_mut().options.simkl_enabled {
                popups.push(new_popup!(SimklInit, SimklInitPopup::new(home_dir, false)));
            }
            if _config.borrow_mut().options.punch_play_enabled {
                popups.push(new_popup!(
                    PunchPlayInit,
                    PunchPlayInitPopup::new(home_dir, false)
                ));
            }
            if _config.borrow_mut().options.trakt_enabled {
                popups.push(new_popup!(TraktInit, TraktInitPopup::new(home_dir, false)));
            }
            if _config.borrow_mut().options.omdb_enabled {
                popups.push(new_popup!(OMDBInit, OMDBInitPopup::new(home_dir, false)));
            }

            popups
        } else {
            vec![new_popup!(OutOfBox)]
        };

        Drawer {
            image_renderer: RatatuiImage::new(cache_dir),

            refresh_immediate: 0,
            // home_dir: home_dir.to_path_buf(),
            cache_dir: cache_dir.to_path_buf(),
            show_term_size_warning: false,

            active_popup: None,
            current_screen: None,
            screen_queue: vec![Screens::MainScreen(MainScreen::new(
                home_dir,
                _config.clone(),
            ))],
            popup_queue,

            _config,
        }
    }

    pub fn render_app(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        self.refresh_immediate = self.refresh_immediate.saturating_sub(1);

        self.check_term_size(frame);
        self.image_renderer.update();

        self.draw_current_screen(frame, key_event_handler);

        self.try_pop_queues(key_event_handler);
        self.check_popups(key_event_handler);
        if !self.show_term_size_warning {
            if self.active_popup.is_some() {
                self.draw_popup(frame, key_event_handler);
            }
            self.render_footer(frame, key_event_handler);
        }
    }

    fn draw_current_screen(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame.area());

        if self.show_term_size_warning {
            self.render_term_size_warning(frame);
        } else if let Some(current_screen) = self.current_screen.as_mut() {
            match current_screen {
                Screens::MainScreen(main_screen) => {
                    main_screen.render(frame, key_event_handler, &mut self.image_renderer);
                }
            }
        }
    }

    fn check_popups(&mut self, key_event_handler: &mut KeyEventHandler) {
        if let Some(popup) = self.active_popup.as_mut() {
            popup.update();

            match popup {
                Popup::ManagePlays(_) => {}
                Popup::DeleteMovie(_) => {}
                Popup::AddMovie(add_movie_popup) => {
                    if let AddMoviePopupPhase::Done = add_movie_popup.phase {
                        let refetch_details = add_movie_popup.refetch_details;
                        key_event_handler.bind_immediate(move |app, _| {
                            if refetch_details {
                                app.update_movie_details();
                            } else {
                                app.add_movie();
                            }
                        });
                    }
                }
                Popup::TMDBInit(tmdb_init_popup) => {
                    if let TMDBInitPopupPhase::Done = tmdb_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_tmdb_user_tokens();
                        });
                    }
                }
                Popup::SimklInit(simkl_init_popup) =>
                    if let SimklInitPopupPhase::Done = simkl_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_simkl_user_tokens();
                        });
                    },
                Popup::PunchPlayInit(punch_play_init_popup) =>
                    if let PunchPlayInitPopupPhase::Done = punch_play_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_punch_play_user_tokens();
                        });
                    },
                Popup::TraktInit(trakt_init_popup) => {
                    if let TraktInitPopupPhase::Done = trakt_init_popup.phase {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_trakt_user_tokens();
                        });
                    }
                }
                Popup::OMDBInit(omdb_init_popup) =>
                    if omdb_init_popup.done {
                        key_event_handler.bind_immediate(|app, _| {
                            app.set_omdb_user_tokens();
                        });
                    },
                Popup::OutOfBox(_) => {}
                Popup::AdvancedFilter(_) => {}
                Popup::FetchMovies(fetch_movies_popup) =>
                    if fetch_movies_popup.done {
                        key_event_handler.bind_immediate(|app, _| {
                            app.save_data(true, false, true, true);
                            if let Some(Screens::MainScreen(main_screen)) =
                                app.drawer.current_screen.as_mut()
                            {
                                main_screen.filter_sort_movies(false);
                            }
                            app.drawer.close_popup();
                        });
                    },
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

                if matches!(self.active_popup, Some(Popup::AdvancedFilter(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        if let Some(Popup::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.initialize(
                                &app.movies.borrow().values().collect_vec(),
                                &app.persons.borrow(),
                            );
                        }
                    });
                } else if matches!(self.active_popup, Some(Popup::FetchMovies(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        let lists = if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.lists.values().collect_vec()
                        } else {
                            vec![]
                        };

                        if let Some(Popup::FetchMovies(fetch_movies_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            fetch_movies_popup.initialize(
                                app.tmdb_tokens.clone(),
                                app.punch_play_tokens.clone(),
                                app.trakt_tokens.clone(),
                                app.omdb_tokens.clone(),
                                app.movies.clone(),
                                app.collections.clone(),
                                app.persons.clone(),
                                &app.watched.borrow(),
                                &lists,
                            );
                        }
                    });
                }
            } else if !self.screen_queue.is_empty() {
                self.current_screen = self.screen_queue.pop();

                if matches!(self.current_screen, Some(Screens::MainScreen(_))) {
                    key_event_handler.bind_immediate(|app, _| {
                        // app.initialize_processors();
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            if main_screen.initialize(app.movies.clone(), app.watched.clone()) {
                                app.drawer.open_fetch_movies_popup();
                            }
                        }
                    });
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_fetch_movies_popup(&mut self) {
        self.popup_queue.push(new_popup!(FetchMovies));
    }

    // pub fn open_trakt_init_popup(&mut self) {
    //     self.popup_queue.push(new_popup!(TraktInit, TraktInitPopup::new(
    //         &self._home_dir,
    //         true,
    //     )));
    // }

    // pub fn open_punch_play_init_popup(&mut self) {
    //     self.popup_queue
    //         .push(new_popup!(PunchPlayInit, PunchPlayInitPopup::new(
    //             &self._home_dir,
    //             true,
    //         )));
    // }

    // pub fn open_tmdb_init_popup(&mut self) {
    //     self.popup_queue
    //         .push(new_popup!(TMDBInit, TMDBInitPopup::new(&self._home_dir, true)));
    // }

    // pub fn open_omdb_init_popup(&mut self) {
    //     self.popup_queue
    //         .push(new_popup!(OMDBInit, OMDBInitPopup::new(&self._home_dir, true)));
    // }

    pub fn open_add_movie_popup(
        &mut self,
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
        take_rating: bool,
    ) {
        self.popup_queue
            .push(Popup::AddMovie(Box::new(AddMoviePopup::new(
                tmdb_tokens,
                punch_play_tokens,
                trakt_tokens,
                omdb_tokens,
                take_rating,
                &self.cache_dir,
            ))));
    }

    pub fn open_refetch_details_popup(
        &mut self,
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
    ) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.popup_queue.push(Popup::AddMovie(Box::new(
                AddMoviePopup::new_refetch_details(
                    main_screen.current_movie().unwrap().id,
                    tmdb_tokens,
                    punch_play_tokens,
                    trakt_tokens,
                    omdb_tokens,
                    &self.cache_dir,
                ),
            )));
        }
    }

    pub fn open_add_play_popup(&mut self) {
        self.popup_queue.push(Popup::ManagePlays(Box::new(
            ManagePlaysPopup::new_add_play(),
        )));
    }

    pub fn open_edit_movie_popup(&mut self, watched: &FxIndexMap<u32, Entry>) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            let entry = &watched[&main_screen.current_movie().unwrap().id];
            self.popup_queue.push(Popup::ManagePlays(Box::new(
                ManagePlaysPopup::new_edit_rating(entry),
            )));
        }
    }

    pub fn open_manage_plays_popup(&mut self, watched: &FxIndexMap<u32, Entry>) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            let entry = watched.get(&main_screen.current_movie().unwrap().id);
            self.popup_queue
                .push(Popup::ManagePlays(Box::new(ManagePlaysPopup::new(
                    entry.cloned(),
                ))));
        }
    }

    pub fn open_delete_movie_popup(&mut self, movies: &FxIndexMap<u32, Movie>) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            if let Some(name) = movies
                .get(&main_screen.current_movie().unwrap().id)
                .map(|x| &x.title)
            {
                self.popup_queue
                    .push(Popup::DeleteMovie(Box::new(DeleteMoviePopup::new(name))));
            }
        }
    }

    pub fn open_advanced_filter_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.popup_queue
                .push(Popup::AdvancedFilter(Box::new(AdvancedFilterPopup::new(
                    &main_screen.filter_criteria,
                ))));
        }
    }

    pub fn close_popup(&mut self) {
        self.active_popup = None;
    }

    pub fn check_refresh_immediate(&mut self) -> bool {
        self.refresh_immediate > 0
    }

    pub fn check_refresh_delayed(&mut self) -> bool {
        (if let Some(active_popup) = self.active_popup.as_ref() {
            active_popup.update_next_frame()
        } else {
            false
        }) | if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_ref() {
            main_screen.drawing_images
        } else {
            false
        }
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

    fn render_footer(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        let area = frame
            .area()
            .resize(Size::new(frame.area().width, 2))
            .offset(Offset::new(0, frame.area().height as i32 - 2));

        frame.render_widget(Clear, area);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), area);

        // ↔⇆⬌⬍⮀⬅⬆⬇←↕→↓↹•↵⏎
        let bind_to_string = |bind: &key_event_handler::Bind| {
            match bind {
                key_event_handler::Bind::Horizontal => {
                    span!(" ←→ ")
                }
                key_event_handler::Bind::Vertical => span!(" ↕ "),
                key_event_handler::Bind::Enter => span!(" ↵ "),
                key_event_handler::Bind::Esc => span!(" Esc "),
                key_event_handler::Bind::Tab => span!(" ↹ "),
                key_event_handler::Bind::Key(x) => {
                    span!(format!(" {} ", if x == " " { "␣" } else { x }))
                }
                _ => "_".into(),
            }
            .bold()
            .fg(tailwind::AMBER.c600)
        };
        let binds = key_event_handler
            .get_key_binds_descriptions(self, (area.width / 10 * area.height) as usize);

        let num_items_per_row = (binds.len() as f64 / area.height as f64).ceil() as usize;
        let len_item = ((area.width - 2 * (num_items_per_row.saturating_sub(1) as u16)) as f32
            / num_items_per_row as f32)
            .floor() as u16;

        let verts = Layout::vertical(vec![constraint!(==1); area.height as usize]).split(area);
        let mut areas = verts.iter().flat_map(|&area| {
            Layout::horizontal(vec![constraint!(==len_item); num_items_per_row])
                .split(area)
                .iter()
                .copied()
                .collect_vec()
        });

        binds.into_iter().for_each(|x| {
            let bind = bind_to_string(&x.0);
            let desc = ellipsize_string(&x.1, len_item as usize - bind.width());
            frame.render_widget(
                line![bind, span!(desc).fg(tailwind::SLATE.c500)],
                areas.next().unwrap(),
            );
        });
    }
}
