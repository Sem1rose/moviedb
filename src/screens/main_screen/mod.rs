mod movie_description;
mod movies_list;
use crate::{
    app::App,
    draw::Drawer,
    screens::{
        main_screen::{movie_description::MovieDescription, movies_list::MoviesList},
        Screens,
    },
    types::*,
};
use crossterm::event::KeyModifiers;
use log::error;
use nucleo_matcher::{pattern::Atom, Config, Matcher};
use ratatui::style::Stylize;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, vertical};
use std::cmp::Ordering;
use style::palette::tailwind;
use tui_input::{backend::crossterm::EventHandler, Input};
// use threadpool::ThreadPool;

//                    id     backdrop/poster
// pub type MovieID = (usize, bool);

#[derive(Default)]
pub enum MainScreenEventRecievers {
    #[default]
    List,
    Description,
    Search,
}

#[derive(Default, Clone, Copy)]
pub enum Sort {
    #[default]
    AddedDate,
    UserRating,
    Relevance,
    IMDBRating,
    Name,
    ReleaseDate,
}

impl MainScreenEventRecievers {
    fn tab(&mut self) {
        *self = match self {
            MainScreenEventRecievers::List => MainScreenEventRecievers::Description,
            MainScreenEventRecievers::Description => MainScreenEventRecievers::List,
            MainScreenEventRecievers::Search => MainScreenEventRecievers::List,
        };
    }

    fn back_tab(&mut self) {
        *self = match self {
            MainScreenEventRecievers::List => MainScreenEventRecievers::Description,
            MainScreenEventRecievers::Description => MainScreenEventRecievers::List,
            MainScreenEventRecievers::Search => MainScreenEventRecievers::Description,
        };
    }
}

#[derive(Default)]
pub struct MainScreen {
    pub movies_list: MoviesList,
    pub movie_description: MovieDescription,

    pub filtered_movies: Vec<Movie>,

    pub selected: MainScreenEventRecievers,

    pub search_input: Input,

    prev_sort: Option<Sort>,
    pub sort: Sort,
    pub sort_ascending: bool,
}

impl MainScreen {
    pub fn current_movie(&self) -> &Movie {
        &self.filtered_movies[self.movies_list.current_movie_index()]
    }

    pub fn filter_sort_movies(&mut self, app: &App) {
        self.filter_movies(app, false);

        if self.movies_list.current_movie_index() >= self.filtered_movies.len() {
            self.movies_list.selected = self
                .movies_list
                .num_visible_movies
                .min(self.filtered_movies.len())
                .max(1)
                - 1;

            self.movies_list.scroll_pos =
                if self.filtered_movies.len() <= self.movies_list.num_visible_movies {
                    0
                } else {
                    self.filtered_movies.len() - 1 - self.movies_list.selected
                }
        }

        if let Sort::AddedDate = self.sort {
            return;
        }
        self.sort_movies(app);
    }

    fn filter_movies(&mut self, app: &App, sort: bool) {
        // if self.search_input.value().is_empty() {
        //     self.filtered_movies = app.movies.clone();
        // } else {
        //     self.filtered_movies = app
        //         .movies
        //         .iter()
        //         .filter(|x| x.match_search(self.search_input.value()))
        //         .cloned()
        //         .collect();
        //     self.movies_list.scroll_pos = 0;
        //     self.movies_list.selected = 0;
        // };

        // self.filtered_movies = app
        //     .movies
        //     .iter()
        //     .filter(|x| x.match_search("the"))
        //     .cloned()
        //     .collect();

        if self.search_input.value().is_empty() {
            self.filtered_movies = app.movies.clone();
            if let Some(sort) = self.prev_sort.take() {
                self.sort = sort;
            } else if let Sort::Relevance = self.sort {
                self.sort = Sort::default();
            }
            return;
        }

        if let Sort::Relevance = self.sort {
        } else {
            self.prev_sort = Some(self.sort);
            self.sort = Sort::Relevance;
        }

        let mut conf = Config::DEFAULT;
        conf.prefer_prefix = true;
        let mut matcher = Matcher::new(conf);
        let pattern = Atom::parse(
            self.search_input.value(),
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Never,
        );
        let mut scores = vec![];
        for movie in &app.movies {
            if let Some(score) = pattern.score(
                nucleo_matcher::Utf32Str::Ascii(
                    (movie.name.clone() + " " + &movie.year)
                        .to_string()
                        .as_bytes(),
                ),
                &mut matcher,
            ) {
                scores.push((score, movie));
            }
        }

        self.filtered_movies = if sort {
            scores.sort_by_key(|x| x.0);
            scores.reverse();
            scores.iter().map(|x| x.1.clone()).collect()
        } else {
            scores.iter().map(|x| x.1.clone()).collect()
        }
    }

    fn cmp_ratings(a: &Movie, b: &Movie) -> Ordering {
        let mut rating_a: f64 = 0.0;
        let mut rating_b: f64 = 0.0;

        for i in (0..a.ratings.len()).rev() {
            if let Rating::IMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::IMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::Trakt(r_a, c_a) = a.ratings[i] {
                if let Rating::Trakt(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::TMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::TMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::Metascore(r_a) = a.ratings[i] {
                if let Rating::Metascore(r_b) = b.ratings[i] {
                    if r_a == 0 || r_b == 0 {
                        continue;
                    }

                    rating_a = r_a as f64;
                    rating_b = r_b as f64;

                    break;
                }
            }
        }

        rating_a.partial_cmp(&rating_b).unwrap()
    }

    fn sort_movies(&mut self, app: &App) {
        match self.sort {
            Sort::AddedDate => self.filter_movies(app, false),
            Sort::Relevance => self.filter_movies(app, true),
            Sort::UserRating => {
                self.filtered_movies
                    .sort_by(|x, y| x.user_rating.partial_cmp(&y.user_rating).unwrap());
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::IMDBRating => {
                self.filtered_movies
                    .sort_by(|a, b| MainScreen::cmp_ratings(a, b));
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Name => {
                self.filtered_movies.sort_by_key(|x| x.name.clone());
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::ReleaseDate => {
                self.filtered_movies.sort_by_key(|x| x.year.clone());
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
        }
    }
}

impl Drawer {
    pub fn main_screen_handle_key_events(&mut self, app: &mut App, event: KeyEvent) {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return;
        }

        match code {
            KeyCode::Esc => {
                self.close_popups();
                if let MainScreenEventRecievers::Search = self.main_screen.selected {
                    self.main_screen.selected.tab();
                }
            }
            KeyCode::Tab => {
                self.main_screen.selected.tab();
            }
            KeyCode::BackTab => {
                self.main_screen.selected.back_tab();
            }
            KeyCode::Char('r') => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    self.image_backend.reload_images(
                        app,
                        self.main_screen.movies_list.scroll_pos,
                        Some(self.main_screen.movies_list.num_visible_movies),
                    );
                }
            }
            _ => (),
        }

        match self.main_screen.selected {
            MainScreenEventRecievers::List => match code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('a') => {
                    self.open_add_movie_popup();
                }
                KeyCode::Char('e') => {
                    self.open_edit_movie_popup();
                }
                KeyCode::Char('d') => {
                    self.open_remove_movie_popup();
                }
                KeyCode::Delete => {
                    self.open_remove_movie_popup();
                }
                KeyCode::Char('G') => {
                    self.main_screen.movies_list.goto_index(
                        self.main_screen.filtered_movies.len(),
                        self.main_screen.filtered_movies.len() - 1,
                    );
                }
                KeyCode::Char('g') => {
                    self.main_screen
                        .movies_list
                        .goto_index(self.main_screen.filtered_movies.len(), 0);
                }
                KeyCode::Char('/') => {
                    self.main_screen.search_input.reset();
                    self.main_screen.filter_sort_movies(app);
                    self.main_screen.selected = MainScreenEventRecievers::Search;
                }
                KeyCode::Char('f') => {
                    self.main_screen.search_input.reset();
                    self.main_screen.filter_sort_movies(app);
                    self.main_screen.selected = MainScreenEventRecievers::Search;
                }
                KeyCode::Up => {
                    self.main_screen.movies_list.dec_movie_selection();
                }
                KeyCode::Down => {
                    self.main_screen
                        .movies_list
                        .inc_movie_selection(self.main_screen.filtered_movies.len());
                }
                KeyCode::Right => {
                    self.main_screen.movie_description.next_tab();
                }
                KeyCode::Left => {
                    self.main_screen.movie_description.prev_tab();
                }
                KeyCode::Char('r') => {
                    if event.modifiers.contains(KeyModifiers::CONTROL) {
                        self.image_backend.reload_images(
                            app,
                            self.main_screen.movies_list.scroll_pos,
                            Some(self.main_screen.movies_list.num_visible_movies),
                        );
                    }
                }
                _ => (),
            },
            MainScreenEventRecievers::Description => match code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('a') => {
                    self.open_add_movie_popup();
                }
                KeyCode::Char('e') => {
                    self.open_edit_movie_popup();
                }
                KeyCode::Char('d') => {
                    self.open_remove_movie_popup();
                }
                KeyCode::Delete => {
                    self.open_remove_movie_popup();
                }
                KeyCode::Char('/') => {
                    self.main_screen.search_input.reset();
                    self.main_screen.filter_sort_movies(app);
                    self.main_screen.selected = MainScreenEventRecievers::Search;
                }
                KeyCode::Char('f') => {
                    self.main_screen.search_input.reset();
                    self.main_screen.filter_sort_movies(app);
                    self.main_screen.selected = MainScreenEventRecievers::Search;
                }
                KeyCode::Char('G') => {}
                KeyCode::Char('g') => {}
                KeyCode::Up => {}
                KeyCode::Down => {}
                KeyCode::Right => {
                    self.main_screen.movie_description.next_tab();
                }
                KeyCode::Left => {
                    self.main_screen.movie_description.prev_tab();
                }
                _ => (),
            },
            MainScreenEventRecievers::Search => match code {
                KeyCode::Esc => {
                    self.main_screen.selected.tab();
                }
                KeyCode::Enter => {
                    self.main_screen.selected.tab();
                }
                _ => {
                    let x = self.main_screen.search_input.value().to_string();
                    self.main_screen
                        .search_input
                        .handle_event(&Event::Key(event));
                    if self.main_screen.search_input.value() != x {
                        self.main_screen.filter_sort_movies(app);
                    }
                }
            },
        }
    }

    pub fn open_main_screen(&mut self) {
        self.close_popups();

        self.current_screen = Screens::MainScreen;
    }

    pub fn render_main_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);
        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), horiz_lay[0]);

        {
            let [_, area, _] = horizontal![>=1, <=25, ==1].areas(vert_lay[0]);

            let [_, input_top, input_center, input_bottom, _] =
                vertical![>=0, ==1, ==1, ==1,>=0].areas(area);

            let [_, input_area, _] = horizontal![==1, >=1, ==1].areas(input_center);

            // ▄▀█ ▂🮂▗▖▘▝
            frame.render_widget(
                Paragraph::new("🮃".repeat(input_bottom.width as usize)).fg(tailwind::RED.c700),
                input_bottom,
            );
            frame.render_widget(
                Paragraph::new("▂".repeat(input_top.width as usize)).fg(tailwind::RED.c700),
                input_top,
            );
            frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

            let width = input_area.width as usize - 1;
            let start = self.main_screen.search_input.visual_scroll(width);
            let cursor_pos = self.main_screen.search_input.cursor() - start;
            let mut chars = self.main_screen.search_input.value().chars().skip(start);

            let mut input_string: Vec<Span> = vec![];
            for i in 0..=(start + width) {
                let c = chars.next().unwrap_or(' ');
                if i == cursor_pos {
                    if let MainScreenEventRecievers::Search = self.main_screen.selected {
                        input_string.push(c.to_string().reversed());
                    } else {
                        input_string.push(c.to_string().into());
                    }
                } else {
                    input_string.push(c.to_string().into());
                }
            }
            frame.render_widget(Line::from_iter(input_string), input_area);
        }

        self.render_movies_list(frame, app, horiz_lay[1], num_movies)?;

        if !self.main_screen.filtered_movies.is_empty() {
            self.draw_movie_description(app, frame, horiz_lay[0]);
        }

        Ok(())
    }
}
