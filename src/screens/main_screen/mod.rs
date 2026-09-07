use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use itertools::Itertools;
use log::error;
use nucleo_matcher::{Config as MatcherConfig, Matcher, pattern::Atom};
use ratatui::{
    Frame,
    layout::{Offset, Position, Rect},
    macros::{horizontal, line, vertical},
    style::{
        Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::Block,
};
use ratatui_textarea::TextArea;
use strum::IntoEnumIterator;

use crate::{
    config::Config,
    helpers::{self, SuperOrd},
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    load_file,
    screens::Screens,
    tokens::{PunchPlayTokens, SimklTokens, TMDBTokens},
    types::{
        Entry, FilterCriterion, FxIndexMap, List, ListID, ListItem, Movie, Person, RatingSource,
        Sort, pop_criterion,
    },
    widgets::{self, ContextMenu, ListDirection, ScrolledList},
};

mod description;
mod list;

pub use description::*;
pub use list::*;

const CONTEXT_MENU_MODEL: [&str; 6] = [
    "Add play",
    "Edit",
    "Manage plays",
    "Change artworks",
    "Refetch details",
    "Delete",
];
pub struct MainScreen {
    tab:                 usize,
    item:                usize,
    pub sort:            Sort,
    pub drawing_images:  bool,
    pub sort_ascending:  bool,
    pub filter_criteria: Vec<FilterCriterion>,
    pub search_input:    TextArea<'static>,

    pub selected_list: ListID,
    pub lists:         FxIndexMap<ListID, List>,

    pub _config:         Rc<RefCell<Config>>,
    pub movies:          Rc<RefCell<FxIndexMap<u32, Movie>>>,
    pub watched:         Rc<RefCell<FxIndexMap<u32, Entry>>>,
    pub persons:         Rc<RefCell<FxIndexMap<u32, Person>>>,
    pub filtered_movies: Vec<Movie>,

    movies_list:        ScrolledList,
    movies_description: MoviesDescription,
    sort_popup:         ContextMenu,
    context_menu_pos:   Option<Position>,
    context_menu_model: Vec<usize>,
    context_menu:       ContextMenu,

    home_dir: PathBuf,
}

impl MainScreen {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    pub fn new(home_dir: &Path, _config: Rc<RefCell<Config>>) -> Self {
        Self {
            tab: 0,
            item: 0,
            sort: Sort::default(),
            drawing_images: false,
            sort_ascending: false,
            search_input: TextArea::default(),
            filter_criteria: vec![],
            _config,

            selected_list: Default::default(),
            lists: FxIndexMap::from_iter(
                load_file!("lists", home_dir)
                    .unwrap_or(vec![])
                    .into_iter()
                    .map(|x: List| (x.id, x)),
            ),
            movies: Default::default(),
            watched: Default::default(),
            persons: Default::default(),
            filtered_movies: vec![],

            movies_list: ScrolledList::new(
                ListDirection::Vertical(false),
                MOVIE_WIDGET_HEIGHT as u16,
            ),
            movies_description: Default::default(),
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
        persons: Rc<RefCell<FxIndexMap<u32, Person>>>,
    ) -> bool {
        self.movies = movies;
        self.watched = watched;
        self.persons = persons;

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
            nucleo_matcher::pattern::Normalization::Smart,
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
                        nucleo_matcher::pattern::Normalization::Smart,
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
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'm',
                    "Change artworks".into(),
                    |app, _| {
                        app.drawer.open_change_artwork_popup(&app.tmdb_tokens);
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

            if !self.context_menu_model.is_empty() {
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
                    if CONTEXT_MENU_MODEL[i] == "Add play" {
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
                    } else if CONTEXT_MENU_MODEL[i] == "Edit" {
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
                    } else if CONTEXT_MENU_MODEL[i] == "Manage plays" {
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
                    } else if CONTEXT_MENU_MODEL[i] == "Refetch details" {
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
                    } else if CONTEXT_MENU_MODEL[i] == "Change artworks" {
                        key_event_handler.bind_key(
                            (None, None),
                            'm',
                            "Change artworks".into(),
                            |app, _| {
                                app.drawer.open_change_artwork_popup(&app.tmdb_tokens);

                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.context_menu_pos = None;
                                }
                            },
                        );
                    } else if CONTEXT_MENU_MODEL[i] == "Delete" {
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
                let y = if pos.y + height > frame.area().height - 4 {
                    frame.area().height - 4 - height
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
                        if CONTEXT_MENU_MODEL[i] == "Add play" {
                            app.drawer.open_add_play_popup();
                        } else if CONTEXT_MENU_MODEL[i] == "Edit" {
                            app.drawer.open_edit_movie_popup(&app.watched.borrow());
                        } else if CONTEXT_MENU_MODEL[i] == "Manage plays" {
                            app.drawer.open_manage_plays_popup(&app.watched.borrow());
                        } else if CONTEXT_MENU_MODEL[i] == "Refetch details" {
                            app.drawer.open_refetch_details_popup(
                                app.tmdb_tokens.clone(),
                                app.punch_play_tokens.clone(),
                                app.trakt_tokens.clone(),
                                app.omdb_tokens.clone(),
                            );
                        } else if CONTEXT_MENU_MODEL[i] == "Change artworks" {
                            app.drawer.open_change_artwork_popup(&app.tmdb_tokens);
                        } else if CONTEXT_MENU_MODEL[i] == "Delete" {
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
                image_renderer.add_overlay(Rect {
                    x,
                    y,
                    width,
                    height: height + 2,
                });

                for i in 0..len {
                    let option_index = self.context_menu_model[i + self.context_menu.scroll_pos];
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        move |app, _| {
                            if CONTEXT_MENU_MODEL[option_index] == "Add play" {
                                app.drawer.open_add_play_popup();
                            } else if CONTEXT_MENU_MODEL[option_index] == "Edit" {
                                app.drawer.open_edit_movie_popup(&app.watched.borrow());
                            } else if CONTEXT_MENU_MODEL[option_index] == "Manage plays" {
                                app.drawer.open_manage_plays_popup(&app.watched.borrow());
                            } else if CONTEXT_MENU_MODEL[option_index] == "Refetch details" {
                                app.drawer.open_refetch_details_popup(
                                    app.tmdb_tokens.clone(),
                                    app.punch_play_tokens.clone(),
                                    app.trakt_tokens.clone(),
                                    app.omdb_tokens.clone(),
                                );
                            } else if CONTEXT_MENU_MODEL[option_index] == "Change artworks" {
                                app.drawer.open_change_artwork_popup(&app.tmdb_tokens);
                            } else if CONTEXT_MENU_MODEL[option_index] == "Delete" {
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
}
