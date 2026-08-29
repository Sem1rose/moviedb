use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use itertools::{Itertools, izip};
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
use strum::IntoEnumIterator;
use throbber_widgets_tui::ThrobberState;

use crate::{
    config::Config,
    helpers::{self, SuperOrd},
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{self, KeyEventHandler},
    load_file,
    screens::Screens,
    tokens::{PunchPlayTokens, SimklTokens, TMDBTokens},
    types::{
        Entry, FilterCriterion, FxIndexMap, List, ListID, ListItem, Movie, RatingSource, Sort,
        pop_criterion,
    },
    widgets::{self, ContextMenu, ScrollView},
};

#[derive(Default)]
pub struct PlaysTab {
    scroll_pos:        usize,
    alignment_bottom:  bool,
    num_visible_items: usize,
}

const DESCRIPTION_TABS: [&str; 2] = ["Overview", "Plays"];
#[derive(Default)]
pub struct MoviesDescription {
    pub available_tabs:  Vec<usize>,
    pub selected_tab:    usize,
    pub plays_tab:       PlaysTab,
    pub overview_scroll: usize,
}

const CONTEXT_MENU_MODEL: [&str; 5] = [
    "Add play",
    "Edit",
    "Manage plays",
    "Refetch details",
    "Delete",
];
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

    pub selected_list: ListID,
    pub lists:         FxIndexMap<ListID, List>,

    pub _config:         Rc<RefCell<Config>>,
    pub movies:          Rc<RefCell<FxIndexMap<u32, Movie>>>,
    pub watched:         Rc<RefCell<FxIndexMap<u32, Entry>>>,
    pub filtered_movies: Vec<Movie>,

    movies_list:        ScrollView,
    movies_description: MoviesDescription,
    sort_popup:         ContextMenu,
    context_menu_pos:   Option<Position>,
    context_menu_model: Vec<usize>,
    context_menu:       ContextMenu,

    home_dir: PathBuf,
}

const MOVIE_WIDGET_HEIGHT: usize = 11;

impl MainScreen {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (
            Some(self.tab),
            Some(
                self.item
                    + if self.tab == 1 {
                        self.movies_description.selected_tab << 9
                    } else {
                        0
                    },
            ),
        )
    }

    pub fn new(home_dir: &Path, _config: Rc<RefCell<Config>>) -> Self {
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
            lists: FxIndexMap::from_iter(
                load_file!("lists", home_dir)
                    .unwrap_or(vec![])
                    .into_iter()
                    .map(|x: List| (x.id, x)),
            ),
            movies: helpers::default_rc(),
            watched: helpers::default_rc(),
            filtered_movies: vec![],

            movies_list: ScrollView::new(MOVIE_WIDGET_HEIGHT as u16),
            movies_description: MoviesDescription::default(),
            sort_popup: ContextMenu::new(vec![], 6, None, false).with_submenu(
                <usize>::from(Sort::Rating(Default::default())),
                RatingSource::iter()
                    .map(|x| x.as_ref().to_string())
                    .enumerate()
                    .collect_vec(),
                6,
                None,
            ),
            context_menu_pos: None,
            context_menu: ContextMenu::new(vec![], 5, None, false),
            context_menu_model: vec![],

            home_dir: home_dir.to_path_buf(),
        }
    }

    pub fn initialize(
        &mut self,
        movies: Rc<RefCell<FxIndexMap<u32, Movie>>>,
        watched: Rc<RefCell<FxIndexMap<u32, Entry>>>,
    ) -> bool {
        self.movies = movies;
        self.watched = watched;

        if self.lists.is_empty() || !self.lists.keys().any(|x| *x == ListID::Watchlist) {
            self.lists.insert_before(
                0,
                ListID::Watchlist,
                List {
                    id:       ListID::Watchlist,
                    name:     "Watchlist".into(),
                    items:    Default::default(),
                    readonly: false,
                },
            );
            self.save_lists();
        }

        let fetch_movies = {
            let movies_borrowed = self.movies.borrow();
            self.get_list_ids()
                .iter()
                .any(|x| !movies_borrowed.contains_key(x))
        };
        if fetch_movies {
            true
        } else {
            self.filter_sort_movies(false);

            false
        }
    }

    pub fn save_lists(&self) {
        let path = &self.home_dir.join("lists.json");
        match serde_json::to_string(&self.lists.values().collect_vec()) {
            Err(error) => {
                error!("Error while trying to serialize {}: {error}", "lists")
            }
            Ok(serialized) => {
                _ = fs::rename(path, self.home_dir.join("lists").with_extension("json.bak"));
                if let Err(error) = fs::write(path, serialized) {
                    error!("Error while trying to save {}: {error}", "lists")
                }
            }
        }
    }

    pub fn add_list(&mut self, list: List) {
        match self.lists.entry(list.id) {
            indexmap::map::Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().name = list.name;
                occupied_entry.get_mut().items = list.items;
                occupied_entry.get_mut().readonly = list.readonly;
            }
            indexmap::map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert_entry(list);
            }
        }

        self.save_lists();
    }

    pub fn update_list(
        &mut self,
        id: ListID,
        items: Vec<ListItem>,
        deleted_items: Option<Vec<u32>>,
        overwrite: bool,
    ) {
        self.lists.entry(id).and_modify(|x| {
            if overwrite {
                x.items = items.into_iter().map(|x| (x.id, x)).collect();
            } else {
                x.items.extend(
                    items
                        .into_iter()
                        .filter_map(|y| {
                            if !x.items.contains_key(&y.id) {
                                Some((y.id, y))
                            } else {
                                None
                            }
                        })
                        .collect_vec(),
                );
                if let Some(deleted_items) = deleted_items {
                    x.items.retain(|x, _| !deleted_items.contains(x));
                }
            }
        });

        self.save_lists();
    }

    pub fn open_list(&mut self, index: usize, key_event_handler: &mut KeyEventHandler) -> bool {
        if index > self.lists.len() + 1 {
            return false;
        }

        self.open_list_by_id(
            if index == 0 {
                ListID::All
            } else if index == 1 {
                ListID::Watched
            } else {
                self.lists[index - 2].id
            },
            key_event_handler,
        )
    }

    pub fn open_list_by_id(&mut self, id: ListID, key_event_handler: &mut KeyEventHandler) -> bool {
        if self.selected_list == id {
            return false;
        }

        if matches!(id, ListID::Watched | ListID::All) || self.lists.contains_key(&id) {
            self.selected_list = id;

            let fetch_movies = {
                let movies_borrowed = self.movies.borrow();
                self.get_list_ids()
                    .iter()
                    .any(|x| !movies_borrowed.contains_key(x))
            };
            if matches!(id, ListID::Watchlist) && matches!(self.sort, Sort::FirstWatched) {
                self.sort = Default::default();
            }
            if fetch_movies {
                key_event_handler.bind_immediate(|app, _| app.drawer.open_fetch_movies_popup());

                false
            } else {
                self.filter_sort_movies(false);

                true
            }
        } else {
            false
        }
    }

    pub fn open_list_and_select_movie(
        &mut self,
        key_event_handler: &mut KeyEventHandler,
        list_id: ListID,
        movie_id: u32,
    ) {
        if self.open_list_by_id(list_id, key_event_handler) {
            let pos = self.filtered_movies.iter().position(|x| x.id == movie_id);
            if let Some(index) = pos {
                self.movies_list
                    .goto_index(index, true, self.filtered_movies.len());
            }
        }
    }

    pub fn get_list_ids(&self) -> Vec<u32> {
        if matches!(&self.selected_list, ListID::Watched) {
            self.watched.borrow().keys().copied().collect()
        } else if matches!(&self.selected_list, ListID::All) {
            self.movies.borrow().keys().copied().collect()
        } else {
            self.lists
                .get(&self.selected_list)
                .map(|x| x.items.keys().copied().collect_vec())
                .unwrap()
        }
    }

    fn get_list_movies(&self) -> Vec<Movie> {
        let movies_borrowed = self.movies.borrow();
        self.get_list_ids()
            .iter()
            .map(|id| movies_borrowed[id].clone())
            .collect()
    }

    fn list_editable(&self) -> bool {
        matches!(self.selected_list, ListID::Watched | ListID::Watchlist)
            || !(matches!(self.selected_list, ListID::Collection(_) | ListID::All)
                || self.lists[&self.selected_list].readonly)
    }

    fn refetch_current_list(
        &mut self,
        watched: &FxIndexMap<u32, Entry>,
        tmdb_tokens: &TMDBTokens,
        simkl_tokens: &SimklTokens,
        punch_play_tokens: &PunchPlayTokens,
    ) {
        match self.selected_list {
            ListID::Watchlist => {
                let default_utc = DateTime::<Utc>::default();
                let items = [
                    simkl::movie::get_user_watchlist(
                        simkl_tokens.access_token(),
                        simkl_tokens.client_id(),
                        simkl_tokens.app_name(),
                        simkl_tokens.app_version(),
                    )
                    .unwrap_or_default()
                    .movies
                    .unwrap_or_default()
                    .into_iter()
                    .map(|x| ListItem {
                        id:       x.show_or_movie.ids.tmdb.unwrap(),
                        added_at: x.added_to_watchlist_at,
                    })
                    .collect_vec(),
                    punch_play::movie::get_user_watchlist(punch_play_tokens.access_token())
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|x| {
                            if x.item_type == "movie" {
                                Some(ListItem {
                                    id:       x.tmdb_id,
                                    added_at: x.added_at,
                                })
                            } else {
                                None
                            }
                        })
                        .collect_vec(),
                    tmdb::movie::get_user_watchlist(
                        tmdb_tokens.access_token(),
                        tmdb_tokens.account_id(),
                    )
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| ListItem {
                        id:       x.id,
                        added_at: default_utc + TimeDelta::seconds(i as i64),
                    })
                    .collect_vec(),
                ]
                .into_iter()
                .flatten();

                self.update_list(
                    ListID::Watchlist,
                    items
                        .sorted_by_key(|x| x.id)
                        .dedup_by(|a, b| a.id == b.id)
                        .filter(|x| !watched.contains_key(&x.id))
                        .collect_vec(),
                    None,
                    false,
                );
            }
            ListID::TMDB(id) => {
                if let Ok(list_details) =
                    tmdb::list::get_list_details(tmdb_tokens.access_token(), id)
                {
                    self.add_list(List::from_tmdb(list_details, false));
                }
            }
            ListID::PunchPlay(id) => {
                if let Ok(list_details) =
                    punch_play::list::get_list_details(punch_play_tokens.access_token(), id)
                {
                    self.add_list(List::from_punch_play(list_details, false));
                }
            }
            ListID::Collection(id) => {
                if let Ok(collection_details) =
                    tmdb::collection::get_collection_details(tmdb_tokens.access_token(), id)
                {
                    self.add_list(List::from_collection(collection_details));
                }
            }
            _ => (),
        }
    }

    pub fn current_movie(&self) -> Option<&Movie> {
        self.filtered_movies.get(self.movies_list.selected_index)
    }

    pub fn goto_index(&mut self, index: isize) {
        let index = if index.is_negative() {
            (self.filtered_movies.len() as isize + index) as usize
        } else {
            (index as usize).min(self.filtered_movies.len() - 1)
        };

        self.movies_list
            .goto_index(index, false, self.filtered_movies.len());
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
                    movie.title.clone()// + " " + &movie.release_date.year().to_string())
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

            self.movies_list
                .goto_index(index, true, self.filtered_movies.len());
        }
    }

    fn filter_movies(&mut self) {
        let mut movies = self.get_list_movies();
        for criterion in &self.filter_criteria {
            match criterion {
                FilterCriterion::Title(name, _) if !name.is_empty() => {
                    if name.is_empty() {
                        continue;
                    }
                    let mut conf: MatcherConfig = MatcherConfig::DEFAULT;
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
                                movie.title.clone()
                                    // + " "
                                    // + &movie.release_date.year().to_string())
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
                FilterCriterion::Title(_, _) => (),
                FilterCriterion::Actors(actors, contains_all, inverted) => {
                    movies.retain(|x| if *contains_all {
                        actors.iter().all(|y| x.credits.cast.iter().map(|x| x.id).contains(y))
                        } else {
                            actors.iter().any(|y| x.credits.cast.iter().map(|x| x.id).contains(y))
                            } ^ *inverted);
                }
                FilterCriterion::Director(director, inverted) => {
                    movies.retain(|x| {
                        x.credits
                            .crew
                            .iter()
                            .filter_map(|x| (x.job_or_character == "Director").then_some(x.id))
                            .contains(director)
                            ^ *inverted
                    });
                }
                FilterCriterion::Genres(genres, contains_all, inverted) => {
                    movies.retain(|x| if *contains_all {genres.iter().all(|y| x.genres.contains(y))} else {genres.iter().any(|y| x.genres.contains(y))} ^ *inverted);
                }
                FilterCriterion::Released(lower_bound, upper_bound, inverted) => {
                    movies.retain(|x| {
                        (x.release_date.year() as u32).is_between(lower_bound, upper_bound)
                            ^ *inverted
                    });
                }
                FilterCriterion::FirstWatched(lower_bound, upper_bound, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies.retain(|x| {
                        watched_borrowed
                            .get(&x.id)
                            .map(|y| {
                                (y.get_first_play().year() as u32)
                                    .is_between(lower_bound, upper_bound)
                                    ^ *inverted
                            })
                            .unwrap_or(false)
                    });
                }
                FilterCriterion::LastWatched(lower_bound, upper_bound, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies.retain(|x| {
                        watched_borrowed
                            .get(&x.id)
                            .map(|y| {
                                (y.get_latest_play().year() as u32)
                                    .is_between(lower_bound, upper_bound)
                                    ^ *inverted
                            })
                            .unwrap_or(false)
                    });
                }
                FilterCriterion::Rating(rating, ordering, inverted) => {
                    movies.retain(|x| {
                        (x.get_first_external_rating().partial_cmp(rating).unwrap() == *ordering)
                            ^ *inverted
                    });
                }
                FilterCriterion::UserRating(rating, ordering, inverted) => {
                    let watched_borrowed = self.watched.borrow();
                    movies.retain(|x| {
                        watched_borrowed
                            .get(&x.id)
                            .map(|y| {
                                (y.get_user_rating().partial_cmp(rating).unwrap() == *ordering)
                                    ^ *inverted
                            })
                            .unwrap_or(false)
                    });
                }
                FilterCriterion::Language(language, inverted) => {
                    movies.retain(|x| (*language == x.language) ^ *inverted);
                }
                FilterCriterion::Country(country, inverted) => {
                    movies.retain(|x| (x.origin_country == *country) ^ *inverted);
                }
                FilterCriterion::Certification(certifications, inverted) => {
                    movies.retain(|x| certifications.contains(&x.certification) ^ *inverted);
                }
            }
        }

        self.filtered_movies = movies;
    }

    fn sort_movies(&mut self) {
        match self.sort {
            Sort::UserRating => {
                self.filtered_movies.sort_by(|x, y| {
                    self.watched
                        .borrow()
                        .get(&x.id)
                        .map(|x| x.get_user_rating())
                        .unwrap_or(f64::NAN)
                        .total_cmp(
                            &self
                                .watched
                                .borrow()
                                .get(&y.id)
                                .map(|y| y.get_user_rating())
                                .unwrap_or(f64::NAN),
                        )
                });
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Rating(rating_source) => {
                self.filtered_movies
                    .sort_by(|a, b| a.cmp_rating(b, rating_source));
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
                self.filtered_movies.sort_by_key(|x| x.release_date);
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::FirstWatched => {
                self.filtered_movies.sort_by_key(|x| {
                    self.watched
                        .borrow()
                        .get(&x.id)
                        .map(|x| x.get_first_play())
                        .unwrap_or_default()
                });
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::MostRecent => {
                if let Some(list) = self.lists.get(&self.selected_list) {
                    self.filtered_movies.sort_by_key(|x| {
                        list.items
                            .get(&x.id)
                            .map(|x| x.added_at)
                            .unwrap_or_default()
                    });
                } else {
                    self.filtered_movies.sort_by_key(|x| {
                        self.watched
                            .borrow()
                            .get(&x.id)
                            .map(|x| x.get_latest_play())
                            .unwrap_or_default()
                    });
                }
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Relevance => (),
        }
    }

    pub fn filter_sort_movies(&mut self, keep_selected: bool) {
        let selected_movie_id = self.current_movie().map(|x| x.id).unwrap_or(u32::MAX);

        self.filter_movies();

        match self.sort {
            Sort::Relevance => {}
            _ => {
                self.sort_movies();
            }
        }

        if keep_selected {
            let pos = self
                .filtered_movies
                .iter()
                .position(|x| x.id == selected_movie_id);
            if let Some(index) = pos {
                self.movies_list
                    .goto_index(index, true, self.filtered_movies.len());
            } else {
                self.movies_list.reset();
            }
        } else {
            self.movies_list.reset();
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
                        main_screen.filter_sort_movies(true);
                    }
                }
            });
        }

        if !self.filtered_movies.is_empty() {
            for tab in 0..=1 {
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'r',
                    "Refetch details".into(),
                    |app, _| {
                        app.drawer.open_refetch_details_popup(
                            app.tmdb_tokens.clone(),
                            app.punch_play_tokens.clone(),
                            app.trakt_tokens.clone(),
                            app.omdb_tokens.clone(),
                        );
                    },
                );
                if matches!(self.current_movie(), Some(movie) if movie.released) {
                    key_event_handler.bind_key(
                        (Some(tab), None),
                        'A',
                        "Add play".into(),
                        |app, _| {
                            app.drawer.open_add_play_popup();
                        },
                    );
                    key_event_handler.bind_key(
                        (Some(tab), None),
                        'E',
                        "Manage plays".into(),
                        |app, _| {
                            app.drawer.open_manage_plays_popup(&app.watched.borrow());
                        },
                    );

                    if self
                        .watched
                        .borrow()
                        .contains_key(&self.current_movie().unwrap().id)
                    {
                        key_event_handler.bind_key(
                            (Some(tab), None),
                            'e',
                            "Edit movie".into(),
                            |app, _| {
                                app.drawer.open_edit_movie_popup(&app.watched.borrow());
                            },
                        );
                    }
                }
                if self.list_editable() {
                    key_event_handler.bind_key(
                        (Some(tab), None),
                        'd',
                        "Delete movie".into(),
                        |app, _| {
                            app.drawer.open_delete_movie_popup(&app.movies.borrow());
                        },
                    );
                }
            }
        }

        for i in 0..=(9.min(self.lists.len() + 2)) {
            key_event_handler.bind_key((Some(0), None), i, "".into(), move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.open_list(i, &mut app.key_event_handler);
                }
            });
        }

        if !matches!(self.selected_list, ListID::Local(_) | ListID::Watched) {
            key_event_handler.bind_key((Some(0), None), 'R', "Update list".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match main_screen.selected_list {
                        ListID::Watched => {
                            // app.refetch_watched();
                        }
                        _ => {
                            main_screen.refetch_current_list(
                                &app.watched.borrow_mut(),
                                &app.tmdb_tokens,
                                &app.simkl_tokens,
                                &app.punch_play_tokens,
                            );
                        }
                    }

                    app.drawer.open_fetch_movies_popup();
                }
            });
        }

        if self.list_editable() {
            let take_rating = self.selected_list == ListID::Watched;
            key_event_handler.bind_key((Some(0), None), 'a', "Add movie".into(), move |app, _| {
                app.drawer.open_add_movie_popup(
                    app.tmdb_tokens.clone(),
                    app.punch_play_tokens.clone(),
                    app.trakt_tokens.clone(),
                    app.omdb_tokens.clone(),
                    take_rating,
                );
            });
        }
        key_event_handler.bind_key((Some(0), None), 'F', "Advanced Filter".into(), |app, _| {
            app.drawer.open_advanced_filter_popup();
        });
        key_event_handler.bind_key((Some(0), None), 'l', "Manage lists".into(), |app, _| {
            app.drawer.open_manage_lists_popup();
        });
        key_event_handler.bind_key((Some(0), None), ',', "Sort by".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 1;
                // main_screen.sort_popup.reset_state();
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
                    main_screen.filter_sort_movies(true);
                }
            }
        });
        key_event_handler.bind_key((Some(0), None), 'f', "Filter".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 0;

                if let Sort::Relevance = main_screen.sort {
                    main_screen.sort = Sort::default();
                }
                _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title("".into(), true));

                if !main_screen.search_input.is_empty() {
                    main_screen.search_input = TextArea::from([""]);
                    main_screen.filter_sort_movies(true);
                }
            }
        });

        let frame_area = frame.area();

        // let num_movies = ((frame_area.height - 5) as f32 / 9.0).floor() as usize;
        // let footer_height = (((frame_area.height - 5) % 9) % num_movies as u16) + 2;
        let [header, vert, _] = vertical![==3, >=1, ==2].areas(frame_area);

        let [description, list] = horizontal![==vert.width * 3 / 8 - 1, >=0].areas(vert);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), header);

        self.drawing_images = false;
        self.render_movies_list(frame, image_renderer, key_event_handler, list);
        self.render_movie_description(frame, image_renderer, key_event_handler, description);
        self.render_header(frame, header, key_event_handler);

        if let Some(pos) = self.context_menu_pos {
            self.context_menu_model = (0..CONTEXT_MENU_MODEL.len())
                .filter(|x| match x {
                    0 | 2 => {
                        matches!(self.current_movie(), Some(movie) if movie.released)
                    }
                    1 =>
                        matches!(self.current_movie(), Some(movie) if movie.released)
                            && self
                                .watched
                                .borrow()
                                .contains_key(&self.current_movie().unwrap().id),
                    4 => self.list_editable(),
                    _ => true,
                })
                .collect_vec();
            if self
                .context_menu_model
                .iter()
                .ne(self.context_menu.model.keys())
            {
                self.context_menu.change_model(
                    self.context_menu_model
                        .iter()
                        .map(|&x| (x, CONTEXT_MENU_MODEL[x].to_string()))
                        .collect(),
                    None,
                );
            }

            if self.context_menu_model.is_empty() {
            } else {
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
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                    }
                });

                key_event_handler.bind_key((None, None), 'q', "Cancel".into(), |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                    }
                });

                key_event_handler.bind_vertical(
                    (None, None),
                    "Navigate".into(),
                    move |app, data| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            if let key_event_handler::Data::Direction(dir, _) = data {
                                main_screen.context_menu.scroll(dir);
                            }
                        }
                    },
                );

                for &i in &self.context_menu_model {
                    if i == 0 {
                        key_event_handler.bind_key(
                            (None, None),
                            'A',
                            "Add play".into(),
                            |app, _| {
                                app.drawer.open_add_play_popup();

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    } else if i == 1 {
                        key_event_handler.bind_key(
                            (None, None),
                            'e',
                            "Edit movie".into(),
                            |app, _| {
                                app.drawer.open_edit_movie_popup(&app.watched.borrow());

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    } else if i == 2 {
                        key_event_handler.bind_key(
                            (None, None),
                            'E',
                            "Manage plays".into(),
                            |app, _| {
                                app.drawer.open_add_play_popup();

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    } else if i == 3 {
                        key_event_handler.bind_key(
                            (None, None),
                            'R',
                            "Refetch details".into(),
                            |app, _| {
                                app.drawer.open_refetch_details_popup(
                                    app.tmdb_tokens.clone(),
                                    app.punch_play_tokens.clone(),
                                    app.trakt_tokens.clone(),
                                    app.omdb_tokens.clone(),
                                );

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    } else if i == 4 {
                        key_event_handler.bind_key(
                            (None, None),
                            'd',
                            "Delete movie".into(),
                            |app, _| {
                                app.drawer.open_delete_movie_popup(&app.movies.borrow());

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    }
                }

                let width = self.context_menu.width;
                let height = self
                    .context_menu
                    .model
                    .len()
                    .min(self.context_menu.num_visible_items) as u16;

                let x = if pos.x + width > frame.area().width {
                    frame.area().width - width
                } else {
                    pos.x
                };
                let y = if pos.y + height > frame.area().height {
                    frame.area().height - height
                } else {
                    pos.y
                };

                key_event_handler.bind_enter((None, None), "Choose".into(), |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                        let i = main_screen.context_menu_model
                            [*main_screen.context_menu.choose().first().unwrap()];
                        if i == 0 {
                            app.drawer.open_add_play_popup();
                        } else if i == 1 {
                            app.drawer.open_edit_movie_popup(&app.watched.borrow());
                        } else if i == 2 {
                            app.drawer.open_manage_plays_popup(&app.watched.borrow());
                        } else if i == 3 {
                            app.drawer.open_refetch_details_popup(
                                app.tmdb_tokens.clone(),
                                app.punch_play_tokens.clone(),
                                app.trakt_tokens.clone(),
                                app.omdb_tokens.clone(),
                            );
                        } else if i == 4 {
                            app.drawer.open_delete_movie_popup(&app.movies.borrow());
                        }
                    }
                });

                let (mut mouse_area, len) = self
                    .context_menu
                    .render(Position { x, y }, frame, key_event_handler)
                    .into_iter()
                    .nth(0)
                    .unwrap()
                    .1;

                for i in 0..len {
                    let option_index = self.context_menu_model[i];
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        move |app, _| {
                            if option_index == 0 {
                                app.drawer.open_add_play_popup();
                            } else if option_index == 1 {
                                app.drawer.open_edit_movie_popup(&app.watched.borrow());
                            } else if option_index == 2 {
                                app.drawer.open_manage_plays_popup(&app.watched.borrow());
                            } else if option_index == 3 {
                                app.drawer.open_refetch_details_popup(
                                    app.tmdb_tokens.clone(),
                                    app.punch_play_tokens.clone(),
                                    app.trakt_tokens.clone(),
                                    app.omdb_tokens.clone(),
                                );
                            } else if option_index == 4 {
                                app.drawer.open_delete_movie_popup(&app.movies.borrow());
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
                    main_screen.filter_sort_movies(true);
                }
            }
        });
        key_event_handler.bind_esc((Some(2), None), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_tab((Some(2), None), "Change focus".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if main_screen.item == 0 {
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
                } else {
                    // main_screen.sort = Sort::Relevance;
                    main_screen.search_input = TextArea::from([""]);
                    _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                    main_screen
                        .filter_criteria
                        .push(FilterCriterion::Title("".into(), true));
                    main_screen.filter_sort_movies(true);
                }

                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.item += 1;
                        if main_screen.item > 2 {
                            main_screen.item = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.item = main_screen.item.checked_sub(1).unwrap_or(2);
                    }
                    _ => (),
                }
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
                main_screen.item = 1;
                // main_screen.sort_popup.reset_state();
            }
        });
        key_event_handler.bind_key(
            (Some(2), Some(2)),
            ' ',
            "Toggle sort order".into(),
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.sort_ascending = !main_screen.sort_ascending;
                    main_screen.filter_sort_movies(true);
                }
            },
        );
        key_event_handler.bind_key((Some(2), Some(1)), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_key((Some(2), Some(2)), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        if matches!(self.sort, Sort::Rating(_)) {
            key_event_handler.bind_horizontal(
                (Some(2), Some(1)),
                if self.sort_popup.opened_submenu.is_none() {
                    "Open submenu/Navigate"
                } else {
                    "Close submenu"
                }
                .into(),
                |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Direction(false, _) => {
                                main_screen.sort_popup.open_submenu(true);
                            }
                            key_event_handler::Data::Direction(true, _) => {
                                if main_screen.sort_popup.opened_submenu.is_some() {
                                    main_screen.sort_popup.close_submenu();
                                } else {
                                    main_screen.item += 1;
                                }
                            }
                            _ => (),
                        }
                        main_screen.filter_sort_movies(true);
                    }
                },
            );
        } else {
            key_event_handler.bind_horizontal(
                (Some(2), Some(1)),
                "Navigate".into(),
                |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        if let key_event_handler::Data::Direction(true, _) = data {
                            main_screen.item += 1;
                        }
                    }
                },
            );
        }
        key_event_handler.bind_horizontal((Some(2), Some(2)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if let key_event_handler::Data::Direction(false, _) = data {
                    main_screen.item -= 1;
                }
            }
        });

        key_event_handler.bind_vertical(
            (Some(2), Some(1)),
            if self.sort_popup.opened_submenu.is_none() {
                "Change sort"
            } else {
                "Change rating source"
            }
            .into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if let key_event_handler::Data::Direction(direction, _) = data {
                        main_screen.sort_popup.scroll(direction);

                        main_screen.sort = if let Some(submenu_id) =
                            main_screen.sort_popup.opened_submenu.as_ref()
                        {
                            Sort::Rating(
                                RatingSource::from_repr(
                                    main_screen.sort_popup.submenus[submenu_id].id_from_index(
                                        main_screen.sort_popup.submenus[submenu_id].selected_index,
                                    ),
                                )
                                .unwrap(),
                            )
                        } else {
                            Sort::from_repr(
                                main_screen
                                    .sort_popup
                                    .id_from_index(main_screen.sort_popup.selected_index),
                            )
                            .unwrap()
                        };
                    }
                    main_screen.filter_sort_movies(true);
                }
            },
        );
        key_event_handler.bind_vertical(
            (Some(2), Some(2)),
            "Change sort order".into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(false, _) => {
                            if !main_screen.sort_ascending {
                                main_screen.sort_ascending = true;
                                main_screen.filter_sort_movies(true);
                            }
                        }
                        key_event_handler::Data::Direction(true, _)
                            if main_screen.sort_ascending =>
                        {
                            main_screen.sort_ascending = false;
                            main_screen.filter_sort_movies(true);
                        }
                        _ => (),
                    }
                }
            },
        );

        key_event_handler.bind_input_field((Some(2), Some(0)), "".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if let key_event_handler::Data::Key(key_event) = data {
                    main_screen.search_input.input(key_event);

                    let FilterCriterion::Title(_, filter) = pop_criterion!(
                        main_screen.filter_criteria,
                        FilterCriterion::Title(_, _),
                        FilterCriterion::Title(String::new(), false)
                    ) else {
                        unreachable!()
                    };
                    main_screen.sort = if filter && !main_screen.search_input.is_empty() {
                        Sort::Relevance
                    } else {
                        Sort::default()
                    };
                    main_screen.filter_criteria.push(FilterCriterion::Title(
                        main_screen.search_input.lines()[0].clone(),
                        filter,
                    ));
                    if filter {
                        main_screen.filter_sort_movies(false);
                    } else {
                        main_screen.find_and_goto_movie();
                    }
                }
            }
        });

        let sort_max_width = Sort::iter().map(|x| x.as_ref().len()).max().unwrap() + 4;

        let [_, input_area, _, sort_area, _, direction_area, _] =
            horizontal![>=1, <=25, ==1, ==sort_max_width as u16, ==1, ==3, ==1].areas(area);

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
            None,
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

                        main_screen
                            .filter_criteria
                            .push(FilterCriterion::Title("".into(), true));
                        main_screen.filter_sort_movies(true);
                    }
                }
            },
        );

        widgets::dropdown(
            tab_selected,
            self.item == 1,
            frame,
            sort_area,
            helpers::ellipsize_string(self.sort.as_ref(), sort_area.width as usize - 4),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            sort_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 1;
                    // main_screen.sort_popup.reset_state();
                }
            },
        );

        if tab_selected && self.item == 1 {
            let items = Sort::iter()
                .filter(|x| match x {
                    Sort::MostRecent => true,
                    Sort::ReleaseDate => true,
                    Sort::Rating(_) => true,
                    Sort::Name => true,
                    Sort::FirstWatched => {
                        let watched_borrowed = self.watched.borrow();
                        self.get_list_ids()
                            .iter()
                            .any(|x| watched_borrowed.contains_key(x))
                    }
                    Sort::UserRating => {
                        let watched_borrowed = self.watched.borrow();
                        self.get_list_ids()
                            .iter()
                            .any(|x| watched_borrowed.contains_key(x))
                    }
                    Sort::Relevance => !self.search_input.is_empty(),
                })
                .collect_vec();

            self.sort_popup.model = items
                .iter()
                .map(|&x| {
                    (
                        <usize>::from(x),
                        helpers::ellipsize_string(x.as_ref(), sort_area.width as usize - 2),
                    )
                })
                .collect();

            self.sort_popup.selected_index = self
                .sort_popup
                .index_from_id(<usize>::from(self.sort))
                .unwrap_or_default();
            if let Sort::Rating(source) = self.sort {
                self.sort_popup
                    .submenus
                    .get_mut(&<usize>::from(Sort::Rating(Default::default())))
                    .unwrap()
                    .selected_index = source as usize;
            };

            let areas = self
                .sort_popup
                .render_dropdown(sort_area, frame, key_event_handler);

            for (k, (mut mouse_area, len)) in areas {
                if k.is_empty() {
                    let scroll_pos = self.sort_popup.scroll_pos;
                    for i in 0..len {
                        let index = i + scroll_pos;
                        if self.sort_popup.selected_index != index {
                            key_event_handler.bind_mouse_button_down(
                                ratatui::crossterm::event::MouseButton::Left,
                                mouse_area,
                                move |app, _| {
                                    if let Some(Screens::MainScreen(main_screen)) =
                                        app.drawer.current_screen.as_mut()
                                    {
                                        main_screen.sort = Sort::from_repr(index).unwrap();
                                        main_screen.sort_popup.selected_index = index;

                                        if !matches!(main_screen.sort, Sort::Rating(_)) {
                                            main_screen.tab = 0;
                                            main_screen.item = 0;
                                        } else {
                                            main_screen.sort_popup.open_submenu(true);
                                        }
                                        main_screen.filter_sort_movies(true);
                                    }
                                },
                            );
                        }
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                } else {
                    let scroll_pos = self
                        .sort_popup
                        .submenus
                        .get_mut(&<usize>::from(Sort::Rating(Default::default())))
                        .unwrap()
                        .scroll_pos;
                    for i in 0..len {
                        let index = i + scroll_pos;
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.tab = 0;
                                    main_screen.item = 0;
                                    main_screen.sort =
                                        Sort::Rating(RatingSource::from_repr(index).unwrap());
                                    // main_screen.sort_popup.reset_state();
                                    main_screen.filter_sort_movies(true);
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                }
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
                    main_screen.filter_sort_movies(true);
                }
            },
        );

        sort_area
    }

    fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
    ) {
        let num_items = self.filtered_movies.len();

        if !self.filtered_movies.is_empty() {
            let num_visible_items = self.movies_list.num_visible_items;

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
                    if let key_event_handler::Data::Direction(direction, modifiers) = data {
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            if direction {
                                main_screen.goto_index(
                                    (main_screen.movies_list.selected_index
                                        + num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.goto_index(
                                    main_screen
                                        .movies_list
                                        .selected_index
                                        .saturating_sub(num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            }
                        } else {
                            main_screen.movies_list.scroll(direction, num_items);
                        }
                    }
                }
            });
        }

        if self.movies_list.selected_index >= num_items {
            self.movies_list.selected_index = num_items.saturating_sub(1);
            self.movies_list.scroll_pos = self
                .movies_list
                .selected_index
                .saturating_sub(self.movies_list.num_visible_items.saturating_sub(1));
        }

        let [movies_area, scrollbar_area] = horizontal![>=0, ==1].areas(area);

        let mut areas = vec![];
        self.movies_list.render(
            num_items,
            movies_area,
            scrollbar_area,
            frame,
            key_event_handler,
            |_scroll_view, area, index, _selected, _alternate, _frame, key_event_handler| {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;

                            main_screen.movies_list.goto_index(index, false, num_items);
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

                            main_screen.movies_list.goto_index(index, false, num_items);

                            if let key_event_handler::Data::Mouse(mouse_event) = data {
                                main_screen.context_menu_pos =
                                    Some(Position::new(mouse_event.column, mouse_event.row));
                                main_screen.context_menu.reset_state();
                            }
                        }
                    },
                );

                areas.push((index, area));
            },
        );

        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            scrollbar_area.resize(Size::new(1, 1)),
            move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if main_screen.movies_list.alignment_bottom
                        && main_screen.movies_list.partially_visible
                    {
                        main_screen.movies_list.alignment_bottom = false;
                    } else if main_screen.movies_list.scroll_pos > 0 {
                        main_screen.movies_list.scroll_pos -= 1;
                    }
                }
            },
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            scrollbar_area
                .resize(Size::new(1, 1))
                .offset(Offset::new(0, scrollbar_area.height as i32 - 1)),
            move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if !main_screen.movies_list.alignment_bottom
                        && main_screen.movies_list.partially_visible
                    {
                        main_screen.movies_list.alignment_bottom = true;
                    } else if main_screen.movies_list.scroll_pos
                        < num_items.saturating_sub(main_screen.movies_list.num_visible_items)
                    {
                        main_screen.movies_list.scroll_pos += 1;
                    }
                }
            },
        );

        for (index, area) in areas {
            self.draw_movie_widget(index, frame, image_renderer, area);
        }
    }

    fn draw_movie_widget(
        &mut self,
        movie_index: usize,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        area: Rect,
    ) {
        let is_partially_visible = MOVIE_WIDGET_HEIGHT > area.height as usize;
        let selected = self.movies_list.selected_index == movie_index;
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

        let vert_lay = helpers::add_padding(
            area,
            if is_partially_visible {
                Padding::new(
                    2,
                    2,
                    if self.movies_list.alignment_bottom { 0 } else { 1 },
                    if self.movies_list.alignment_bottom { 1 } else { 0 },
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

        let rating = self
            .watched
            .borrow()
            .get(&movie.id)
            .map(|x| x.get_user_rating());
        let rating_color = rating.as_ref().map(|rating| {
            if *rating >= 9.0 {
                tailwind::SKY.c400
            } else if *rating >= 8.0 {
                tailwind::GREEN.c500
            } else if *rating >= 7.5 {
                tailwind::LIME.c400
            } else if *rating >= 7.0 {
                material::AMBER.c400
            } else if *rating >= 6.0 {
                material::DEEP_ORANGE.c300
            } else {
                material::RED.c400
            }
        });

        let mut description_lines: Vec<Line<'_>> = vec![];

        const TITLE_LINES: usize = 2;
        let mut title_lines = helpers::wrap_text(&movie.title, description_area.width as usize - 4);
        for _ in 0..(TITLE_LINES.saturating_sub(title_lines.len())) {
            description_lines.push("".into());
        }
        title_lines.reverse();
        for _ in 0..(TITLE_LINES.min(title_lines.len()) - 1) {
            description_lines.push(title_lines.pop().unwrap().bold().into());
        }
        description_lines.push(line!(
            helpers::ellipsize_string(
                &title_lines.pop().unwrap(),
                description_area.width as usize - 5,
            )
            .bold(),
            " ",
            movie.release_date.year().to_string().italic(),
            if !movie.released { span!(" - ") } else { "".into() },
            if !movie.released {
                span!("Not released").italic().fg(tailwind::RED.c300)
            } else {
                "".into()
            }
        ));

        if let (Some(rating), Some(rating_color)) = &(rating, rating_color) {
            description_lines.push(
                format!("{:.1}", rating)
                    .set_style(*rating_color)
                    .bold()
                    .into(),
            );
        } else {
            description_lines.push("Not watched".fg(tailwind::RED.c300).italic().into());
        }

        const TAGLINE_LINES: usize = 2;
        let mut tagline_lines = helpers::wrap_text(&movie.tagline, description_area.width as usize);
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
                if self.movies_list.alignment_bottom {
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
        // if let Some(rating_color) = &rating_color {
        if tab_selected && selected {
            frame.render_widget(
                text![span!("▐"); highlight_area.height as usize]
                    .fg(*rating_color.as_ref().unwrap_or(&tailwind::RED.c300)),
                highlight_area,
            );
        }
        // }

        self.drawing_images |= !image_renderer.draw_image(
            // ImageID::Person(self.movies.borrow()[&self.filtered_movies[movie_index].id].credits.cast[0].id),
            // ImageID::Person(self.movies.borrow()[&self.filtered_movies[movie_index].id].credits.crew.iter().find(|x| x.job_or_character == "Director").unwrap().id),
            // if let Some(collection_id) = self.movies.borrow()[&self.filtered_movies[movie_index].id].tmdb_collection {
            //     ImageID::Collection(collection_id, false)
            // } else {
            //     ImageID::Movie(self.filtered_movies[movie_index].id, false)
            // },
            ImageID::Movie(self.filtered_movies[movie_index].id, false),
            poster_area,
            if is_partially_visible {
                Some(SignedPosition {
                    x: 0,
                    y: if self.movies_list.alignment_bottom {
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
            self.current_movie().cloned()
        };

        let inner = helpers::add_padding(area, Padding::proportional(1));
        let backdrop_height = (inner.width as f32 * 9.0 / 16.0).ceil() as u16 >> 1;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==8, >=1].areas(inner);

        frame.render_widget(Block::new().bg(tailwind::GRAY.c950), backdrop_area);
        if let Some(movie) = movie {
            let [title_area, ratings_area, _, tabs_area] =
                vertical![==3, ==2, ==1, ==2].areas(title_area);

            let mut name = movie.title.clone();
            name = helpers::ellipsize_string(&name, title_area.width as usize);

            let rating = self
                .watched
                .borrow()
                .get(&movie.id)
                .map(|x| x.get_user_rating());
            let user_rating_widget_bg = rating
                .as_ref()
                .map(|rating| {
                    if *rating >= 9.0 {
                        tailwind::SKY.c400
                    } else if *rating >= 8.0 {
                        tailwind::GREEN.c500
                    } else if *rating >= 7.5 {
                        tailwind::LIME.c400
                    } else if *rating >= 7.0 {
                        material::AMBER.c400
                    } else if *rating >= 6.0 {
                        tailwind::ORANGE.c500
                    } else {
                        material::RED.c400
                    }
                })
                .unwrap_or(tailwind::BLACK);
            let user_rating_widget_fg = rating
                .as_ref()
                .map(|rating| {
                    if *rating >= 7.0 {
                        tailwind::STONE.c950
                    } else {
                        tailwind::STONE.c200
                    }
                })
                .unwrap_or(tailwind::GRAY.c300);

            frame.render_widget(
                line![
                    span!("     "),
                    name.clone().bold(),
                    span!(" "),
                    movie.release_date.year().to_string().italic()
                ]
                .centered(),
                title_area.resize(Size::new(title_area.width, 1)),
            );
            frame.render_widget(
                line!(
                    span!("").fg(user_rating_widget_bg),
                    if let Some(rating) = rating {
                        format!("{rating:.1}")
                    } else {
                        "Not watched".into()
                    }
                    .bold()
                    .fg(user_rating_widget_fg)
                    .bg(user_rating_widget_bg),
                    span!("").fg(user_rating_widget_bg)
                )
                .centered(),
                helpers::add_padding(title_area, Padding::top(1))
                    .resize(Size::new(title_area.width, 1)), // .centered(constraint!(==5), constraint!(==1)),
            );
            if movie.released {
                self.draw_ratings(&movie, frame, ratings_area);
            } else {
                if movie.release_date > Default::default() {
                    frame.render_widget(
                        text![
                            line!(
                                span!("").fg(user_rating_widget_bg),
                                "Not released"
                                    .bold()
                                    .fg(user_rating_widget_fg)
                                    .bg(user_rating_widget_bg),
                                span!("").fg(user_rating_widget_bg)
                            ),
                            line!(
                                "Releases: ".bold().fg(tailwind::SKY.c400),
                                span!(movie.release_date.format("%A, %-d %B, %C%y"))
                                    .italic()
                                    .fg(tailwind::INDIGO.c200),
                            )
                        ]
                        .centered(),
                        ratings_area,
                    );
                } else {
                    frame.render_widget(
                        line!(
                            span!("").fg(user_rating_widget_bg),
                            "Not released"
                                .bold()
                                .fg(user_rating_widget_fg)
                                .bg(user_rating_widget_bg),
                            span!("").fg(user_rating_widget_bg)
                        )
                        .centered(),
                        helpers::add_padding(ratings_area, Padding::top(1)),
                    );
                }
            }

            self.movies_description.available_tabs = DESCRIPTION_TABS
                .iter()
                .enumerate()
                .filter_map(|(i, x)| {
                    if *x == DESCRIPTION_TABS[0] {
                        Some(i)
                    } else if *x == DESCRIPTION_TABS[1] {
                        self.watched.borrow().contains_key(&movie.id).then_some(i)
                    } else {
                        None
                    }
                })
                .collect_vec();
            let num_available_tabs = self.movies_description.available_tabs.len();
            if self.movies_description.selected_tab >= num_available_tabs {
                self.movies_description.selected_tab = 0;
            }

            if num_available_tabs > 1 {
                key_event_handler.bind_horizontal(
                    (Some(1), None),
                    "Change tab".into(),
                    move |app, data| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Direction(true, _) => {
                                    main_screen.movies_description.selected_tab =
                                        (main_screen.movies_description.selected_tab + 1)
                                            .min(num_available_tabs - 1);
                                }
                                key_event_handler::Data::Direction(false, _) => {
                                    main_screen.movies_description.selected_tab = main_screen
                                        .movies_description
                                        .selected_tab
                                        .saturating_sub(1);
                                }
                                _ => (),
                            }
                        }
                    },
                );
            }

            const BGS: [Color; DESCRIPTION_TABS.len()] =
                [material::GREEN.c600, material::LIGHT_BLUE.c600];
            const FGS: [Color; DESCRIPTION_TABS.len()] =
                [material::BLUE.c100, material::YELLOW.c100];
            const _BGS: [Color; DESCRIPTION_TABS.len()] =
                [material::TEAL.c800, material::INDIGO.c600];
            const _FGS: [Color; DESCRIPTION_TABS.len()] =
                [material::BLUE_GRAY.c200, material::BLUE_GRAY.c200];
            let mut tabs = self
                .movies_description
                .available_tabs
                .iter()
                .enumerate()
                .flat_map(|(i, x)| {
                    [
                        format!(" {} ", DESCRIPTION_TABS[*x])
                            .fg(if description_selected { FGS[*x] } else { _FGS[*x] })
                            .bg(if description_selected { BGS[*x] } else { _BGS[*x] })
                            .add_modifier(if i == self.movies_description.selected_tab {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                        " ".into(),
                    ]
                })
                .dropping_back(1)
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
                            main_screen.movies_description.selected_tab = i / 2;
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
                        BGS[self.movies_description.selected_tab]
                    } else {
                        _BGS[self.movies_description.selected_tab]
                    }),
                ],
                tabs_area,
            );

            match self.movies_description.available_tabs[self.movies_description.selected_tab] {
                0 => {
                    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), description_area);

                    let mut overview_lines =
                        helpers::wrap_text(&movie.overview, description_area.width as usize)
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
                    self.movies_description.overview_scroll =
                        self.movies_description.overview_scroll.min(
                            overview_lines
                                .len()
                                .saturating_sub(description_area.height as usize),
                        );
                    let text = Text::from_iter(
                        overview_lines.split_off(self.movies_description.overview_scroll),
                    );

                    frame.render_widget(text, description_area);

                    key_event_handler.bind_vertical(
                        (Some(1), Some(self.movies_description.selected_tab << 9)),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Screens::MainScreen(main_screen)) =
                                app.drawer.current_screen.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Direction(false, _) => {
                                        main_screen.movies_description.overview_scroll =
                                            main_screen
                                                .movies_description
                                                .overview_scroll
                                                .saturating_sub(1);
                                    }
                                    key_event_handler::Data::Direction(true, _) => {
                                        main_screen.movies_description.overview_scroll += 1;
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                1 => self.draw_plays_tab(key_event_handler, &movie, frame, description_area),
                _ => (),
            };

            self.drawing_images |= !image_renderer.draw_image(
                ImageID::Movie(movie.id, true),
                backdrop_area,
                None,
                &mut self.throbber_state,
                frame,
            );
        }
    }

    fn draw_ratings(&self, movie: &Movie, frame: &mut Frame, area: Rect) {
        let imdb_colors = (
            Color::Rgb(245, 197, 24),
            Color::Black,
            Color::Rgb(250, 225, 120),
        );
        let letterboxd_colors = (
            Color::Rgb(0, 192, 48),
            Color::Black,
            Color::Rgb(115, 226, 122),
        );
        let trakt_colors = (
            Color::Rgb(165, 61, 185),
            Color::White,
            Color::Rgb(230, 140, 245),
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
        if movie.external_ratings.letterboxd.0 > 0.0 {
            ratings.push((
                "letterboxd",
                format!("{:.1}", movie.external_ratings.letterboxd.0),
            ));
        }
        if movie.external_ratings.trakt.0 > 0 {
            ratings.push(("trakt", movie.external_ratings.trakt.0.to_string()));
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
            .split(helpers::add_padding(area, Padding::top(1)));
        let mut widgets = vec![];
        let mut links = vec![];
        let mut labels = line!();
        for (name, rating) in ratings {
            let (bg, fg) = if name == "imdb" {
                labels.push_span(span!("IMDB").fg(imdb_colors.2));
                links.push("".to_string());

                (imdb_colors.0, imdb_colors.1)
            } else if name == "letterboxd" {
                labels.push_span(span!("Letterboxd").fg(letterboxd_colors.2));
                links.push("".to_string());

                (letterboxd_colors.0, letterboxd_colors.1)
            } else if name == "trakt" {
                labels.push_span(span!("Trakt").fg(trakt_colors.2));
                links.push("".to_string());

                (trakt_colors.0, trakt_colors.1)
            } else if name == "tmdb" {
                labels.push_span(span!("TMDB").fg(tmdb_colors.2));
                links.push(format!("https://www.themoviedb.org/movie/{}", movie.id));

                (tmdb_colors.0, tmdb_colors.1)
            } else if name == "popcorn" {
                labels.push_span(span!("Popcorn").fg(popcorn_colors.2));
                links.push("".to_string());

                (popcorn_colors.0, popcorn_colors.1)
            } else if name == "tomatoes" {
                labels.push_span(span!("Tomatoes").fg(tomatoes_colors.2));
                links.push("".to_string());

                (tomatoes_colors.0, tomatoes_colors.1)
            } else {
                continue;
            };

            widgets.push(line!["".fg(bg), rating.bg(bg).fg(fg).bold(), "".fg(bg)]);
        }

        for ((widget, label, link), &area) in
            izip!(widgets.into_iter(), labels, links).zip(widget_areas.iter())
        {
            frame.render_widget(label, area.offset(Offset::new(0, -1)));
            frame.render_widget(
                widgets::Hyperlink {
                    text: widget.into(),
                    url:  link,
                },
                area,
            );
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
        self.movies_description.plays_tab.num_visible_items =
            num_visible_plays + if render_partially_visible_play { 1 } else { 0 };

        if num_plays > num_visible_plays {
            key_event_handler.bind_vertical(
                (Some(1), Some(self.movies_description.selected_tab << 9)),
                "Scroll".into(),
                move |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Direction(false, _) => {
                                if main_screen.movies_description.plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description.plays_tab.alignment_bottom =
                                        false;
                                } else {
                                    main_screen.movies_description.plays_tab.scroll_pos =
                                        main_screen
                                            .movies_description
                                            .plays_tab
                                            .scroll_pos
                                            .saturating_sub(1);
                                }
                            }
                            key_event_handler::Data::Direction(true, _) => {
                                if !main_screen.movies_description.plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description.plays_tab.alignment_bottom =
                                        true;
                                } else {
                                    main_screen.movies_description.plays_tab.scroll_pos += 1;
                                }
                            }
                            _ => (),
                        }
                    }
                },
            );

            self.movies_description.plays_tab.scroll_pos =
                self.movies_description.plays_tab.scroll_pos.min(
                    num_plays.saturating_sub(self.movies_description.plays_tab.num_visible_items),
                );
            self.movies_description.plays_tab.alignment_bottom &= render_partially_visible_play;
        } else {
            self.movies_description.plays_tab.scroll_pos = 0;
            self.movies_description.plays_tab.alignment_bottom = false;
        }

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), area);

        let mut remaining_area = helpers::add_padding(area, Padding::left(1));
        for i in 0..self.movies_description.plays_tab.num_visible_items {
            let [area, remaining] = if render_partially_visible_play
                && i == (!self.movies_description.plays_tab.alignment_bottom as usize
                    * (self.movies_description.plays_tab.num_visible_items - 1))
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else {
                vertical![==3, >= 0]
            }
            .areas(remaining_area);

            let index = self.movies_description.plays_tab.scroll_pos + i;
            if index < num_plays {
                let partially_visible = area.height < 3;
                let play = &movie_plays[num_plays - 1 - index];

                let alternate = i & 1 == 1;
                let latest = index == 0;
                let last = index == num_plays - 1;

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
                    helpers::add_padding(area, Padding::left(2)),
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

                let local_date = play.date.with_timezone(&chrono::Local);
                for i in 0..area.height {
                    let index = if partially_visible {
                        if self.movies_description.plays_tab.alignment_bottom {
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
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
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
                                    if play.date == DateTime::<Utc>::default() {
                                        "Unknown".into()
                                    } else {
                                        local_date.format("%d/%m/%Y %H:%M").to_string()
                                    }
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
                                helpers::add_padding(areas[i as usize], Padding::left(4)),
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
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
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
