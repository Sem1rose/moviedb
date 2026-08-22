use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};

use chrono::{DateTime, Datelike, Local, Utc};
use itertools::Itertools;
use log::error;
use punch_play::{
    self,
    smo::{DetailsResponse, ItemDetails as PunchPlayItemDetails},
};
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{HorizontalAlignment, Layout, Margin, Offset, Position, Size},
    macros::{constraint, horizontal, line, span, vertical},
    style::{
        Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    text::Text,
    widgets::{Block, Padding},
};
use ratatui_textarea::{TextArea, WrapMode};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tmdb::{
    self,
    smo::{MovieDetails as TMDBMovieDetails, SearchResult},
};
use trakt::{
    self,
    smo::{TraktDetailsResponse, TraktSearchResponseMovie},
};

use crate::{
    app::App,
    helpers,
    key_event_handler::{self, KeyEventHandler},
    omdb::OMDBDetailsResponse,
    popups::{Popup, PopupTrait},
    tokens::{OMDBTokens, PunchPlayTokens, TMDBTokens, TraktTokens},
    types::MovieDetailsResponse,
    widgets::{self, Action, ActionType, ScrollView},
};

#[derive(Default)]
pub enum Phase {
    ConfirmRefetchDetails(u32),
    #[default]
    SelectMovie,
    GetRating,
    GettingDetails,
    Error(String),
    Done,
}

#[allow(clippy::upper_case_acronyms)]
enum SearchResults {
    Trakt(anyhow::Result<Vec<TraktSearchResponseMovie>>),
    PunchPlay(anyhow::Result<Vec<PunchPlayItemDetails>>),
    TMDB(anyhow::Result<Vec<SearchResult>>),
}
struct SearchResultMovie {
    title:        String,
    release_year: u32,
    rating:       f64,
    id:           u32,
}
impl From<TraktSearchResponseMovie> for SearchResultMovie {
    fn from(value: TraktSearchResponseMovie) -> Self {
        Self {
            title:        value.title,
            release_year: value.year.unwrap_or(1970) as u32,
            rating:       value.rating,
            id:           value.ids.tmdb,
        }
    }
}
impl From<PunchPlayItemDetails> for SearchResultMovie {
    fn from(value: PunchPlayItemDetails) -> Self {
        Self {
            title:        value.name,
            release_year: value.release_date.year() as u32,
            rating:       value.community_rating,
            id:           value.tmdb_id,
        }
    }
}
impl From<SearchResult> for SearchResultMovie {
    fn from(value: SearchResult) -> Self {
        Self {
            title:        value.title,
            release_year: value.release_date.year() as u32,
            rating:       value.vote_average.unwrap_or(0.0),
            id:           value.id,
        }
    }
}

#[derive(Default)]
pub struct AddMoviePopup {
    pub tick:         u64,
    pub phase:        Phase,
    throbber_visible: bool,
    item:             usize,
    scrollview:       ScrollView,

    input0:         TextArea<'static>,
    input1:         TextArea<'static>,
    throbber_state: ThrobberState,

    last_input_tick:  Option<u64>,
    search_results:   Option<Vec<SearchResultMovie>>,
    rx_search_result: Option<Receiver<SearchResults>>,

    take_rating:                         bool,
    pub refetch_details:                 bool,
    pub user_rating:                     f64,
    pub date:                            DateTime<Utc>,
    pub trakt_movie_details_result:      Option<TraktDetailsResponse>,
    pub punch_play_movie_details_result: Option<DetailsResponse>,
    pub tmdb_movie_details_result:       Option<TMDBMovieDetails>,
    pub omdb_movie_details_result:       Option<OMDBDetailsResponse>,
    rx_details_response:                 Option<Receiver<anyhow::Result<MovieDetailsResponse>>>,

    tmdb_tokens:       TMDBTokens,
    punch_play_tokens: PunchPlayTokens,
    trakt_tokens:      TraktTokens,
    omdb_tokens:       OMDBTokens,

    _cache_dir: PathBuf,
}

impl AddMoviePopup {
    pub fn new(
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
        take_rating: bool,
        cache_dir: &Path,
    ) -> Self {
        Self {
            tmdb_tokens,
            punch_play_tokens,
            trakt_tokens,
            omdb_tokens,
            take_rating,
            scrollview: ScrollView::new(5),

            _cache_dir: cache_dir.to_path_buf(),
            ..Default::default()
        }
    }

    pub fn new_refetch_details(
        tmdb_id: u32,
        tmdb_tokens: TMDBTokens,
        punch_play_tokens: PunchPlayTokens,
        trakt_tokens: TraktTokens,
        omdb_tokens: OMDBTokens,
        cache_dir: &Path,
    ) -> Self {
        Self {
            trakt_tokens,
            punch_play_tokens,
            tmdb_tokens,
            omdb_tokens,
            _cache_dir: cache_dir.to_path_buf(),
            refetch_details: true,
            phase: Phase::ConfirmRefetchDetails(tmdb_id),
            ..Default::default()
        }
    }

    pub fn request_search(&mut self) {
        let (tx_search_results, rx_search_results) = mpsc::channel();

        let search_string = self.input0.lines()[0].trim().to_string();
        let trakt_status = self.trakt_tokens.status;
        let punch_play_status = self.punch_play_tokens.status;
        let tmdb_status = self.tmdb_tokens.status;
        let client_id = self.trakt_tokens.client_id_owned();
        let tmdb_access_token = self.tmdb_tokens.access_token_owned();

        thread::spawn(move || {
            if tmdb_status.is_some() {
                _ = tx_search_results.send(SearchResults::TMDB(tmdb::movie::find_movie(
                    &tmdb_access_token,
                    &search_string,
                )));
            } else if trakt_status.is_some() {
                _ = tx_search_results.send(SearchResults::Trakt(trakt::movie::find_movie(
                    &client_id,
                    &search_string,
                )));
            } else if punch_play_status.is_some() {
                _ = tx_search_results.send(SearchResults::PunchPlay(
                    punch_play::movie::find_movie(&search_string),
                ));
            }
        });

        self.rx_search_result = Some(rx_search_results);
    }

    pub fn request_details(&mut self, tmdb_id: u32) {
        let (tx_details_request, rx_details_response): (
            mpsc::Sender<anyhow::Result<MovieDetailsResponse>>,
            Receiver<anyhow::Result<MovieDetailsResponse>>,
        ) = mpsc::channel();

        let trakt_status = self.trakt_tokens.status;
        let punch_play_status = self.punch_play_tokens.status;
        let omdb_status = self.omdb_tokens.status;
        let omdb_api_key = self.omdb_tokens.key_owned();
        let trakt_client_id = self.trakt_tokens.client_id_owned();
        let punch_play_access_token = self.punch_play_tokens.access_token_owned();
        let tmdb_access_token = self.tmdb_tokens.access_token_owned();
        // let cache_dir = self.cache_dir.clone();

        thread::spawn(move || {
            _ = tx_details_request.send(App::fetch_movie_details(
                &omdb_api_key,
                &trakt_client_id,
                &punch_play_access_token,
                &tmdb_access_token,
                trakt_status,
                punch_play_status,
                omdb_status,
                tmdb_id,
            ));
        });

        self.rx_details_response = Some(rx_details_response);
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::SelectMovie => {
                self.item = 1;
                self.input0 = TextArea::from([""]);
                self.input1 = TextArea::from([""]);
                if self.take_rating {
                    Phase::GetRating
                } else {
                    self.request_details(
                        self.search_results.as_ref().unwrap()[self.scrollview.selected_index].id,
                    );

                    Phase::GettingDetails
                }
            }
            Phase::GetRating => {
                self.user_rating = format!("{:.1}", self.input0.lines()[0].parse::<f64>().unwrap())
                    .parse()
                    .unwrap();
                let input = self.input1.lines()[0].to_lowercase();
                self.date = if ["now", ""].contains(&input.trim()) {
                    chrono::Local::now()
                } else if input.trim() == "unknown" {
                    Default::default()
                } else {
                    self.input1.lines()[0].parse().unwrap()
                }
                .to_utc();

                self.request_details(
                    self.search_results.as_ref().unwrap()[self.scrollview.selected_index].id,
                );

                Phase::GettingDetails
            }
            Phase::GettingDetails => Phase::Done,
            Phase::ConfirmRefetchDetails(tmdb_id) => {
                self.request_details(tmdb_id);

                Phase::GettingDetails
            }
            _ => Phase::SelectMovie,
        };
    }

    pub fn validate_input_rating(&mut self) -> bool {
        if self.input0.is_empty() {
            return false;
        }

        if let Ok(x) = self.input0.lines()[0].parse() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn validate_input_date(&mut self) -> bool {
        ["now", "unknown", ""].contains(&self.input1.lines()[0].trim().to_lowercase().as_str())
            || self.input1.lines()[0].parse::<DateTime<Local>>().is_ok()
    }
}

impl PopupTrait for AddMoviePopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.throbber_visible || self.last_input_tick.is_some()
    }

    fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }
        if matches!(self.phase, Phase::SelectMovie) {
            if let Some(last_tick) = self.last_input_tick {
                if self.tick - last_tick > 20 {
                    self.last_input_tick = None;

                    self.scrollview.reset();
                    self.search_results = None;
                    if self.input0.lines()[0].trim().is_empty() {
                        _ = self.rx_search_result.take();
                    } else {
                        self.request_search();
                    }
                }
            }
        }
        match self.phase {
            Phase::SelectMovie =>
                if let Some(rx_search_results) = self.rx_search_result.as_ref() {
                    if let Ok(search_result) = rx_search_results.try_recv() {
                        self.search_results = match search_result {
                            SearchResults::TMDB(tmdb_results) => match tmdb_results {
                                Ok(results) =>
                                    Some(results.into_iter().map(|x| x.into()).collect_vec()),
                                Err(error) => {
                                    error!("TMDB error while searching: {error:#?}");
                                    None
                                }
                            },
                            SearchResults::PunchPlay(punch_play_results) =>
                                match punch_play_results {
                                    Ok(results) =>
                                        Some(results.into_iter().map(|x| x.into()).collect_vec()),
                                    Err(error) => {
                                        error!("PunchPlay error while searching: {error:#?}");
                                        None
                                    }
                                },
                            SearchResults::Trakt(trakt_results) => match trakt_results {
                                Ok(results) =>
                                    Some(results.into_iter().map(|x| x.into()).collect_vec()),
                                Err(error) => {
                                    error!("Trakt error while searching: {error:#?}");
                                    None
                                }
                            },
                        };
                        _ = self.rx_search_result.take();
                    }
                },
            Phase::GettingDetails => match self.rx_details_response.as_ref().unwrap().try_recv() {
                Ok(details_response) =>
                    if let Ok(details_response) = details_response {
                        self.trakt_movie_details_result = details_response.trakt;
                        self.punch_play_movie_details_result = details_response.punch_play;
                        self.tmdb_movie_details_result = details_response.tmdb;
                        self.omdb_movie_details_result = details_response.omdb;

                        self.advance_phase();

                        self.rx_details_response = None;
                    } else if let Err(error) = details_response {
                        self.item = 0;
                        self.rx_details_response = None;
                        self.phase = Phase::Error(format!("{error}"));
                    },
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.item = 0;
                    self.rx_details_response = None;
                    self.phase = Phase::Error("Error while fetching movie details".into());
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        let num_results = self.search_results.as_ref().map(|x| x.len());
        self.throbber_visible = false;
        match &self.phase {
            Phase::SelectMovie => {
                let num_results = num_results.unwrap_or_default();
                if num_results > 0 {
                    key_event_handler.bind_vertical(
                        (None, None),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Popup::AddMovie(add_movie_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let key_event_handler::Data::Direction(direction, _) = data {
                                    add_movie_popup.scrollview.scroll(direction, num_results);
                                }
                            }
                        },
                    );

                    key_event_handler.bind_enter((None, None), "Select".into(), |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.advance_phase();
                        }
                    });
                }
                key_event_handler.bind_input_field((None, None), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            let old_query = add_movie_popup.input0.lines()[0].clone();
                            add_movie_popup.input0.input(key_event);
                            let input_empty = add_movie_popup.input0.lines()[0].trim().is_empty();

                            if add_movie_popup.input0.lines()[0].trim() != old_query.trim()
                                && !input_empty
                            {
                                add_movie_popup.search_results = None;
                                _ = add_movie_popup.rx_search_result.take();
                                add_movie_popup.last_input_tick = Some(add_movie_popup.tick);
                            } else if input_empty {
                                add_movie_popup.search_results = None;
                                _ = add_movie_popup.rx_search_result.take();
                            }
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(28, 66, frame.area()),
                    " Add movie ",
                    false,
                );
                for i in 0..popup_area.width {
                    for j in 0..popup_area.height {
                        frame
                            .buffer_mut()
                            .cell_mut(Position::new(popup_area.x + i, popup_area.y + j))
                            .unwrap()
                            .set_diff_option(ratatui::buffer::CellDiffOption::AlwaysUpdate);
                    }
                }
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [search_input_area, horiz] = vertical![==3, >=1].areas(popup_area);
                let [results_list_area, scrollbar_area] = horizontal![>=1, ==1].areas(horiz);

                widgets::input_field(
                    true,
                    true,
                    true,
                    &mut self.input0,
                    WrapMode::None,
                    frame,
                    search_input_area,
                    " Name ",
                    "Search",
                    Some(Padding::new(1, 2, 0, 0)),
                );

                if self.rx_search_result.is_some() {
                    self.throbber_visible = true;
                    frame.render_stateful_widget(
                        Throbber::default()
                            .throbber_set(throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE)
                            .throbber_style(Style::new().bold().fg(tailwind::CYAN.c600).bold()),
                        search_input_area
                            .offset(Offset::new(search_input_area.width as i32 - 3, 1))
                            .resize(Size::new(1, 1)),
                        &mut self.throbber_state,
                    );
                }

                self.scrollview.render(
                    num_results,
                    results_list_area,
                    scrollbar_area,
                    frame,
                    key_event_handler,
                    |scroll_view, area, index, selected, alternate, frame, key_event_handler| {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            area,
                            move |app, _| {
                                if let Some(Popup::AddMovie(add_movie_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    if selected {
                                        add_movie_popup.advance_phase();
                                    } else {
                                        add_movie_popup.scrollview.goto_index(
                                            index,
                                            false,
                                            num_results,
                                        );
                                    }
                                }
                            },
                        );

                        frame.render_widget(
                            Block::new().bg(if selected {
                                tailwind::TEAL.c600
                            } else if !alternate {
                                tailwind::GRAY.c600
                            } else {
                                tailwind::SLATE.c700
                            }),
                            area,
                        );

                        let result = &self.search_results.as_ref().unwrap()[index];
                        let areas = Layout::vertical(vec![constraint!(==1); area.height as usize])
                            .split(area);
                        for i in 0..area.height {
                            let index = if area.height < scroll_view.item_height {
                                if scroll_view.alignment_bottom {
                                    i + (scroll_view.item_height - area.height)
                                } else {
                                    i
                                }
                            } else {
                                i
                            };
                            if index == 0 {
                                frame.render_widget(
                                    line!("▔".repeat(area.width as usize)).fg(if selected {
                                        tailwind::EMERALD.c700
                                    // } else if !alternate {
                                    //     tailwind::GRAY.c600
                                    } else {
                                        tailwind::SLATE.c600
                                    }),
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
                                    ]
                                    .left_aligned(),
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
                                );
                            } else if index == 3 {
                                frame.render_widget(
                                    line![format!("{:.1}", result.rating)]
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
                                        .left_aligned(),
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
                                );
                            } else if index == 4 {
                                frame.render_widget(
                                    line!("▁".repeat(area.width as usize)).fg(if selected {
                                        tailwind::EMERALD.c700
                                    // } else if !alternate {
                                    //     tailwind::GRAY.c600
                                    } else {
                                        tailwind::SLATE.c600
                                    }),
                                    areas[i as usize],
                                );
                            }
                        }
                    },
                );

                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    scrollbar_area.resize(Size::new(1, 1)),
                    move |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if add_movie_popup.scrollview.alignment_bottom
                                && add_movie_popup.scrollview.partially_visible
                            {
                                add_movie_popup.scrollview.alignment_bottom = false;
                            } else if add_movie_popup.scrollview.scroll_pos > 0 {
                                add_movie_popup.scrollview.scroll_pos -= 1;
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
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if !add_movie_popup.scrollview.alignment_bottom
                                && add_movie_popup.scrollview.partially_visible
                            {
                                add_movie_popup.scrollview.alignment_bottom = true;
                            } else if add_movie_popup.scrollview.scroll_pos
                                < num_results
                                    .saturating_sub(add_movie_popup.scrollview.num_visible_items)
                            {
                                add_movie_popup.scrollview.scroll_pos += 1;
                            }
                        }
                    },
                );
            }
            Phase::GetRating => {
                let rating_valid = self.validate_input_rating();
                let date_valid = self.validate_input_date();

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                add_movie_popup.item += 1;
                                if add_movie_popup.item > 3 {
                                    add_movie_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                add_movie_popup.item =
                                    add_movie_popup.item.checked_sub(1).unwrap_or(3);
                            }
                            _ => {}
                        }
                    }
                });

                key_event_handler.bind_esc((None, Some(0)), "Close".into(), |app, _| {
                    app.drawer.close_popup();
                });
                key_event_handler.bind_esc((None, None), "Back".into(), |app, _| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                    }
                });
                key_event_handler.bind_esc((None, Some(4)), "Close".into(), |app, _| {
                    app.drawer.close_popup();
                });

                key_event_handler.bind_horizontal((None, Some(3)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let crate::key_event_handler::Data::Direction(true, _) = data {
                            add_movie_popup.item = 4;
                        }
                    }
                });
                key_event_handler.bind_horizontal((None, Some(4)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let crate::key_event_handler::Data::Direction(false, _) = data {
                            add_movie_popup.item = 3;
                        }
                    }
                });

                key_event_handler.bind_vertical((None, Some(1)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let crate::key_event_handler::Data::Direction(true, _) = data {
                            add_movie_popup.item = 2;
                        }
                    }
                });
                key_event_handler.bind_vertical((None, Some(2)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let crate::key_event_handler::Data::Direction(false, _) = data {
                            add_movie_popup.item = 1;
                        }
                    }
                });

                key_event_handler.bind_enter((None, Some(0)), "Back".into(), |app, _| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                        add_movie_popup.input0 = TextArea::from([""]);
                        add_movie_popup.phase = Phase::SelectMovie;
                    }
                });
                if rating_valid {
                    key_event_handler.bind_enter((None, Some(1)), "".into(), |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 2;
                        }
                    });
                    if date_valid {
                        key_event_handler.bind_enter(
                            (None, Some(2)),
                            "Confirm".into(),
                            |app, _| {
                                if let Some(Popup::AddMovie(add_movie_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    add_movie_popup.advance_phase();
                                    add_movie_popup.throbber_visible = true;
                                }
                            },
                        );
                        key_event_handler.bind_enter(
                            (None, Some(3)),
                            "Confirm".into(),
                            |app, _| {
                                if let Some(Popup::AddMovie(add_movie_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    add_movie_popup.advance_phase();
                                    add_movie_popup.throbber_visible = true;
                                }
                            },
                        );
                    }
                }
                key_event_handler.bind_enter((None, Some(4)), "Cancel".into(), |app, _| {
                    app.drawer.close_popup();
                });

                key_event_handler.bind_input_field((None, Some(1)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            let parsed = add_movie_popup.input0.lines()[0]
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            if let KeyCode::Char(x) = &key_event.code {
                                if add_movie_popup.input0.lines()[0].len() >= 3 || parsed >= 10.0 {
                                    return;
                                }

                                if !x.is_ascii_digit() && *x != '.' {
                                    return;
                                }
                            }

                            add_movie_popup.input0.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(2)), "".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            add_movie_popup.input1.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(12, 44, frame.area()),
                    " Add movie ",
                    true,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                for i in 0..popup_area.width {
                    for j in 0..popup_area.height {
                        frame
                            .buffer_mut()
                            .cell_mut(Position::new(popup_area.x + i, popup_area.y + j))
                            .unwrap()
                            .set_diff_option(ratatui::buffer::CellDiffOption::AlwaysUpdate);
                    }
                }
                let [_, rating_input_area, date_input_area, _] = vertical![==1, ==3, ==3, >=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                let mouse_area = widgets::action(
                    Action::new(" Back ", ActionType::Normal, self.item == 0, true),
                    HorizontalAlignment::Left,
                    false,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 0;
                            add_movie_popup.phase = Phase::SelectMovie;
                            add_movie_popup.input0 = TextArea::from([""]);
                        }
                    },
                );

                let actions_mouse_areas = widgets::actions(
                    [
                        Action::new(
                            " Confirm ",
                            ActionType::Default,
                            self.item == 3,
                            rating_valid && date_valid,
                        ),
                        Action::new(" Cancel ", ActionType::Critical, self.item == 4, true),
                    ],
                    HorizontalAlignment::Right,
                    true,
                    1,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        move |app, _| {
                            if let Some(Popup::AddMovie(add_movie_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if i == 0 {
                                    if rating_valid && date_valid {
                                        add_movie_popup.advance_phase();
                                        add_movie_popup.throbber_visible = true;
                                    }
                                } else {
                                    app.drawer.close_popup();
                                }
                            }
                        },
                    );
                }

                widgets::input_field(
                    true,
                    self.item == 1,
                    rating_valid,
                    &mut self.input0,
                    WrapMode::None,
                    frame,
                    rating_input_area,
                    " Rating ",
                    "Enter a rating",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    rating_input_area,
                    |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 1;
                        }
                    },
                );

                widgets::input_field(
                    true,
                    self.item == 2,
                    date_valid,
                    &mut self.input1,
                    WrapMode::None,
                    frame,
                    date_input_area,
                    " Watched At ",
                    "Now",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    date_input_area,
                    |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 2;
                        }
                    },
                );
            }
            Phase::GettingDetails | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(10, 50, frame.area()),
                    if self.refetch_details {
                        " Refetch details "
                    } else {
                        " Add movie "
                    },
                    false,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, throbber_area, _] = vertical![>=1, ==1, >=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(line!("Getting details").centered(), message_area);

                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                        .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                    throbber_area.centered(constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::ConfirmRefetchDetails(_) => {
                key_event_handler.bind_tab((None, None), "Navigate".into(), |app, _| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = (add_movie_popup.item == 0) as usize;
                    }
                });

                key_event_handler.bind_esc((None, None), "Cancel".into(), |app, _| {
                    app.drawer.close_popup();
                });

                key_event_handler.bind_enter((None, Some(0)), "Confirm".into(), |app, _| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.advance_phase();
                    }
                });
                key_event_handler.bind_enter((None, Some(1)), "Cancel".into(), |app, _| {
                    app.drawer.close_popup();
                });

                key_event_handler.bind_horizontal((None, None), "Navigate".into(), |app, data| {
                    if let Some(Popup::AddMovie(add_movie_popup)) = app.drawer.active_popup.as_mut()
                    {
                        if let crate::key_event_handler::Data::Direction(dir, _) = data {
                            add_movie_popup.item = dir as usize;
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(8, 40, frame.area()),
                    " Refetch details ",
                    true,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area] = vertical![>=3]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(
                    Text::from_iter(helpers::wrap_text(
                        "Refetch movie details?",
                        message_area.width as usize,
                    )),
                    message_area,
                );

                let actions_mouse_areas = widgets::actions(
                    [
                        Action::new(" Confirm ", ActionType::Normal, self.item == 0, true),
                        Action::new(" Cancel ", ActionType::Critical, self.item == 1, true),
                    ],
                    HorizontalAlignment::Right,
                    true,
                    1,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        move |app, _| {
                            if i == 0 {
                                if let Some(Popup::AddMovie(add_movie_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    add_movie_popup.advance_phase();
                                }
                            } else {
                                app.drawer.close_popup();
                            }
                        },
                    );
                }
            }
            Phase::Error(error) => {
                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(11, 44, frame.area()),
                    " Error ",
                    true,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, _] = vertical![>=1, ==1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(
                    Text::from_iter(helpers::wrap_text(
                        &format!("{error:#?}"),
                        message_area.width as usize,
                    ))
                    .centered(),
                    message_area,
                );

                if self.refetch_details {
                    let mouse_area = widgets::action(
                        Action::new(" Cancel ", ActionType::Normal, self.item == 0, true),
                        HorizontalAlignment::Center,
                        true,
                        helpers::add_padding(popup_area, Padding::right(1)),
                        frame,
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        |app, _| {
                            app.drawer.close_popup();
                        },
                    );
                    key_event_handler.bind_enter((None, None), "Cancel".into(), |app, _| {
                        app.drawer.close_popup();
                    });
                } else {
                    key_event_handler.bind_tab((None, None), "Navigate".into(), |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = (add_movie_popup.item == 0) as usize;
                        }
                    });
                    key_event_handler.bind_horizontal(
                        (None, None),
                        "Navigate".into(),
                        |app, data| {
                            if let Some(Popup::AddMovie(add_movie_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let crate::key_event_handler::Data::Direction(dir, _) = data {
                                    add_movie_popup.item = dir as usize;
                                }
                            }
                        },
                    );

                    key_event_handler.bind_enter((None, Some(0)), "Back".into(), |app, _| {
                        if let Some(Popup::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.item = 0;
                            add_movie_popup.phase = Phase::SelectMovie;
                            add_movie_popup.input0 = TextArea::from([""]);
                        }
                    });
                    key_event_handler.bind_enter((None, Some(1)), "Cancel".into(), |app, _| {
                        app.drawer.close_popup();
                    });

                    let actions_mouse_areas = widgets::actions(
                        [
                            Action::new(" Back ", ActionType::Default, self.item == 0, true),
                            Action::new(" Cancel ", ActionType::Critical, self.item == 1, true),
                        ],
                        HorizontalAlignment::Center,
                        true,
                        1,
                        helpers::add_padding(popup_area, Padding::right(1)),
                        frame,
                    );
                    for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if i == 0 {
                                    if let Some(Popup::AddMovie(add_movie_popup)) =
                                        app.drawer.active_popup.as_mut()
                                    {
                                        add_movie_popup.item = 0;
                                        add_movie_popup.phase = Phase::SelectMovie;
                                        add_movie_popup.input0 = TextArea::from([""]);
                                    }
                                } else {
                                    app.drawer.close_popup();
                                }
                            },
                        );
                    }
                }
            }
        }
    }
}
