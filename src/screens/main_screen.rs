use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use chrono::Datelike;
use itertools::Itertools;
use log::{error, info};
use nucleo_matcher::{Config as MatcherConfig, Matcher, pattern::Atom};
use ratatui::{
    Frame,
    crossterm::event::KeyModifiers,
    layout::{Layout, Offset, Position, Rect, Size},
    macros::{constraint, horizontal, line, span, text, vertical},
    style::{
        Color, Modifier, Styled, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Padding},
};
use ratatui_image::sliced::SignedPosition;
use ratatui_textarea::TextArea;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, IntoEnumIterator};
use throbber_widgets_tui::ThrobberState;

use crate::{
    config::Config,
    helpers::{add_padding, default_rc, ellipsize_string, ids_to_movies, is_between, wrap_text},
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{self, KeyEventHandler},
    load_file,
    screens::Screens,
    types::{Entry, FilterCriterion, FxIndexMap, Movie, Sort, pop_criterion},
    widgets,
};

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum ListID {
    TMDB(u32),
    Collection(u32),
    Custom(u32),
}
impl Default for ListID {
    fn default() -> Self {
        Self::Custom(0)
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub struct List {
    pub id:     ListID,
    pub name:   String,
    pub movies: Vec<u32>,
}
impl From<&[Entry]> for List {
    fn from(value: &[Entry]) -> Self {
        Self {
            id:     Default::default(),
            name:   "Watched Movies".into(),
            movies: value.iter().map(|x| x.movie_id).collect(),
        }
    }
}

#[derive(Default)]
pub struct PlaysTab {
    scroll_pos:        usize,
    alignment_bottom:  bool,
    num_visible_items: usize,
}

pub struct MainScreen {
    tick:                u64,
    tab:                 usize,
    item:                usize,
    pub sort:            Sort,
    pub drawing_images:  bool,
    pub sort_ascending:  bool,
    pub filter_criteria: Vec<FilterCriterion>,
    pub search_input:    TextArea<'static>,
    throbber_state:      ThrobberState,

    selected_list: ListID,
    lists:         Vec<List>,

    pub _config:         Rc<RefCell<Config>>,
    pub movies:          Rc<RefCell<FxIndexMap<u32, Movie>>>,
    pub watched:         Rc<RefCell<FxIndexMap<u32, Entry>>>,
    pub filtered_movies: Vec<Movie>,

    movies_list_scroll_pos:             usize,
    movies_list_selected_item:          usize,
    movies_list_alignment_bottom:       bool,
    movies_list_partially_visible_item: bool,
    movies_list_num_visible_items:      usize,
    movies_description_selected_tab:    usize,

    context_menu_pos:      Option<Position>,
    context_menu_selected: usize,

    movies_description_plays_tab:       PlaysTab,
    movies_description_overview_scroll: usize,
}

const MOVIE_WIDGET_HEIGHT: usize = 11;

impl MainScreen {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (
            Some(self.tab),
            Some(
                self.item
                    + if self.tab == 1 {
                        self.movies_description_selected_tab << 9
                    } else {
                        0
                    },
            ),
        )
    }

    pub fn new(home_dir: &PathBuf, _config: Rc<RefCell<Config>>) -> Self {
        Self {
            tick: 0,
            tab: 0,
            item: 0,
            sort: Sort::default(),
            drawing_images: false,
            sort_ascending: false,
            search_input: TextArea::default(),
            filter_criteria: vec![],
            throbber_state: ThrobberState::default(),
            _config,

            selected_list: Default::default(),
            lists: load_file!("lists", home_dir).unwrap_or(vec![]),

            movies: default_rc(),
            watched: default_rc(),
            filtered_movies: vec![],

            movies_list_scroll_pos: 0,
            movies_list_selected_item: 0,
            movies_list_alignment_bottom: false,
            movies_list_partially_visible_item: false,
            movies_list_num_visible_items: 0,
            movies_description_selected_tab: 0,

            context_menu_pos: None,
            context_menu_selected: 0,

            movies_description_plays_tab: PlaysTab::default(),
            movies_description_overview_scroll: 0,
        }
    }

    pub fn initialize(
        &mut self,
        movies: Rc<RefCell<FxIndexMap<u32, Movie>>>,
        watched: Rc<RefCell<FxIndexMap<u32, Entry>>>,
    ) {
        self.movies = movies;
        self.watched = watched;

        self.filter_sort_movies(None);
    }

    pub fn get_list_ids(&self) -> Vec<u32> {
        if matches!(&self.selected_list, ListID::Custom(0)) {
            self.watched.borrow().values().map(|x| x.movie_id).collect()
        } else {
            self.lists
                .iter()
                .find_map(|x| (x.id == *&self.selected_list).then(|| x.movies.clone()))
                .unwrap()
        }
    }

    fn get_list_movies(&self) -> Vec<Movie> {
        ids_to_movies(&self.get_list_ids(), &self.movies.borrow())
    }

    pub fn current_movie(&self) -> Option<&Movie> {
        self.filtered_movies.get(self.movies_list_selected_item)
    }

    pub fn goto_index(&mut self, index: isize) {
        let index = if index.is_negative() {
            (self.filtered_movies.len() as isize + index) as usize
        } else {
            (index as usize).min(self.filtered_movies.len() - 1)
        };

        self.movies_list_selected_item = index;
        self.movies_list_scroll_pos = self
            .movies_list_scroll_pos
            .min(self.movies_list_selected_item);
        if self.movies_list_selected_item - self.movies_list_scroll_pos
            >= self.movies_list_num_visible_items
        {
            self.movies_list_scroll_pos =
                self.movies_list_selected_item - self.movies_list_num_visible_items + 1;
        }
    }

    fn find_and_goto_movie(&mut self) {
        let search_text = &self.search_input.lines()[0];
        if search_text.is_empty() {
            return;
        }

        let mut conf = MatcherConfig::DEFAULT;
        conf.prefer_prefix = true;
        let mut matcher = Matcher::new(conf);
        let pattern = Atom::parse(
            search_text,
            nucleo_matcher::pattern::CaseMatching::Ignore,
            nucleo_matcher::pattern::Normalization::Never,
        );
        let mut scores = vec![];
        for movie in &self.filtered_movies {
            if let Some(score) = pattern.score(
                nucleo_matcher::Utf32Str::Ascii(
                    (movie.title.clone() + " " + &movie.release_date.year().to_string())
                        .to_string()
                        .as_bytes(),
                ),
                &mut matcher,
            ) {
                scores.push((score, movie));
            }
        }

        scores.sort_by_key(|x| x.0);
        scores.reverse();

        if let Some(&(_, movie)) = scores.first() {
            let index = self
                .filtered_movies
                .iter()
                .position(|x| x == movie)
                .unwrap();

            self.movies_list_selected_item = index;
            self.movies_list_scroll_pos = index
                .saturating_sub(self.movies_list_num_visible_items / 2)
                .min(
                    self.filtered_movies
                        .len()
                        .saturating_sub(self.movies_list_num_visible_items),
                );
            self.movies_list_alignment_bottom = false;
        }
    }

    fn filter_movies(&mut self) {
        let mut movies = self.get_list_movies();
        for criterion in &self.filter_criteria {
            match criterion {
                FilterCriterion::Title(name, _) => {
                    if name.is_empty() {
                        continue;
                    }
                    let mut conf = MatcherConfig::DEFAULT;
                    conf.prefer_prefix = true;
                    let mut matcher = Matcher::new(conf);
                    let pattern = Atom::parse(
                        name,
                        nucleo_matcher::pattern::CaseMatching::Ignore,
                        nucleo_matcher::pattern::Normalization::Never,
                    );
                    let mut scores = vec![];
                    for movie in &movies {
                        if let Some(score) = pattern.score(
                            nucleo_matcher::Utf32Str::Ascii(
                                (movie.title.clone()
                                    + " "
                                    + &movie.release_date.year().to_string())
                                    .to_string()
                                    .as_bytes(),
                            ),
                            &mut matcher,
                        ) {
                            scores.push((score, movie));
                        }
                    }

                    if let Sort::Relevance = self.sort {
                        scores.sort_by_key(|x| x.0);
                        if !self.sort_ascending {
                            scores.reverse();
                        }
                    }
                    movies = scores.iter().map(|&(_, movie)| movie.clone()).collect();
                }
                FilterCriterion::Actors(actors, contains_all, inverted) => {
                    movies = movies.into_iter().filter(|x| if *contains_all {actors.iter().all(|y| x.credits.cast.iter().map(|x| x.id).contains(y))} else {actors.iter().any(|y| x.credits.cast.iter().map(|x| x.id).contains(y))} ^ if *inverted {true} else {false}).collect();
                }
                FilterCriterion::Director(director, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            x.credits
                                .crew
                                .iter()
                                .filter_map(|x| (x.job_or_character == "Director").then_some(x.id))
                                .contains(director)
                                ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
                FilterCriterion::Genres(genres, contains_all, inverted) => {
                    movies = movies.into_iter().filter(|x| if *contains_all {genres.iter().all(|y| x.genres.contains(y))} else {genres.iter().any(|y| x.genres.contains(y))} ^ if *inverted {true} else {false}).collect();
                }
                FilterCriterion::Released(lower_bound, upper_bound, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            is_between(x.release_date.year() as u32, *lower_bound, *upper_bound)
                                ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
                FilterCriterion::FirstWatched(lower_bound, upper_bound, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            watched_borrowed
                                .get(&x.id)
                                .map(|y| {
                                    is_between(
                                        y.get_first_play().year() as u32,
                                        *lower_bound,
                                        *upper_bound,
                                    ) ^ if *inverted { true } else { false }
                                })
                                .unwrap_or(false)
                        })
                        .collect();
                }
                FilterCriterion::LastWatched(lower_bound, upper_bound, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            watched_borrowed
                                .get(&x.id)
                                .map(|y| {
                                    is_between(
                                        y.get_latest_play().year() as u32,
                                        *lower_bound,
                                        *upper_bound,
                                    ) ^ if *inverted { true } else { false }
                                })
                                .unwrap_or(false)
                        })
                        .collect();
                }
                FilterCriterion::Rating(rating, ordering, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            (x.get_external_rating().partial_cmp(rating).unwrap() == *ordering)
                                ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
                FilterCriterion::UserRating(rating, ordering, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            watched_borrowed
                                .get(&x.id)
                                .map(|y| {
                                    (y.get_user_rating().partial_cmp(rating).unwrap() == *ordering)
                                        ^ if *inverted { true } else { false }
                                })
                                .unwrap_or(false)
                        })
                        .collect();
                }
                FilterCriterion::Language(language, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            (*language == x.language) ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
                FilterCriterion::Country(country, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            (x.origin_country == *country) ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
                FilterCriterion::Certification(certifications, inverted) => {
                    movies = movies
                        .into_iter()
                        .filter(|x| {
                            certifications.contains(&x.certification)
                                ^ if *inverted { true } else { false }
                        })
                        .collect();
                }
            }
        }

        self.filtered_movies = movies;
    }

    fn sort_movies(&mut self) {
        match self.sort {
            Sort::UserRating => {
                self.filtered_movies.sort_by(|x, y| {
                    self.watched.borrow()[&x.id]
                        .get_user_rating()
                        .partial_cmp(&self.watched.borrow()[&y.id].get_user_rating())
                        .unwrap()
                });
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Rating => {
                self.filtered_movies
                    .sort_by(|a, b| a.partial_cmp(b).unwrap());
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Name => {
                self.filtered_movies.sort_by_key(|x| x.title.clone());
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::ReleaseDate => {
                self.filtered_movies
                    .sort_by_key(|x| x.release_date.year().to_string());
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::DateAdded => {
                self.filtered_movies
                    .sort_by_key(|x| self.watched.borrow()[&x.id].get_first_play().clone());
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::MostRecent => {
                self.filtered_movies
                    .sort_by_key(|x| self.watched.borrow()[&x.id].get_latest_play().clone());
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Relevance => (),
        }
    }

    pub fn filter_sort_movies(&mut self, keep_selected: Option<bool>) {
        let selected_movie_id = self.current_movie().map(|x| x.id).unwrap_or(u32::MAX);

        self.filter_movies();

        match self.sort {
            Sort::Relevance => {}
            _ => {
                self.sort_movies();
            }
        }

        if let Some(keep_selected) = keep_selected {
            if keep_selected {
                let pos = self
                    .filtered_movies
                    .iter()
                    .position(|x| x.id == selected_movie_id);
                if let Some(index) = pos {
                    self.movies_list_selected_item = index;

                    if self.movies_list_scroll_pos > index
                        || index >= self.movies_list_scroll_pos + self.movies_list_num_visible_items
                    {
                        self.movies_list_scroll_pos = index
                            .saturating_sub(self.movies_list_num_visible_items / 2)
                            .min(
                                self.filtered_movies
                                    .len()
                                    .saturating_sub(self.movies_list_num_visible_items),
                            );
                        self.movies_list_alignment_bottom = false;
                    }
                } else {
                    self.movies_list_selected_item = 0;
                    self.movies_list_scroll_pos = 0;
                }
            } else {
                self.movies_list_selected_item = 0;
                self.movies_list_scroll_pos = 0;
            }
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        if !self.search_input.is_empty() {
            key_event_handler.bind_esc((Some(0), None), "Clear search".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if let Sort::Relevance = main_screen.sort {
                        main_screen.sort = Sort::default();
                    }

                    main_screen.search_input = TextArea::from([""]);
                    let FilterCriterion::Title(_, filter) = pop_criterion!(
                        main_screen.filter_criteria,
                        FilterCriterion::Title(_, _),
                        FilterCriterion::Title(String::new(), false)
                    ) else {
                        unreachable!()
                    };
                    if filter {
                        main_screen.filter_sort_movies(Some(true));
                    }
                }
            });
        }

        if !self.filtered_movies.is_empty() {
            for tab in 0..=1 {
                key_event_handler.bind_key((Some(tab), None), 'A', "Add play".into(), |app, _| {
                    app.drawer.open_add_play_popup();
                });
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'R',
                    "Refetch details".into(),
                    |app, _| {
                        app.drawer.open_refetch_details_popup(
                            app.trakt_tokens.clone(),
                            app.punch_play_tokens.clone(),
                            app.tmdb_tokens.clone(),
                            app.omdb_tokens.clone(),
                        );
                    },
                );
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'e',
                    "Edit movie".into(),
                    |app, _| {
                        app.drawer.open_edit_movie_popup();
                    },
                );
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'd',
                    "Delete movie".into(),
                    |app, _| {
                        app.drawer.open_delete_movie_popup();
                    },
                );
            }
        }
        key_event_handler.bind_key((Some(0), None), 'a', "Add movie".into(), |app, _| {
            app.drawer.open_add_movie_popup(
                app.trakt_tokens.clone(),
                app.punch_play_tokens.clone(),
                app.tmdb_tokens.clone(),
                app.omdb_tokens.clone(),
            );
        });
        key_event_handler.bind_key((Some(0), None), 'F', "Advanced Filter".into(), |app, _| {
            app.drawer.open_advanced_filter_popup();
        });
        key_event_handler.bind_key((Some(0), None), ',', "Sort by".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 1;
            }
        });
        key_event_handler.bind_key((Some(0), None), '/', "Find".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 0;

                _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title("".into(), false));

                if !main_screen.search_input.is_empty() {
                    main_screen.search_input = TextArea::from([""]);
                    main_screen.filter_sort_movies(Some(true));
                }
            }
        });
        key_event_handler.bind_key((Some(0), None), 'f', "Filter".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 0;

                main_screen.sort = Sort::Relevance;
                _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title("".into(), true));

                if !main_screen.search_input.is_empty() {
                    main_screen.search_input = TextArea::from([""]);
                    main_screen.filter_sort_movies(Some(true));
                }
            }
        });

        let frame_area = frame.area();

        // let num_movies = ((frame_area.height - 5) as f32 / 9.0).floor() as usize;
        // let footer_height = (((frame_area.height - 5) % 9) % num_movies as u16) + 2;
        let [header, vert, _] = vertical![==3, >=1, ==2].areas(frame_area);

        let [description, list] = horizontal![==(vert.width * 3 / 8).max(0), >=0].areas(vert);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), header);

        self.drawing_images = false;
        self.render_movies_list(frame, image_renderer, key_event_handler, list);
        self.render_movie_description(frame, image_renderer, key_event_handler, description);
        self.render_header(frame, header, key_event_handler);

        if let Some(pos) = self.context_menu_pos {
            key_event_handler.clear();

            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                    }
                },
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Right,
                frame.area(),
                |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                    }
                },
            );

            key_event_handler.bind_esc((None, None), "Cancel".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });

            key_event_handler.bind_enter((None, None), "Choose".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    if main_screen.context_menu_selected == 0 {
                        app.drawer.open_add_play_popup();
                    } else if main_screen.context_menu_selected == 1 {
                        app.drawer.open_edit_movie_popup();
                    } else if main_screen.context_menu_selected == 2 {
                        app.drawer.open_delete_movie_popup();
                    }
                }
            });

            key_event_handler.bind_key((None, None), 'q', "Cancel".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });
            key_event_handler.bind_key((None, None), 'A', "Add play".into(), |app, _| {
                app.drawer.open_add_play_popup();

                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });
            key_event_handler.bind_key((None, None), 'R', "Refetch details".into(), |app, _| {
                app.drawer.open_refetch_details_popup(
                    app.trakt_tokens.clone(),
                    app.punch_play_tokens.clone(),
                    app.tmdb_tokens.clone(),
                    app.omdb_tokens.clone(),
                );

                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });
            key_event_handler.bind_key((None, None), 'e', "Edit movie".into(), |app, _| {
                app.drawer.open_edit_movie_popup();

                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });
            key_event_handler.bind_key((None, None), 'd', "Delete movie".into(), |app, _| {
                app.drawer.open_delete_movie_popup();

                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                }
            });

            key_event_handler.bind_vertical((None, None), "Navigate".into(), |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(false, _) => {
                            main_screen.context_menu_selected =
                                main_screen.context_menu_selected.saturating_sub(1);
                        }
                        key_event_handler::Data::Direction(true, _) => {
                            main_screen.context_menu_selected =
                                (main_screen.context_menu_selected + 1).min(2);
                        }
                        _ => (),
                    }
                }
            });

            let actions = vec!["Add play", "Refetch details", "Edit", "Delete"];
            let width = actions.iter().map(|x| x.len()).max().unwrap() as u16 + 4;
            let height = actions.len() as u16 + 2;

            let x = if pos.x + width - 1 >= frame.area().width {
                frame.area().width - width
            } else {
                pos.x
            };
            let y = if pos.y + height - 1 >= frame.area().height {
                frame.area().height - height
            } else {
                pos.y
            };

            let (mut mouse_area, len) = widgets::normal_popup(
                actions
                    .iter()
                    .map(|x| {
                        line!(" ", *x, " ")
                            .fg(material::INDIGO.c200)
                            .bg(material::INDIGO.c900)
                    })
                    .collect(),
                self.context_menu_selected,
                0,
                5,
                Position { x, y },
                width,
                frame,
                key_event_handler,
            );

            for i in 0..len {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if i == 0 {
                            app.drawer.open_add_play_popup();
                        } else if i == 1 {
                            app.drawer.open_refetch_details_popup(
                                app.trakt_tokens.clone(),
                                app.punch_play_tokens.clone(),
                                app.tmdb_tokens.clone(),
                                app.omdb_tokens.clone(),
                            );
                        } else if i == 2 {
                            app.drawer.open_edit_movie_popup();
                        } else if i == 3 {
                            app.drawer.open_delete_movie_popup();
                        }

                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.context_menu_pos = None;
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
            }
        }
    }

    fn render_header(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) -> Rect {
        let tab_selected = self.tab == 2;

        key_event_handler.bind_esc((Some(2), Some(0)), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                if let Sort::Relevance = main_screen.sort {
                    main_screen.sort = Sort::default();
                }

                main_screen.search_input = TextArea::from([""]);
                let FilterCriterion::Title(_, filter) = pop_criterion!(
                    main_screen.filter_criteria,
                    FilterCriterion::Title(_, _),
                    FilterCriterion::Title(String::new(), false)
                ) else {
                    unreachable!()
                };
                if filter {
                    main_screen.filter_sort_movies(Some(true));
                }
            }
        });
        key_event_handler.bind_esc((Some(2), None), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_tab((Some(2), Some(0)), "Change focus".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.item = 1;

                let FilterCriterion::Title(name, filter) = pop_criterion!(
                    main_screen.filter_criteria,
                    FilterCriterion::Title(_, _),
                    FilterCriterion::Title(String::new(), false)
                ) else {
                    unreachable!()
                };

                if name.is_empty() || !filter {
                    if let Sort::Relevance = main_screen.sort {
                        main_screen.sort = Sort::default();
                    }
                    main_screen.search_input = TextArea::from([""]);
                } else if filter {
                    main_screen
                        .filter_criteria
                        .push(FilterCriterion::Title(name, true));
                }
            }
        });
        key_event_handler.bind_tab((Some(2), None), "Change focus".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.item = 0;

                main_screen.sort = Sort::Relevance;
                main_screen.search_input = TextArea::from([""]);
                _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title("".into(), true));
                main_screen.filter_sort_movies(Some(true));
            }
        });

        key_event_handler.bind_enter((Some(2), Some(0)), "Confirm".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                let FilterCriterion::Title(name, filter) = pop_criterion!(
                    main_screen.filter_criteria,
                    FilterCriterion::Title(_, _),
                    FilterCriterion::Title(String::new(), false)
                ) else {
                    unreachable!()
                };

                if name.is_empty() || !filter {
                    if let Sort::Relevance = main_screen.sort {
                        main_screen.sort = Sort::default();
                    }
                    main_screen.search_input = TextArea::from([""]);
                } else if filter {
                    main_screen
                        .filter_criteria
                        .push(FilterCriterion::Title(name, true));
                }
            }
        });
        key_event_handler.bind_enter((Some(2), None), "Confirm".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_key((Some(2), Some(1)), ',', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_key((Some(2), Some(2)), ',', "Sort".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 1;
            }
        });
        key_event_handler.bind_key(
            (Some(2), Some(2)),
            ' ',
            "Toggle sort order".into(),
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.sort_ascending = !main_screen.sort_ascending;
                    main_screen.filter_sort_movies(Some(true));
                }
            },
        );
        key_event_handler.bind_key((Some(2), None), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_horizontal((Some(2), Some(1)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.item += 1;
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_horizontal((Some(2), Some(2)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.item -= 1;
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_vertical((Some(2), Some(1)), "Change sort".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.sort = Sort::from_repr(
                            (main_screen.sort as usize).checked_sub(1).unwrap_or(0),
                        )
                        .unwrap();
                    }
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.sort = Sort::from_repr(main_screen.sort as usize + 1)
                            .unwrap_or(main_screen.sort);

                        if main_screen.search_input.is_empty()
                            && matches!(main_screen.sort, Sort::Relevance)
                        {
                            main_screen.sort = Sort::from_repr(main_screen.sort as usize - 1)
                                .unwrap_or(main_screen.sort);
                        }
                    }
                    _ => (),
                }
                main_screen.filter_sort_movies(Some(true));
            }
        });
        key_event_handler.bind_vertical(
            (Some(2), Some(2)),
            "Change sort order".into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(false, _) => {
                            if !main_screen.sort_ascending {
                                main_screen.sort_ascending = true;
                                main_screen.filter_sort_movies(Some(true));
                            }
                        }
                        key_event_handler::Data::Direction(true, _) => {
                            if main_screen.sort_ascending {
                                main_screen.sort_ascending = false;
                                main_screen.filter_sort_movies(Some(true));
                            }
                        }
                        _ => (),
                    }
                }
            },
        );

        key_event_handler.bind_input_field((Some(2), Some(0)), "".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Key(key_event) => {
                        main_screen.search_input.input(key_event);

                        let FilterCriterion::Title(_, filter) = pop_criterion!(
                            main_screen.filter_criteria,
                            FilterCriterion::Title(_, _),
                            FilterCriterion::Title(String::new(), false)
                        ) else {
                            unreachable!()
                        };
                        main_screen.filter_criteria.push(FilterCriterion::Title(
                            main_screen.search_input.lines()[0].clone(),
                            filter,
                        ));
                        if filter {
                            main_screen.filter_sort_movies(Some(false));
                        } else {
                            main_screen.find_and_goto_movie();
                        }
                    }
                    _ => {}
                }
            }
        });

        let [_debug_area, input_area, _, sort_area, _, direction_area, _] =
            horizontal![>=1, <=25, ==1, <=16, ==1, ==3, ==1].areas(area);

        let filter = if let Some(FilterCriterion::Title(n, f)) =
            pop_criterion!(self.filter_criteria, FilterCriterion::Title(_, _))
        {
            if tab_selected || f {
                self.filter_criteria
                    .push(FilterCriterion::Title(n.clone(), f));
            } else {
                self.search_input = TextArea::from([""]);
            }

            if (tab_selected && self.item == 0) || !n.is_empty() {
                Some(f)
            } else {
                None
            }
        } else {
            None
        };

        widgets::input_field(
            tab_selected,
            self.item == 0,
            true,
            &mut self.search_input,
            ratatui_textarea::WrapMode::None,
            frame,
            input_area,
            match filter {
                Some(true) => " Filter ",
                Some(false) => " Find ",
                None => "",
            },
            "Search",
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            input_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 0;

                    let filter = if let Some(FilterCriterion::Title(n, f)) =
                        pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _))
                    {
                        main_screen
                            .filter_criteria
                            .push(FilterCriterion::Title(n, f));
                        true
                    } else {
                        false
                    };

                    if !filter {
                        main_screen.search_input = TextArea::from([""]);
                        let FilterCriterion::Title(_, _) = pop_criterion!(
                            main_screen.filter_criteria,
                            FilterCriterion::Title(_, _),
                            FilterCriterion::Title(String::new(), false)
                        ) else {
                            unreachable!()
                        };
                        let criterion = FilterCriterion::Title("".into(), true);
                        main_screen.filter_criteria.push(criterion);
                        main_screen.filter_sort_movies(Some(true));
                    }
                }
            },
        );

        widgets::dropdown(
            tab_selected,
            self.item == 1,
            frame,
            sort_area,
            ellipsize_string(self.sort.as_ref(), sort_area.width as usize - 4),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            sort_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 1;
                }
            },
        );

        if tab_selected && self.item == 1 {
            let mut items = Sort::iter()
                .map(|x| {
                    line!(
                        " ",
                        ellipsize_string(x.as_ref(), sort_area.width as usize - 2),
                        " "
                    )
                    .fg(material::INDIGO.c200)
                    .bg(material::INDIGO.c900)
                })
                .collect_vec();
            if self.search_input.is_empty() {
                _ = items.pop();

                if matches!(self.sort, Sort::Relevance) {
                    self.sort = Sort::default();
                }
            }

            let (mut mouse_area, len) = widgets::dropdown_popup(
                items,
                self.sort as usize,
                0,
                Sort::COUNT,
                sort_area,
                frame,
                key_event_handler,
            );
            for i in 0..len {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;
                            main_screen.sort = Sort::from_repr(i).unwrap();
                            main_screen.filter_sort_movies(Some(true));
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
            }
        }

        let direction_block =
            Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(if tab_selected {
                    if self.item == 2 {
                        material::BLUE.c600
                    } else {
                        material::INDIGO.c800
                    }
                } else {
                    tailwind::SLATE.c700
                });
        let direction = if self.sort_ascending { "⬆" } else { "⬇" };
        frame.render_widget(&direction_block, direction_area);
        frame.render_widget(
            line!(direction)
                .centered()
                .bold()
                .fg(if tab_selected {
                    if self.item == 2 {
                        material::TEAL.c100
                    } else {
                        material::INDIGO.c200
                    }
                } else {
                    material::GRAY.c400
                })
                .bg(if tab_selected {
                    if self.item == 2 {
                        material::BLUE.c600
                    } else {
                        material::INDIGO.c800
                    }
                } else {
                    tailwind::SLATE.c700
                }),
            direction_block.inner(direction_area),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            direction_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.sort_ascending = !main_screen.sort_ascending;
                    main_screen.filter_sort_movies(Some(true));
                }
            },
        );

        return sort_area;
    }

    fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
    ) {
        if self.filtered_movies.len() > 0 {
            let num_visible_items = self.movies_list_num_visible_items;

            key_event_handler.bind_tab((Some(0), None), "Change focus".into(), |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(true, _) => {
                            main_screen.tab += 1;
                            if main_screen.tab > 1 {
                                main_screen.tab = 0;
                            }
                        }
                        key_event_handler::Data::Direction(false, _) => {
                            main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                        }
                        _ => (),
                    }
                }
            });

            key_event_handler.bind_key((Some(0), None), "gg", "Jump to top".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(0);
                }
            });
            key_event_handler.bind_key((Some(0), None), 'G', "Jump to bottom".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(-1);
                }
            });

            key_event_handler.bind_vertical((Some(0), None), "Scroll".into(), move |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(true, modifiers) => {
                            if modifiers.contains(KeyModifiers::SHIFT) {
                                main_screen.goto_index(
                                    (main_screen.movies_list_selected_item
                                        + num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.movies_list_selected_item =
                                    (main_screen.movies_list_selected_item + 1)
                                        .min(main_screen.filtered_movies.len().saturating_sub(1));

                                if main_screen.movies_list_selected_item
                                    < main_screen.movies_list_scroll_pos
                                {
                                    main_screen.movies_list_scroll_pos = main_screen
                                        .movies_list_selected_item
                                        .min(main_screen.filtered_movies.len().saturating_sub(
                                            main_screen.movies_list_num_visible_items,
                                        ));
                                } else if main_screen.movies_list_selected_item
                                    - main_screen.movies_list_scroll_pos
                                    >= main_screen.movies_list_num_visible_items
                                {
                                    main_screen.movies_list_scroll_pos = main_screen
                                        .movies_list_selected_item
                                        - main_screen.movies_list_num_visible_items
                                        + 1;
                                }
                            }
                        }
                        key_event_handler::Data::Direction(false, modifiers) => {
                            if modifiers.contains(KeyModifiers::SHIFT) {
                                main_screen.goto_index(
                                    main_screen
                                        .movies_list_selected_item
                                        .saturating_sub(num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.movies_list_selected_item =
                                    main_screen.movies_list_selected_item.saturating_sub(1);

                                if main_screen.movies_list_selected_item
                                    < main_screen.movies_list_scroll_pos
                                {
                                    main_screen.movies_list_scroll_pos = main_screen
                                        .movies_list_selected_item
                                        .min(main_screen.filtered_movies.len().saturating_sub(
                                            main_screen.movies_list_num_visible_items,
                                        ));
                                } else if main_screen.movies_list_selected_item
                                    - main_screen.movies_list_scroll_pos
                                    >= main_screen.movies_list_num_visible_items
                                {
                                    main_screen.movies_list_scroll_pos = main_screen
                                        .movies_list_selected_item
                                        - main_screen.movies_list_num_visible_items
                                        + 1;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });
        }

        if self.movies_list_selected_item >= self.filtered_movies.len() {
            self.movies_list_selected_item = self.filtered_movies.len().saturating_sub(1);
            self.movies_list_scroll_pos = self
                .movies_list_selected_item
                .saturating_sub(self.movies_list_num_visible_items.saturating_sub(1));
        }

        let num_visible_movies = area.height as usize / MOVIE_WIDGET_HEIGHT;
        let partially_visible_movie_height =
            area.height as usize - num_visible_movies * MOVIE_WIDGET_HEIGHT;
        let render_partially_visible_movie = partially_visible_movie_height > 0;
        if self.movies_list_num_visible_items
            != num_visible_movies + if render_partially_visible_movie { 1 } else { 0 }
            || self.movies_list_partially_visible_item != render_partially_visible_movie
        {
            let selected_movie_index = self
                .movies_list_selected_item
                .saturating_sub(self.movies_list_scroll_pos)
                .saturating_sub(
                    if self.movies_list_partially_visible_item && self.movies_list_alignment_bottom
                    {
                        1
                    } else {
                        0
                    },
                )
                .min(num_visible_movies.saturating_sub(1));

            if self
                .movies_list_selected_item
                .saturating_sub(selected_movie_index)
                == 0
            {
                self.movies_list_scroll_pos = 0;
                self.movies_list_alignment_bottom = false;
            } else {
                self.movies_list_scroll_pos = self
                    .movies_list_selected_item
                    .saturating_sub(selected_movie_index)
                    - if render_partially_visible_movie && self.movies_list_alignment_bottom {
                        1
                    } else {
                        0
                    };
            }
        }
        self.movies_list_num_visible_items =
            num_visible_movies + if render_partially_visible_movie { 1 } else { 0 };
        self.movies_list_partially_visible_item = render_partially_visible_movie;

        if self.movies_list_scroll_pos + self.movies_list_num_visible_items
            > self.filtered_movies.len()
        {
            self.movies_list_scroll_pos = self
                .filtered_movies
                .len()
                .saturating_sub(self.movies_list_num_visible_items);
            self.movies_list_alignment_bottom = true;
        }
        if self.movies_list_partially_visible_item {
            if self.filtered_movies.len() <= num_visible_movies {
                self.movies_list_alignment_bottom = false;
            } else if self.movies_list_selected_item == self.movies_list_scroll_pos {
                self.movies_list_alignment_bottom = false;
            } else if self
                .movies_list_selected_item
                .saturating_sub(self.movies_list_scroll_pos)
                == self.movies_list_num_visible_items - 1
            {
                self.movies_list_alignment_bottom = true;
            }
        }

        let [movies_area, scrollbar_area] = horizontal![>=0, ==1].areas(area);
        let mut remaining_area = movies_area;
        for i in 0..self.movies_list_num_visible_items {
            let [area, remaining] = if self.movies_list_partially_visible_item
                && ((i == 0 && self.movies_list_alignment_bottom)
                    || (i == self.movies_list_num_visible_items - 1
                        && !self.movies_list_alignment_bottom))
            {
                vertical![==partially_visible_movie_height as u16, >= 0]
            } else {
                vertical![==MOVIE_WIDGET_HEIGHT as u16, >= 0]
            }
            .areas(remaining_area);
            remaining_area = remaining;

            if !self.filtered_movies.is_empty()
                && i + self.movies_list_scroll_pos < self.filtered_movies.len()
            {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;

                            main_screen.movies_list_selected_item =
                                i + main_screen.movies_list_scroll_pos;
                        }
                    },
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Right,
                    area,
                    move |app, data| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;

                            main_screen.movies_list_selected_item =
                                i + main_screen.movies_list_scroll_pos;

                            if let key_event_handler::Data::Mouse(mouse_event) = data {
                                main_screen.context_menu_pos =
                                    Some(Position::new(mouse_event.column, mouse_event.row));
                                main_screen.context_menu_selected = 0;
                            }
                        }
                    },
                );

                self.draw_movie_widget(i, frame, image_renderer, area);
            } else {
                frame.render_widget(
                    Block::new().bg(if i & 1 == 1 {
                        tailwind::NEUTRAL.c900
                    } else {
                        tailwind::STONE.c900
                    }),
                    area,
                );
            }
        }

        if self.filtered_movies.len() > num_visible_movies {
            widgets::scroll_bar(
                self.filtered_movies.len() + self.movies_list_partially_visible_item as usize,
                self.movies_list_scroll_pos
                    + (self.movies_list_partially_visible_item && self.movies_list_alignment_bottom)
                        as usize,
                self.movies_list_num_visible_items,
                frame,
                scrollbar_area,
            );

            if self.movies_list_scroll_pos > 0 {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    scrollbar_area.resize(Size::new(1, 1)),
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.movies_list_scroll_pos -= 1;
                        }
                    },
                );
            }
            if self.movies_list_scroll_pos
                < self
                    .filtered_movies
                    .len()
                    .saturating_sub(self.movies_list_num_visible_items - 1)
            {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    scrollbar_area
                        .resize(Size::new(1, 1))
                        .offset(Offset::new(0, scrollbar_area.height as i32 - 1)),
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.movies_list_scroll_pos += 1;
                        }
                    },
                );
            }
        }
    }

    fn draw_movie_widget(
        &mut self,
        id: usize,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        area: Rect,
    ) {
        let is_partially_visible = MOVIE_WIDGET_HEIGHT > area.height as usize;
        let movie_index = self.movies_list_scroll_pos + id;
        let selected = self.movies_list_selected_item == movie_index;
        let tab_selected = self.tab == 0;
        let alt = movie_index & 1 == 1;
        let movie = &self.filtered_movies[movie_index];

        let (background, text) = if selected {
            if tab_selected {
                (tailwind::EMERALD.c800, tailwind::EMERALD.c200)
            } else {
                (tailwind::TEAL.c900, tailwind::BLUE.c200)
            }
        } else if !alt {
            (tailwind::ZINC.c800, material::BLUE_GRAY.c200)
        } else {
            (tailwind::GRAY.c800, material::GRAY.c400)
        };

        frame.render_widget(Block::new().bg(background).fg(text), area);

        let vert_lay = add_padding(
            area,
            if is_partially_visible {
                Padding::new(
                    2,
                    2,
                    if self.movies_list_alignment_bottom { 0 } else { 1 },
                    if self.movies_list_alignment_bottom { 1 } else { 0 },
                )
            } else {
                Padding::proportional(1)
            },
        );
        let poster_width = ((MOVIE_WIDGET_HEIGHT - 2) * 2 / 3) as u16 * 2;
        let [poster_area, _, description_area] =
            horizontal![==poster_width, ==1, >=0].areas(vert_lay);
        let highlight_area = area
            .resize(Size::new(2, area.height.saturating_sub(2)))
            .offset(Offset::new(0, 1));

        let rating = self.watched.borrow()[&movie.id].get_user_rating();
        let rating_color = if rating >= 9.0 {
            tailwind::SKY.c400
        } else if rating >= 8.0 {
            tailwind::GREEN.c500
        } else if rating >= 7.5 {
            tailwind::LIME.c400
        } else if rating >= 7.0 {
            material::AMBER.c400
        } else if rating >= 6.0 {
            material::DEEP_ORANGE.c300
        } else {
            material::RED.c400
        };

        let mut description_lines: Vec<Line<'_>> = vec![];

        const TITLE_LINES: usize = 2;
        let mut title_lines = wrap_text(&movie.title, description_area.width as usize - 4);
        for _ in 0..(TITLE_LINES.saturating_sub(title_lines.len())) {
            description_lines.push("".into());
        }
        title_lines.reverse();
        for _ in 0..(TITLE_LINES.min(title_lines.len()) - 1) {
            description_lines.push(title_lines.pop().unwrap().bold().into());
        }
        description_lines.push(
            (ellipsize_string(
                &title_lines.pop().unwrap(),
                description_area.width as usize - 5,
            )
            .bold()
                + " ".into()
                + movie.release_date.year().to_string().italic())
            .into(),
        );

        description_lines.push(
            format!("{:.1}", rating)
                .set_style(rating_color)
                .bold()
                .into(),
        );

        const TAGLINE_LINES: usize = 2;
        let mut tagline_lines = wrap_text(&movie.tagline, description_area.width as usize);
        for _ in 0..(TAGLINE_LINES.saturating_sub(tagline_lines.len())) {
            description_lines.push("".into());
        }
        tagline_lines.reverse();
        for _ in 0..TAGLINE_LINES.min(tagline_lines.len()) {
            description_lines.push(tagline_lines.pop().unwrap().into());
        }

        let areas = Layout::vertical(vec![constraint!(==1); description_area.height as usize])
            .split(description_area);
        for i in (0..description_area.height).rev() {
            let index = if is_partially_visible {
                if self.movies_list_alignment_bottom {
                    i + (MOVIE_WIDGET_HEIGHT as u16 - 1 - area.height)
                } else {
                    i
                }
            } else {
                i
            } as usize;

            let area = areas[i as usize];
            if index == 0 {
                frame.render_widget(
                    line!(format!("#{}", movie_index + 1))
                        .right_aligned()
                        .bold()
                        .fg(if selected {
                            tailwind::GRAY.c200
                        } else {
                            tailwind::GRAY.c400
                        }),
                    area,
                )
            }
            if let Some(index) =
                (description_lines.len() - 1).checked_sub(MOVIE_WIDGET_HEIGHT - 3 - index)
            {
                frame.render_widget(&description_lines[index], area)
            }
        }

        // let unfocused_rating_color = if rating >= 9.0 {
        //     tailwind::SKY.c600
        // } else if rating >= 8.0 {
        //     tailwind::GREEN.c700
        // } else if rating >= 7.5 {
        //     tailwind::LIME.c700
        // } else if rating >= 7.0 {
        //     material::YELLOW.c700
        // } else if rating >= 6.0 {
        //     tailwind::AMBER.c600
        // } else {
        //     material::DEEP_ORANGE.c800
        // };
        if tab_selected && selected {
            frame.render_widget(
                text![span!("▐"); highlight_area.height as usize]
                // text!["1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c"]
                    .fg(
                    // if tab_selected {
                        rating_color
                    // } else {
                    //     unfocused_rating_color
                    // },
                ),
                highlight_area,
            );
        }

        self.drawing_images |= !image_renderer.draw_image(
            ImageID::Movie(self.filtered_movies[movie_index].id, false),
            poster_area,
            if is_partially_visible {
                Some(SignedPosition {
                    x: 0,
                    y: if self.movies_list_alignment_bottom {
                        -(MOVIE_WIDGET_HEIGHT as i16 - 2 - poster_area.height as i16)
                    } else {
                        0
                    },
                })
            } else {
                None
            },
            &mut self.throbber_state,
            frame,
        );
        // frame.render_widget(
        //     line!("1 2 3 4 5 6 7 8 9 a b c d e f "),
        //     poster_area.offset(Offset { x: 0, y: -1 }),
        // );
    }

    fn render_movie_description(
        &mut self,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
    ) {
        const TABS: [&str; 2] = ["Overview", "Plays"];
        const TABS_COUNT: usize = TABS.len();

        key_event_handler.bind_tab((Some(1), None), "Change focus".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.tab += 1;
                        if main_screen.tab > 1 {
                            main_screen.tab = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }

                main_screen.item = 0;
            }
        });

        key_event_handler.bind_horizontal((Some(1), None), "Change tab".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.movies_description_selected_tab =
                            (main_screen.movies_description_selected_tab + 1).min(TABS_COUNT - 1);
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.movies_description_selected_tab = main_screen
                            .movies_description_selected_tab
                            .checked_sub(1)
                            .unwrap_or(0);
                    }
                    _ => (),
                }
            }
        });

        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 1;
                    main_screen.item = 0;
                }
            },
        );

        let description_selected = self.tab == 1;
        let movie = if self.filtered_movies.is_empty() {
            None
        } else {
            Some(&self.filtered_movies[self.movies_list_selected_item].clone())
        };

        let inner = add_padding(area, Padding::proportional(1));
        let backdrop_height = (inner.width as f32 * 9.0 / 16.0).ceil() as u16 >> 1;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==8, >=1].areas(inner);

        if let Some(movie) = movie {
            let [title_area, ratings_area, _, tabs_area] =
                vertical![==3, ==2, ==1, ==2].areas(title_area);

            let mut name = movie.title.clone();
            name = ellipsize_string(&name, title_area.width as usize);

            let rating = self.watched.borrow()[&movie.id].get_user_rating();
            let user_rating_widget_bg = if rating >= 9.0 {
                tailwind::SKY.c400
            } else if rating >= 8.0 {
                tailwind::GREEN.c500
            } else if rating >= 7.5 {
                tailwind::LIME.c400
            } else if rating >= 7.0 {
                material::AMBER.c400
            } else if rating >= 6.0 {
                tailwind::ORANGE.c500
            } else {
                material::RED.c400
            };
            let user_rating_widget_fg = if rating >= 7.0 {
                tailwind::STONE.c950
            } else {
                tailwind::STONE.c200
            };
            let user_rating_widget = line![
                span!("").fg(user_rating_widget_bg),
                span!(format!("{rating:.1}"))
                    .bold()
                    .fg(user_rating_widget_fg)
                    .bg(user_rating_widget_bg),
                span!("").fg(user_rating_widget_bg)
            ];

            frame.render_widget(
                line![
                    span!("     "),
                    name.clone().bold(),
                    span!(" "),
                    movie.release_date.year().to_string().italic()
                ],
                title_area
                    .resize(Size::new(title_area.width, 1))
                    .centered(constraint!(==name.len() as u16 + 10), constraint!(==1)),
            );
            frame.render_widget(
                user_rating_widget,
                add_padding(title_area, Padding::top(1))
                    .resize(Size::new(title_area.width, 1))
                    .centered(constraint!(==5), constraint!(==1)),
            );
            self.draw_ratings(movie, frame, ratings_area);

            const BGS: [Color; 2] = [material::GREEN.c600, material::LIGHT_BLUE.c600];
            const FGS: [Color; 2] = [material::BLUE.c100, material::YELLOW.c100];
            const _BGS: [Color; 2] = [material::TEAL.c800, material::INDIGO.c600];
            const _FGS: [Color; 2] = [material::BLUE_GRAY.c200, material::BLUE_GRAY.c200];
            let mut tabs = TABS
                .iter()
                .enumerate()
                .flat_map(|(i, &x)| {
                    [
                        span!(format!(" {} ", x))
                            .fg(if description_selected { FGS[i] } else { _FGS[i] })
                            .bg(if description_selected { BGS[i] } else { _BGS[i] })
                            .add_modifier(if i == self.movies_description_selected_tab {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                        " ".into(),
                    ]
                })
                .take(TABS.len() * 2 - 1)
                .collect_vec();
            let mut mouse_area = tabs_area;
            for (i, tab) in tabs.iter_mut().enumerate() {
                if i & 1 == 1 {
                    mouse_area = mouse_area.offset(Offset { x: 1, y: 0 });
                    continue;
                }
                mouse_area = mouse_area.resize(Size {
                    width:  tab.width() as u16,
                    height: 1,
                });

                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 1;
                            main_screen.item = 0;
                            main_screen.movies_description_selected_tab = i / 2;
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset {
                    x: mouse_area.width as i32,
                    y: 0,
                });
            }
            frame.render_widget(
                text![
                    tabs,
                    line!("🮂".repeat(title_area.width as usize)).fg(if description_selected {
                        BGS[self.movies_description_selected_tab]
                    } else {
                        _BGS[self.movies_description_selected_tab]
                    }),
                ],
                tabs_area,
            );

            match self.movies_description_selected_tab {
                0 => {
                    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), description_area);

                    let mut overview_lines =
                        wrap_text(&movie.overview, description_area.width as usize)
                            .into_iter()
                            .map(|x| line!(x))
                            .collect_vec();

                    let genres_widgets = movie
                        .genres
                        .iter()
                        .sorted_by(|a, b| a.len().cmp(&b.len()))
                        .map(|genre| {
                            vec![
                                span!("").fg(tailwind::SLATE.c100),
                                span!(genre)
                                    .bold()
                                    .fg(tailwind::SLATE.c950)
                                    .bg(tailwind::SLATE.c100),
                                span!("").fg(tailwind::SLATE.c100),
                            ]
                        });
                    let mut genres_lines = vec![];
                    for widget in genres_widgets {
                        let mut line: Vec<Span<'_>> = genres_lines.pop().unwrap_or(vec![]);

                        if widget.iter().fold(0, |a, b| a + b.content.chars().count())
                            + line.iter().fold(0, |a, b| a + b.content.chars().count())
                            + if !line.is_empty() { 1 } else { 0 }
                            <= description_area.width as usize
                        {
                            if !line.is_empty() {
                                line.push(" ".into());
                            }

                            line.extend(widget);
                            genres_lines.push(line);
                        } else {
                            genres_lines.push(line);
                            genres_lines.push(widget);
                        }
                    }

                    overview_lines.push(line!());
                    overview_lines.extend(
                        genres_lines
                            .into_iter()
                            .map(|x| Line::from_iter(x).centered()),
                    );
                    self.movies_description_overview_scroll =
                        self.movies_description_overview_scroll.min(
                            overview_lines
                                .len()
                                .saturating_sub(description_area.height as usize),
                        );
                    let text = Text::from_iter(
                        overview_lines.split_off(self.movies_description_overview_scroll),
                    );

                    frame.render_widget(text, description_area);

                    key_event_handler.bind_vertical(
                        (Some(1), Some(self.movies_description_selected_tab << 9)),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Screens::MainScreen(main_screen)) =
                                app.drawer.current_screen.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Direction(false, _) => {
                                        main_screen.movies_description_overview_scroll =
                                            main_screen
                                                .movies_description_overview_scroll
                                                .saturating_sub(1);
                                    }
                                    key_event_handler::Data::Direction(true, _) => {
                                        main_screen.movies_description_overview_scroll += 1;
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                1 => self.draw_plays_tab(key_event_handler, movie, frame, description_area),
                _ => (),
            };
        }

        // frame.render_widget(Block::new().bg(tailwind::SLATE.c700), backdrop_area);
        self.drawing_images |= !image_renderer.draw_image(
            ImageID::Movie(self.current_movie().unwrap().id, true),
            backdrop_area,
            None,
            &mut self.throbber_state,
            frame,
        );
    }

    fn draw_ratings(&self, movie: &Movie, frame: &mut Frame, area: Rect) {
        let imdb_colors = (
            Color::Rgb(245, 197, 24),
            Color::Black,
            Color::Rgb(250, 225, 120),
        );
        let trakt_colors = (
            Color::Rgb(165, 61, 185),
            Color::White,
            Color::Rgb(230, 140, 245),
        );
        let letterboxd_colors = (
            Color::Rgb(0, 192, 48),
            Color::Black,
            Color::Rgb(115, 226, 122),
        );
        let tmdb_colors = (
            Color::Rgb(42, 187, 209),
            Color::Black,
            Color::Rgb(140, 205, 215),
        );
        let popcorn_colors = (
            Color::Rgb(255, 114, 33),
            Color::White,
            Color::Rgb(242, 165, 121),
        );
        let tomatoes_colors = (
            Color::Rgb(216, 44, 60),
            Color::White,
            Color::Rgb(247, 100, 103),
        );

        let mut ratings = vec![];
        if movie.external_ratings.imdb.0 > 0.0 {
            ratings.push(("imdb", format!("{:.1}", movie.external_ratings.imdb.0)));
        }
        if movie.external_ratings.trakt.0 > 0 {
            ratings.push(("trakt", movie.external_ratings.trakt.0.to_string()));
        }
        if movie.external_ratings.letterboxd.0 > 0.0 {
            ratings.push((
                "letterboxd",
                format!("{:.1}", movie.external_ratings.letterboxd.0),
            ));
        }
        if movie.external_ratings.tmdb.0 > 0.0 {
            ratings.push(("tmdb", format!("{:.1}", movie.external_ratings.tmdb.0)));
        }
        if movie.external_ratings.popcorn.0 > 0 {
            ratings.push(("popcorn", movie.external_ratings.popcorn.0.to_string()));
        }
        if movie.external_ratings.tomatoes.0 > 0 {
            ratings.push(("tomatoes", movie.external_ratings.tomatoes.0.to_string()));
        }

        if ratings.is_empty() {
            frame.render_widget(line!("NA").centered(), area);

            return;
        }

        let widget_areas = Layout::horizontal(vec![constraint!(==5); ratings.len()])
            .flex(ratatui::layout::Flex::SpaceEvenly)
            .split(add_padding(area, Padding::top(1)));
        let mut widgets = vec![];
        let mut labels = line!();
        for (name, rating) in ratings {
            let (bg, fg) = if name == "imdb" {
                labels.push_span(span!("IMDB").fg(imdb_colors.2));

                (imdb_colors.0, imdb_colors.1)
            } else if name == "trakt" {
                labels.push_span(span!("Trakt").fg(trakt_colors.2));

                (trakt_colors.0, trakt_colors.1)
            } else if name == "letterboxd" {
                labels.push_span(span!("Letterboxd").fg(letterboxd_colors.2));

                (letterboxd_colors.0, letterboxd_colors.1)
            } else if name == "tmdb" {
                labels.push_span(span!("TMDB").fg(tmdb_colors.2));

                (tmdb_colors.0, tmdb_colors.1)
            } else if name == "popcorn" {
                labels.push_span(span!("Popcorn").fg(popcorn_colors.2));

                (popcorn_colors.0, popcorn_colors.1)
            } else if name == "tomatoes" {
                labels.push_span(span!("Tomatoes").fg(tomatoes_colors.2));

                (tomatoes_colors.0, tomatoes_colors.1)
            } else {
                continue;
            };

            widgets.push(vec!["".fg(bg), rating.bg(bg).fg(fg).bold(), "".fg(bg)]);
        }

        for ((widget, label), &area) in widgets
            .into_iter()
            .zip(labels.into_iter())
            .zip(widget_areas.into_iter())
        {
            frame.render_widget(label, area.offset(Offset::new(0, -1)));
            frame.render_widget(Line::from_iter(widget), area);
        }
    }

    fn draw_plays_tab(
        &mut self,
        key_event_handler: &mut KeyEventHandler,
        movie: &Movie,
        frame: &mut Frame,
        area: Rect,
    ) {
        let movie_plays = &self.watched.borrow()[&movie.id].history;
        let tab_selected = self.tab == 1;
        let num_plays = movie_plays.len();
        let num_visible_plays = area.height as usize / 3;
        let partially_visible_play_height = area.height as usize - num_visible_plays * 3;
        let render_partially_visible_play = partially_visible_play_height > 0;
        self.movies_description_plays_tab.num_visible_items =
            num_visible_plays + if render_partially_visible_play { 1 } else { 0 };

        if num_plays > num_visible_plays {
            key_event_handler.bind_vertical(
                (Some(1), Some(self.movies_description_selected_tab << 9)),
                "Scroll".into(),
                move |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Direction(false, _) => {
                                if main_screen.movies_description_plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description_plays_tab.alignment_bottom =
                                        false;
                                } else {
                                    main_screen.movies_description_plays_tab.scroll_pos =
                                        main_screen
                                            .movies_description_plays_tab
                                            .scroll_pos
                                            .saturating_sub(1);
                                }
                            }
                            key_event_handler::Data::Direction(true, _) => {
                                if !main_screen.movies_description_plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description_plays_tab.alignment_bottom =
                                        true;
                                } else {
                                    main_screen.movies_description_plays_tab.scroll_pos += 1;
                                }
                            }
                            _ => (),
                        }
                    }
                },
            );

            self.movies_description_plays_tab.scroll_pos =
                0.max(self.movies_description_plays_tab.scroll_pos.min(
                    num_plays.saturating_sub(self.movies_description_plays_tab.num_visible_items),
                ));
            self.movies_description_plays_tab.alignment_bottom =
                self.movies_description_plays_tab.alignment_bottom & render_partially_visible_play;
        } else {
            self.movies_description_plays_tab.scroll_pos = 0;
            self.movies_description_plays_tab.alignment_bottom = false;
        }

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), area);

        let mut remaining_area = add_padding(area, Padding::left(1));
        for i in 0..self.movies_description_plays_tab.num_visible_items {
            let [area, remaining] = if render_partially_visible_play
                && i == 0
                && self.movies_description_plays_tab.alignment_bottom
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else if render_partially_visible_play
                && i == self.movies_description_plays_tab.num_visible_items - 1
                && !self.movies_description_plays_tab.alignment_bottom
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else {
                vertical![==3, >= 0]
            }
            .areas(remaining_area);

            if self.movies_description_plays_tab.scroll_pos + i < num_plays {
                let partially_visible = area.height < 3;
                let play =
                    &movie_plays[num_plays - self.movies_description_plays_tab.scroll_pos - i - 1];

                let alternate = i & 1 == 1;
                let latest = self.movies_description_plays_tab.scroll_pos + i == 0;
                let last = self.movies_description_plays_tab.scroll_pos + i == num_plays - 1;

                frame.render_widget(
                    Block::new().bg(if latest {
                        if tab_selected {
                            tailwind::ZINC.c600
                        } else {
                            tailwind::ZINC.c700
                        }
                    } else if !alternate {
                        if tab_selected {
                            tailwind::GRAY.c600
                        } else {
                            tailwind::GRAY.c700
                        }
                    } else {
                        if tab_selected {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    }),
                    add_padding(area, Padding::left(2)),
                );

                let areas =
                    Layout::vertical(vec![constraint!(==1); area.height as usize]).split(area);

                let rating_color = if play.rating >= 9.0 {
                    tailwind::SKY.c400
                } else if play.rating >= 8.0 {
                    tailwind::GREEN.c500
                } else if play.rating >= 7.5 {
                    tailwind::LIME.c400
                } else if play.rating >= 7.0 {
                    material::AMBER.c400
                } else if play.rating >= 6.0 {
                    material::DEEP_ORANGE.c300
                } else {
                    material::RED.c400
                };

                for i in 0..area.height {
                    let index = if partially_visible {
                        if self.movies_description_plays_tab.alignment_bottom {
                            i + 3 - area.height
                        } else {
                            i
                        }
                    } else {
                        i
                    };
                    match index {
                        0 =>
                            if !latest {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            } else {
                                frame.render_widget(
                                    span!("▔".repeat(area.width as usize)).fg(if tab_selected {
                                        tailwind::ZINC.c500
                                    } else {
                                        tailwind::ZINC.c600
                                    }),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            },
                        1 => {
                            frame.render_widget(
                                span!("●").fg(if latest {
                                    if tab_selected {
                                        material::YELLOW.c800
                                    } else {
                                        material::CYAN.c500
                                    }
                                } else {
                                    if tab_selected {
                                        material::CYAN.c500
                                    } else {
                                        material::CYAN.c700
                                    }
                                }),
                                areas[i as usize],
                            );
                            frame.render_widget(
                                line![
                                    format!("{:.1}", play.rating).fg(rating_color).add_modifier(
                                        if latest { Modifier::BOLD } else { Modifier::empty() }
                                    ),
                                    span!(" @ "),
                                    play.date
                                        .format("%d/%m/%Y %H:%M")
                                        .to_string()
                                        .fg(if latest {
                                            if tab_selected {
                                                material::YELLOW.c700
                                            } else {
                                                material::CYAN.c600
                                            }
                                        } else {
                                            if tab_selected {
                                                material::CYAN.c500
                                            } else {
                                                material::CYAN.c700
                                            }
                                        }),
                                ],
                                add_padding(areas[i as usize], Padding::left(4)),
                            );
                        }
                        2 => {
                            if !last {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            }
                            if latest {
                                frame.render_widget(
                                    span!("▁".repeat(area.width as usize)).fg(if tab_selected {
                                        tailwind::ZINC.c500
                                    } else {
                                        tailwind::ZINC.c600
                                    }),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            }
                        }
                        _ => (),
                    }
                }
            }

            remaining_area = remaining;
        }
    }
}
