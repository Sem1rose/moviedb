use crate::{
    helpers::{add_padding, dynamic_popup},
    popups::Popups,
    widgets::{self, Action, ActionTypes},
    omdb::{self, OMDBDetailsResponse},
    key_event_handler::{self, KeyEventHandler},
    tokens::{OMDBTokens, TMDBTokens, TraktTokens},
    trakt::{self, TraktDetailsResponse, TraktSearchResponseMovie},
    tmdb::{self, TMDBDetailsResponse, TMDBSearchResult},
    types::Rating,
};
use anyhow::anyhow;
use itertools::Itertools;
use ratatui::{
    layout::*,
    macros::{constraint, horizontal, vertical, line, span},
    prelude::*,
    style::palette::material,
    style::palette::tailwind,
    symbols::{block, scrollbar::Set},
    widgets::*,
    Frame,
};
use ratatui_textarea::{TextArea, WrapMode};
use std::{
    ops::Add,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

#[derive(Default)]
pub enum Phase {
    #[default]
    SelectMovie,
    GetRating,
    GettingDetails,
    Error(String),
    Done,
}

#[derive(Clone)]
enum SearchResultID {
    TMDB(u32),
    IMDBTMDB(String, u32),
}
enum SearchResults {
    TMDB(anyhow::Result<Vec<TMDBSearchResult>>),
    Trakt(anyhow::Result<Vec<TraktSearchResponseMovie>>),
}

struct Movie {
    title: String,
    release_year: String,
    rating: Rating,
    id: SearchResultID,
}
impl From::<TMDBSearchResult> for Movie {
    fn from(value: TMDBSearchResult) -> Self {
        Self {
            title: value.title,
            release_year: value.release_date.unwrap_or("1970".into()),
            rating: Rating::TMDB(value.vote_average.unwrap_or(0.0), value.vote_count),
            id: SearchResultID::TMDB(value.id),
        }
    }
}
impl From::<TraktSearchResponseMovie> for Movie {
    fn from(value: TraktSearchResponseMovie) -> Self {
        Self {
            title: value.title,
            release_year: value.year.unwrap_or(1970).to_string(),
            rating: Rating::Trakt(value.rating, value.votes),
            id: SearchResultID::IMDBTMDB(value.ids.imdb, value.ids.tmdb),
        }
    }
}

struct DetailsResponse {
    pub trakt: Option<TraktDetailsResponse>,
    pub tmdb: Option<TMDBDetailsResponse>,
    pub omdb: Option<OMDBDetailsResponse>,
}

#[derive(Default)]
pub struct AddMoviePopup {
    pub tick: u64,
    pub phase: Phase,
    throbber_visible: bool,
    item: usize,
    tab: usize,
    scroll_pos: usize,
    selected_item: usize,
    alignment_bottom: bool,
    num_visible_items: usize,

    input: TextArea<'static>,
    throbber_state: ThrobberState,

    search_ticket: u64,
    last_input_tick: Option<u64>,
    search_results: Option<Vec<Movie>>,
    rx_search_result: Option<Receiver<(u64, SearchResults)>>,

    pub user_rating: f64,
    pub tmdb_movie_details_result: Option<TMDBDetailsResponse>,
    pub omdb_movie_details_result: Option<OMDBDetailsResponse>,
    pub trakt_movie_details_result: Option<TraktDetailsResponse>,
    rx_details_response: Option<Receiver<anyhow::Result<DetailsResponse>>>,

    tmdb_tokens: TMDBTokens,
    omdb_tokens: OMDBTokens,
    trakt_tokens: TraktTokens,

    cache_dir: PathBuf,
}

impl AddMoviePopup {
    pub fn new(
        trakt_tokens: TraktTokens,
        tmdb_tokens: TMDBTokens,
        omdb_tokens: OMDBTokens,
        cache_dir: &PathBuf,
    ) -> Self {
        Self {
            trakt_tokens,
            tmdb_tokens,
            omdb_tokens,
            cache_dir: cache_dir.clone(),
            ..Default::default()
        }
    }

    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    pub fn update_next_frame(&self) -> bool {
        self.throbber_visible || self.search_results.is_none()
    }

    pub fn request_search(&mut self) {
        let (tx_search_results, rx_search_results) = mpsc::channel();

        let search_string = self.input.lines()[0].clone();
        let access_token = self.tmdb_tokens.access_token_owned();
        let client_id = self.trakt_tokens.client_id_owned();
        let ticket = rand::random();
        self.search_ticket = ticket;

        thread::spawn(move || {
            if !client_id.is_empty() {
                _ = tx_search_results.send((ticket, SearchResults::Trakt(trakt::find_movie(&client_id, &search_string))));
            } else {
                _ = tx_search_results.send((ticket, SearchResults::TMDB(tmdb::find_movie(&access_token, &search_string))));
            }
        });

        self.rx_search_result = Some(rx_search_results);
    }

    pub fn request_details(&mut self) {
        let (tx_details_request, rx_details_request): (mpsc::Sender<anyhow::Result<DetailsResponse>>, Receiver<anyhow::Result<DetailsResponse>>) = mpsc::channel();

        let cache_dir = self.cache_dir.clone();
        let omdb_api_key = self.omdb_tokens.key_owned();
        let trakt_client_id = self.trakt_tokens.client_id_owned();
        let tmdb_access_token = self.tmdb_tokens.access_token_owned();
        let movie_id = self.search_results.as_ref().unwrap()[self.selected_item].id.clone();

        thread::spawn(move || {
            macro_rules! join_or_return {
                ($handle:expr) => {
                    match $handle.join() {
                        Err(e) => {
                            _ = tx_details_request.send(Err(anyhow!("{:#?}", e)));
                            return;
                        }
                        Ok(val) => val,
                    }
                };
            }

            let tmdb_result;
            let omdb_result;
            let trakt_result;
            let tmdb_id;
            let imdb_id;

            match movie_id {
                SearchResultID::TMDB(id) => {
                    tmdb_id = id;
                    let tmdb_handle = {
                        let access_token = tmdb_access_token.clone();
                        thread::spawn(move || {
                            tmdb::get_movie_details(&access_token, tmdb_id)
                        })
                    };
                    tmdb_result = join_or_return!(tmdb_handle);
                    if let Err(error) = tmdb_result {
                        _ = tx_details_request.send(Err(error));
                        return;
                    }

                    imdb_id = tmdb_result.as_ref().unwrap().imdb_id.clone();
                    let trakt_handle = {
                        let imdb_id = imdb_id.clone();
                        let client_id = trakt_client_id.clone();
                        thread::spawn(move || {
                            trakt::get_movie_details(&client_id, &imdb_id)
                        })
                    };
                    let omdb_handle = {
                        let imdb_id = imdb_id.clone();
                        thread::spawn(move || {
                            omdb::get_movie_details(&omdb_api_key, &imdb_id)
                        })
                    };
                    trakt_result = join_or_return!(trakt_handle);
                    omdb_result = join_or_return!(omdb_handle);
                },
                SearchResultID::IMDBTMDB(id_imdb, id_tmdb) => {
                    imdb_id = id_imdb;
                    tmdb_id = id_tmdb;
                    let trakt_handle = {
                        let imdb_id = imdb_id.clone();
                        let client_id = trakt_client_id.clone();
                        thread::spawn(move || {
                            trakt::get_movie_details(&client_id, &imdb_id)
                        })
                    };
                    let omdb_handle = {
                        let imdb_id = imdb_id.clone();
                        thread::spawn(move || {
                            omdb::get_movie_details(&omdb_api_key, &imdb_id)
                        })
                    };
                    let tmdb_handle = {
                        let access_token = tmdb_access_token.clone();
                        thread::spawn(move || {
                            tmdb::get_movie_details(&access_token, tmdb_id)
                        })
                    };
                    trakt_result = join_or_return!(trakt_handle);
                    if let Err(error) = trakt_result {
                        _ = tx_details_request.send(Err(error));
                        return;
                    }
                    omdb_result = join_or_return!(omdb_handle);
                    tmdb_result = join_or_return!(tmdb_handle);
                },
            }

            let result = trakt::get_movie_poster_banner(
                &cache_dir,
                &trakt_client_id,
                &imdb_id,
            );
            if result.is_ok() {
            } else {
                _ = tmdb::get_movie_poster_banner(
                    &cache_dir,
                    &tmdb_access_token,
                    tmdb_id,
                );
            }

            _ = tx_details_request.send(Ok(DetailsResponse {
                trakt: trakt_result.ok(),
                tmdb: tmdb_result.ok(),
                omdb: omdb_result.ok(),
            }));
        });

        self.rx_details_response = Some(rx_details_request);
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::SelectMovie => {
                self.item = 1;
                self.input = TextArea::from([""]);
                Phase::GetRating
            }
            Phase::GetRating => {
                self.user_rating = format!("{:.1}", self.input.lines()[0].parse::<f64>().unwrap())
                    .parse()
                    .unwrap();

                self.request_details();

                Phase::GettingDetails
            }
            Phase::GettingDetails => Phase::Done,
            _ => Phase::SelectMovie,
        };
    }

    pub fn validate_input_rating(&mut self) -> bool {
        if self.input.is_empty() {
            return false;
        }

        if let Ok(x) = self.input.lines()[0].parse() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }
        if let Some(last_tick) = self.last_input_tick {
            if self.tick - last_tick > 20 && matches!(self.phase, Phase::SelectMovie){
                self.last_input_tick = None;

                self.selected_item = 0;
                self.scroll_pos = 0;
                self.search_results = None;
                self.request_search();
            }
        }
        match self.phase {
            Phase::SelectMovie => {
                if let Some(rx_search_results) = self.rx_search_result.as_ref() {
                    if let Ok((ticket, search_result)) = rx_search_results.try_recv() {
                        if ticket != self.search_ticket {
                            return;
                        }

                        self.search_results = match search_result {
                            SearchResults::TMDB(tmdbsearch_results) => {
                                if let Ok(results) = tmdbsearch_results {
                                    Some(results.into_iter().map(|x| x.into()).collect_vec())
                                } else {
                                    None
                                }
                            },
                            SearchResults::Trakt(trakt_results) => {
                                if let Ok(results) = trakt_results {
                                    Some(results.into_iter().map(|x| x.into()).collect_vec())
                                } else {
                                    None
                                }
                            },
                        };
                    }
                }
            }
            Phase::GettingDetails => {
                match self.rx_details_response.as_ref().unwrap().try_recv() {
                    Ok(details_response) => {
                        if let Ok(details_response) = details_response {
                            self.tmdb_movie_details_result = details_response.tmdb;
                            self.trakt_movie_details_result = details_response.trakt;
                            self.omdb_movie_details_result = details_response.omdb;

                            self.advance_phase();

                            self.rx_details_response = None;
                        } else if let Err(error) = details_response {
                            self.rx_details_response = None;
                            self.phase = Phase::Error(format!("{error}"));
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.rx_details_response = None;
                        self.phase = Phase::Error("Error while fetching movie details".into());
                    }
                    _ => ()
                }
            }
            _ => (),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popups();
            },
        );
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popups();
        });

        let num_results = if let Some(search_results) = self.search_results.as_ref() {
            search_results.len()
        } else {
            0
        };
        self.throbber_visible = false;
        match &self.phase {
            Phase::SelectMovie => {
                self.tab = 0;

                key_event_handler.bind_vertical(
                    (Some(self.tab), None),
                    "Scroll".into(),
                    move |app, data| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Direction(true, _) => {
                                    add_movie_popup.selected_item = add_movie_popup
                                        .selected_item
                                        .add(1)
                                        .min(num_results.saturating_sub(1));
                                    if add_movie_popup.selected_item - add_movie_popup.scroll_pos
                                        >= add_movie_popup.num_visible_items
                                    {
                                        add_movie_popup.scroll_pos += 1;
                                    }
                                }
                                key_event_handler::Data::Direction(false, _) => {
                                    add_movie_popup.selected_item =
                                        add_movie_popup.selected_item.saturating_sub(1);
                                    if add_movie_popup.selected_item < add_movie_popup.scroll_pos {
                                        add_movie_popup.scroll_pos -= 1;
                                    }
                                }
                                _ => (),
                            }
                        }
                    },
                );
                if num_results > 0 {
                    key_event_handler.bind_enter((Some(self.tab), None), "Select".into(), |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.advance_phase();
                        }
                    });
                }
                key_event_handler.bind_input_field((Some(self.tab), None), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Key(key_event) => {
                                let old_query = add_movie_popup.input.lines()[0].clone();
                                add_movie_popup.input.input(key_event);

                                if add_movie_popup.input.lines()[0].trim() != old_query.trim()
                                    && !add_movie_popup.input.is_empty()
                                {
                                    add_movie_popup.search_ticket = 0;
                                    add_movie_popup.search_results = None;
                                    add_movie_popup.last_input_tick = Some(add_movie_popup.tick);
                                } else if add_movie_popup.input.is_empty() {
                                    add_movie_popup.search_results = None;
                                }
                            }
                            _ => (),
                        }
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(26),
                    2.4,
                    tailwind::BLUE.c950,
                    "  Add movie  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [search_input_area, horiz] = vertical![==3, >=1].areas(popup_area);
                let [results_list_area, scrollbar_area] = horizontal![>=1, ==1].areas(horiz);

                widgets::input_field(true, true, &mut self.input, WrapMode::None, frame, search_input_area, (0, 1), " Name ", "Search");

                let num_visible_results = results_list_area.height as usize / 5;
                let partially_visible_result_height =
                    results_list_area.height as usize - num_visible_results * 5;
                let render_partially_visible_result = partially_visible_result_height > 0;
                self.num_visible_items = num_visible_results
                    + if render_partially_visible_result {
                        1
                    } else {
                        0
                    };

                if self.selected_item < self.scroll_pos {
                    self.selected_item =
                        self.selected_item.add(1).min(num_results.saturating_sub(1));
                } else if self.selected_item >= num_results {
                    self.selected_item = num_results.saturating_sub(1);
                    self.scroll_pos = self
                        .selected_item
                        .saturating_sub(self.num_visible_items + 1);
                } else if self.selected_item - self.scroll_pos >= self.num_visible_items {
                    self.scroll_pos = self
                        .selected_item
                        .saturating_sub(self.num_visible_items + 1);
                }

                if num_results <= num_visible_results {
                    self.alignment_bottom = false;
                } else if self.selected_item - self.scroll_pos == 0 {
                    self.alignment_bottom = false;
                } else if self.selected_item - self.scroll_pos == self.num_visible_items - 1 {
                    self.alignment_bottom = true;
                }

                let mut remaining_area = results_list_area;
                for i in 0..self.num_visible_items {
                    let [area, remaining] =
                        if render_partially_visible_result && i == 0 && self.alignment_bottom {
                            vertical![==partially_visible_result_height as u16, >= 0]
                        } else if render_partially_visible_result
                            && i == self.num_visible_items - 1
                            && !self.alignment_bottom
                        {
                            vertical![==partially_visible_result_height as u16, >= 0]
                        } else {
                            vertical![==5, >= 0]
                        }
                        .areas(remaining_area);

                    if self.scroll_pos + i < num_results {
                        let result = &self.search_results.as_ref().unwrap()[self.scroll_pos + i];
                        let partially_visible = area.height < 5;

                        let alternate = i & 1 == 1;
                        let selected = self.selected_item == i + self.scroll_pos;

                        frame.render_widget(
                            Block::new().bg(if selected {
                                // if !input_selected {
                                tailwind::TEAL.c600
                                // } else {
                                //     tailwind::TEAL.c900
                                // }
                            } else if !alternate {
                                tailwind::GRAY.c600
                            } else {
                                tailwind::SLATE.c700
                            }),
                            area,
                        );
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            area,
                            move |app, _| {
                                if let Some(Popups::AddMovie(add_movie_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    if selected {
                                        //&& add_movie_popup.item == 1 {
                                        add_movie_popup.advance_phase();
                                    } else {
                                        // add_movie_popup.item = 1;
                                        add_movie_popup.selected_item =
                                            add_movie_popup.scroll_pos + i;
                                    }
                                }
                            },
                        );

                        let areas =
                            Layout::vertical(vec![Constraint::Length(1); area.height as usize])
                                .split(area);

                        for i in 0..area.height {
                            let index = if partially_visible {
                                if self.alignment_bottom {
                                    i + (5 - area.height)
                                } else {
                                    i
                                }
                            } else {
                                i
                            };
                            if index == 0 {
                                frame.render_widget(
                                    Line::from("▔".repeat(area.width as usize)).style(
                                        Style::new().fg(if selected {
                                            // if !input_selected {
                                            tailwind::EMERALD.c700
                                            // } else {
                                            //     tailwind::EMERALD.c800
                                            // }
                                        } else if !alternate {
                                            tailwind::GRAY.c600
                                        } else {
                                            tailwind::SLATE.c600
                                        }),
                                    ),
                                    areas[i as usize],
                                );
                            } else if index == 1 {
                                frame.render_widget(
                                    line![
                                        span!(&result.title)
                                            .fg(if selected {
                                                material::CYAN.c100
                                            } else {
                                                material::ORANGE.c400
                                            })
                                            .add_modifier(if selected {
                                                Modifier::BOLD
                                            } else {
                                                Modifier::empty()
                                            }),
                                        span!("  "),
                                        span!(result.release_year)
                                            .fg(if selected {
                                                material::CYAN.c100
                                            } else {
                                                material::ORANGE.c400
                                            })
                                            .add_modifier(if selected {
                                                Modifier::BOLD
                                            } else {
                                                Modifier::empty()
                                            })
                                            .italic(),
                                    ].left_aligned(),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            } else if index == 3 {
                                frame.render_widget(
                                    line![span!("{:.1}", f64::from(result.rating))
                                        .fg(if selected {
                                            material::CYAN.c100
                                        } else {
                                            material::ORANGE.c400
                                        })
                                        .add_modifier(if selected {
                                            Modifier::BOLD
                                        } else {
                                            Modifier::empty()
                                        }),
                                    ].left_aligned(),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            } else if index == 4 {
                                frame.render_widget(
                                    Line::from("▁".repeat(area.width as usize)).style(
                                        Style::new().fg(if selected {
                                            tailwind::EMERALD.c700
                                        } else if !alternate {
                                            tailwind::GRAY.c600
                                        } else {
                                            tailwind::SLATE.c600
                                        }),
                                    ),
                                    areas[i as usize],
                                );
                            }
                        }
                    } else {
                        frame.render_widget(
                            Block::new().bg(if i & 1 == 0 {
                                tailwind::SLATE.c950
                            } else {
                                tailwind::BLACK
                            }),
                            area,
                        );
                    }

                    remaining_area = remaining;
                }

                let scrollbar =
                    Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                        .symbols(Set {
                            track: block::FULL,
                            thumb: block::FULL, //"🮋",
                            begin: "▲",
                            end: "▼",
                        })
                        .begin_style(
                            Style::new()
                                .bg(material::LIGHT_BLUE.c700)
                                .fg(tailwind::INDIGO.c900),
                        )
                        .end_style(
                            Style::new()
                                .bg(material::LIGHT_BLUE.c700)
                                .fg(tailwind::INDIGO.c900),
                        )
                        .track_style(Style::new().fg(tailwind::SLATE.c900))
                        .thumb_style(
                            Style::new()
                                .fg(material::BLUE.c800)
                                .bg(tailwind::SLATE.c900),
                        );
                let mut scrollbar_state =
                    ScrollbarState::new(num_results.saturating_sub(self.num_visible_items - 1))
                        .position(self.scroll_pos);

                frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }
            Phase::GetRating => {
                self.tab = 1;
                let valid = self.validate_input_rating();

                key_event_handler.bind_tab((Some(self.tab), None), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                add_movie_popup.item += 1;
                                if add_movie_popup.item > 2 {
                                    add_movie_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                add_movie_popup.item =
                                    add_movie_popup.item.checked_sub(1).unwrap_or(2);
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_horizontal((Some(self.tab), Some(2)), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                add_movie_popup.item = 3;
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_horizontal((Some(self.tab), Some(3)), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(false, _) => {
                                add_movie_popup.item = 2;
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_enter((Some(self.tab), Some(3)), "Cancel".into(), |app, _| {
                    app.drawer.close_popups();
                });
                key_event_handler.bind_enter((Some(self.tab), Some(0)), "Back".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                        add_movie_popup.phase = Phase::SelectMovie;
                        add_movie_popup.input = TextArea::from([""]);
                    }
                });
                if valid {
                    key_event_handler.bind_enter((Some(self.tab), None), "Confirm".into(), |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.advance_phase();
                            add_movie_popup.throbber_visible = true;
                        }
                    });
                }
                key_event_handler.bind_input_field((Some(self.tab), Some(1)), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Key(key_event) => {
                                add_movie_popup.input.input(key_event);
                            }
                            _ => (),
                        }
                    }
                });
                key_event_handler.bind_esc((Some(self.tab), Some(0)), "Close".into(), |app, _| {
                    app.drawer.close_popups();
                });
                key_event_handler.bind_esc((Some(self.tab), None), "Back".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(9),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Add movie  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [ _, input_area, _, actions_area] =
                    vertical![==1, ==3, >=1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));

                let mouse_area = widgets::action(Action::new(" Back ", ActionTypes::Normal, self.item == 0, true), HorizontalAlignment::Left, popup_area, frame);
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 0;
                            add_movie_popup.phase = Phase::SelectMovie;
                            add_movie_popup.input = TextArea::from([""]);
                        }
                    },
                );

                let actions_mouse_areas = widgets::actions([Action::new(" Confirm ", ActionTypes::Default, self.item == 2, valid), Action::new(" Cancel ", ActionTypes::Critical, self.item == 3, true)], HorizontalAlignment::Right, 1, actions_area, frame);
                for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        move |app, _| {
                            if let Some(Popups::AddMovie(add_movie_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if i == 0 {
                                    if valid {
                                        add_movie_popup.advance_phase();
                                        add_movie_popup.throbber_visible = true;
                                    }
                                } else {
                                    app.drawer.close_popups();
                                }
                            }
                        },
                    );
                }

                let input_selected = self.item == 1;
                widgets::input_field(input_selected, valid, &mut self.input, WrapMode::None, frame, input_area, (0, 0), " Rating ", "Enter a rating");
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    input_area,
                    |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 1;
                        }
                    },
                );
            }
            Phase::GettingDetails | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = dynamic_popup(
                    frame,
                    Some(8),
                    5.0,
                    tailwind::BLUE.c950,
                    "  Add movie  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, throbber_area, _] =
                    vertical![>=1, ==1, >=1].areas(add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(Paragraph::new("Getting details").centered(), message_area);

                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                        .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                    throbber_area.centered(constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::Error(error) => {
                self.tab = 2;
                key_event_handler.bind_enter((Some(2), None), "Back".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                        add_movie_popup.phase = Phase::SelectMovie;
                        add_movie_popup.input = TextArea::from([""]);
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(9),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Error  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, _, actions_area] =
                    vertical![>=1, ==1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(Paragraph::new(error.as_str()).centered(), message_area);

                let mouse_area = widgets::action(Action::new(" Back ", ActionTypes::Default, true, true), HorizontalAlignment::Center, actions_area, frame);
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 0;
                            add_movie_popup.phase = Phase::SelectMovie;
                            add_movie_popup.input = TextArea::from([""]);
                        }
                    },
                );
            }
        }
    }
}
