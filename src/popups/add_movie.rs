use crate::{
    helpers::{add_padding, center_rect, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    omdb::{self, OMDBDetailsResponse},
    popups::Popups,
    tmdb::{self, TMDBDetailsResponse, TMDBSearchResponse, TMDBSearchResult},
    tokens::{OMDBTokens, TMDBTokens, TraktTokens},
    trakt::{self, TraktDetailsResponse},
};
use ratatui::{
    layout::*,
    macros::{constraint, horizontal, vertical},
    prelude::*,
    style::palette::material,
    symbols::{block, scrollbar::Set},
    widgets::*,
    Frame,
};
use std::{
    ops::Add,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};
use style::palette::tailwind;
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_textarea::TextArea;

#[derive(Default)]
pub enum Phase {
    // GetName,
    // Searching,
    #[default]
    SelectMovie,
    GetRating,
    GettingDetails,
    Error(String),
    Done,
}

pub struct DetailsResponse {
    pub trakt: Option<TraktDetailsResponse>,
    pub tmdb: TMDBDetailsResponse,
    pub omdb: Option<OMDBDetailsResponse>,
}

#[derive(Default)]
pub struct AddMoviePopup {
    pub phase: Phase,
    pub throbber_visible: bool,
    pub tick: u64,

    item: usize,
    tab: usize,

    input: TextArea<'static>,
    throbber_state: ThrobberState,

    rx_search_result: Option<Receiver<(u64, anyhow::Result<TMDBSearchResponse>)>>,
    search_ticket: u64,
    pub search_results: Option<Vec<TMDBSearchResult>>,
    last_input_tick: Option<u64>,

    scroll_pos: usize,
    selected_item: usize,
    num_visible_items: usize,
    alignment_bottom: bool,

    pub user_rating: f64,

    rx_details_response: Option<Receiver<anyhow::Result<DetailsResponse>>>,
    pub tmdb_movie_details_result: Option<TMDBDetailsResponse>,
    pub trakt_movie_details_result: Option<TraktDetailsResponse>,
    pub omdb_movie_details_result: Option<OMDBDetailsResponse>,

    trakt_tokens: TraktTokens,
    tmdb_tokens: TMDBTokens,
    omdb_tokens: OMDBTokens,

    cache_dir: PathBuf,
}

impl AddMoviePopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

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

    pub fn request_search(&mut self) {
        let (tx_search_results, rx_search_results) = mpsc::channel();

        let search_string = self.input.lines()[0].clone();
        let access_token = self.tmdb_tokens.access_token_owned();
        let ticket = rand::random();
        self.search_ticket = ticket;

        thread::spawn(move || {
            _ = tx_search_results.send((ticket, tmdb::find_movie(&access_token, &search_string)));
        });

        self.rx_search_result = Some(rx_search_results);
    }

    pub fn request_details(&mut self) {
        let (tx_details_request, rx_details_request) = mpsc::channel();

        let tmdb_access_token = self.tmdb_tokens.access_token_owned();
        let trakt_client_id = self.trakt_tokens.client_id_owned();
        let omdb_api_key = self.omdb_tokens.key_owned();
        let movie_id = self.search_results.as_ref().unwrap()[self.selected_item].id;
        let cache_dir = self.cache_dir.clone();
        let access_token = self.tmdb_tokens.access_token_owned();
        let client_id = self.trakt_tokens.client_id_owned();

        thread::spawn(move || {
            let tmdb_result = tmdb::get_movie_details(&tmdb_access_token, movie_id);

            if let Ok(tmdb_response) = tmdb_result {
                let trakt_result =
                    trakt::get_movie_details(&trakt_client_id, &tmdb_response.imdb_id);
                let omdb_result = omdb::get_movie_details(&omdb_api_key, &tmdb_response.imdb_id);

                let result = tmdb::get_movie_poster_banner(
                    &cache_dir,
                    &access_token,
                    tmdb_response.id,
                    true,
                );
                if let Ok(true) = result {
                } else {
                    _ = trakt::get_movie_poster_banner(
                        &cache_dir,
                        &client_id,
                        tmdb_response.imdb_id.clone(),
                        true,
                    );
                }

                _ = tx_details_request.send(Ok(DetailsResponse {
                    trakt: trakt_result.map(Some).unwrap_or(None),
                    tmdb: tmdb_response,
                    omdb: omdb_result.map(Some).unwrap_or(None),
                }));
            } else if let Err(error) = tmdb_result {
                _ = tx_details_request.send(Err(error));
            }
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
            if self.tick - last_tick > 20 {
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

                        self.search_results = Some(search_result.unwrap().results);
                    }
                }
            }
            Phase::GettingDetails => {
                let result = self.rx_details_response.as_ref().unwrap().try_recv();
                if let Ok(details_response) = result {
                    if let Ok(details_response) = details_response {
                        self.tmdb_movie_details_result = Some(details_response.tmdb);
                        self.trakt_movie_details_result = details_response.trakt;
                        self.omdb_movie_details_result = details_response.omdb;

                        self.advance_phase();

                        self.rx_details_response = None;
                    } else if let Err(error) = details_response {
                        self.rx_details_response = None;
                        self.phase = Phase::Error(format!("{error}"));
                    }
                } else if let Err(mpsc::TryRecvError::Disconnected) = result {
                    self.rx_details_response = None;
                    self.phase = Phase::Error("Error while fetching movie details".into());
                }
            }
            _ => (),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
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
                    (Some(0), Some(1)),
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
                key_event_handler.bind_enter((Some(0), Some(0)), "Search".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 1;
                    }
                });
                key_event_handler.bind_enter((Some(0), Some(1)), "Select".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.advance_phase();
                    }
                });
                key_event_handler.bind_tab((Some(0), None), "Navigate".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = add_movie_popup.item.checked_sub(1).unwrap_or(1);
                    }
                });
                key_event_handler.bind_esc((Some(0), Some(1)), "Return".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.item = 0;
                    }
                });
                key_event_handler.bind_input_field((Some(0), Some(0)), "".into(), |app, data| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Key(key_event) => {
                                let x = add_movie_popup.input.lines()[0].clone();
                                add_movie_popup.input.input(key_event);
                                if add_movie_popup.input.lines()[0].trim() != x.trim()
                                    && !add_movie_popup.input.is_empty()
                                {
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

                if num_results == 0 {
                    self.item = 0;
                }

                let popup_area = dynamic_popup(
                    frame,
                    Some(24),
                    2.4,
                    tailwind::BLUE.c950,
                    "  Add movie  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                let [search_input_area, horiz] = vertical![==4, >=1].areas(popup_area);
                let [projects_list_area, scrollbar_area] = horizontal![>=1, ==1].areas(horiz);

                let input_selected = self.item == 0;
                self.input.set_style(Style::new().fg(if input_selected {
                    tailwind::SLATE.c200
                } else {
                    tailwind::STONE.c400
                }));
                self.input.set_cursor_style(
                    Style::new()
                        .fg(if input_selected {
                            tailwind::SLATE.c300
                        } else {
                            tailwind::STONE.c400
                        })
                        .add_modifier(if input_selected {
                            Modifier::REVERSED
                        } else {
                            Modifier::default()
                        }),
                );
                self.input.set_block(
                    Block::bordered()
                        .border_type(ratatui::widgets::BorderType::Thick)
                        .style(Style::new().fg(if input_selected {
                            material::BLUE.c500
                        } else {
                            tailwind::STONE.c600
                        }))
                        .title(" Name ")
                        .title_style(Style::new().fg(if input_selected {
                            material::BLUE.c400
                        } else {
                            // if valid {
                            material::BLUE.c600
                            // } else {
                            //     material::RED.c600
                            // }
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input.set_placeholder_text("Search");
                self.input
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
                frame.render_widget(
                    &self.input,
                    add_padding(
                        search_input_area,
                        Padding {
                            left: 1,
                            right: 1,
                            top: 1,
                            bottom: 0,
                        },
                    ),
                );

                let num_visible_projects = projects_list_area.height as usize / 5;
                let partially_visible_project_height =
                    projects_list_area.height as usize - num_visible_projects * 5;
                let render_partially_visible_project = partially_visible_project_height > 0;
                self.num_visible_items = num_visible_projects
                    + if render_partially_visible_project {
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

                if num_results <= num_visible_projects {
                    self.alignment_bottom = false;
                } else if self.selected_item - self.scroll_pos == 0 {
                    self.alignment_bottom = false;
                } else if self.selected_item - self.scroll_pos == self.num_visible_items - 1 {
                    self.alignment_bottom = true;
                }

                let mut remaining_area = add_padding(
                    projects_list_area,
                    Padding {
                        left: 1,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    },
                );
                for i in 0..self.num_visible_items {
                    let [area, remaining] =
                        if render_partially_visible_project && i == 0 && self.alignment_bottom {
                            Layout::vertical([
                                Constraint::Length(partially_visible_project_height as u16),
                                Constraint::Min(0),
                            ])
                        } else if render_partially_visible_project
                            && i == self.num_visible_items - 1
                            && !self.alignment_bottom
                        {
                            Layout::vertical([
                                Constraint::Length(partially_visible_project_height as u16),
                                Constraint::Min(0),
                            ])
                        } else {
                            Layout::vertical([Constraint::Length(5), Constraint::Min(0)])
                        }
                        .areas(remaining_area);

                    if self.scroll_pos + i < num_results {
                        let result = &self.search_results.as_ref().unwrap()[self.scroll_pos + i];
                        let partially_visible = area.height < 5;

                        let alternate = i & 1 == 1;
                        let selected = self.selected_item == i + self.scroll_pos;

                        frame.render_widget(
                            Block::new().bg(if selected {
                                if !input_selected {
                                    tailwind::TEAL.c600
                                } else {
                                    tailwind::TEAL.c900
                                }
                            } else if !alternate {
                                tailwind::GRAY.c600
                            } else {
                                tailwind::SLATE.c700
                            }),
                            area,
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
                                            if !input_selected {
                                                tailwind::EMERALD.c700
                                            } else {
                                                tailwind::EMERALD.c800
                                            }
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
                                    Line::from(vec![
                                        Span::from(&result.title).style(
                                            Style::new()
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
                                        ),
                                        Span::from("  "),
                                        Span::from(&result.release_date).style(
                                            Style::new()
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
                                        ),
                                    ])
                                    .left_aligned(),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            } else if index == 3 {
                                frame.render_widget(
                                    Line::from(vec![Span::from(format!(
                                        "{:.1}",
                                        result.vote_average
                                    ))
                                    .style(
                                        Style::new()
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
                                    )])
                                    .left_aligned(),
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
                key_event_handler.bind_tab((Some(1), None), "".into(), |app, data| {
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
                key_event_handler.bind_horizontal((Some(1), Some(2)), "".into(), |app, data| {
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
                key_event_handler.bind_horizontal((Some(1), Some(3)), "".into(), |app, data| {
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
                key_event_handler.bind_enter((Some(1), Some(3)), "Cancel".into(), |app, _| {
                    app.drawer.close_popups();
                });
                key_event_handler.bind_enter((Some(1), Some(0)), "Back".into(), |app, _| {
                    if let Some(Popups::AddMovie(add_movie_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        add_movie_popup.phase = Phase::SelectMovie;
                        add_movie_popup.item = 0;
                        add_movie_popup.input = TextArea::from([""]);
                    }
                });
                if valid {
                    key_event_handler.bind_enter((Some(1), None), "Confirm".into(), |app, _| {
                        if let Some(Popups::AddMovie(add_movie_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            add_movie_popup.advance_phase();
                            add_movie_popup.throbber_visible = true;
                        }
                    });
                }
                key_event_handler.bind_input_field((Some(1), Some(1)), "".into(), |app, data| {
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
                key_event_handler.bind_esc((Some(1), Some(0)), "Close".into(), |app, _| {
                    app.drawer.close_popups();
                });
                key_event_handler.bind_esc((Some(1), None), "Back".into(), |app, _| {
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

                let [back_area, _, input_area, _, actions_area, _] =
                    vertical![==1, ==1, ==3, >=1, ==1, ==1].areas(popup_area);

                frame.render_widget(
                    Span::from(" Back ").style(
                        Style::new()
                            .fg(if self.item == 0 {
                                tailwind::SLATE.c300
                            } else {
                                tailwind::BLUE.c500
                            })
                            .bg(if self.item == 0 {
                                material::BLUE.c800
                            } else {
                                tailwind::SLATE.c950
                            }),
                    ),
                    back_area,
                );

                frame.render_widget(
                    Line::from(vec![
                        Span::from(" Confirm ").style(
                            Style::new()
                                .fg(if valid {
                                    if self.item == 2 {
                                        tailwind::SLATE.c200
                                    } else {
                                        tailwind::SLATE.c300
                                    }
                                } else {
                                    tailwind::SLATE.c500
                                })
                                .bg(if valid {
                                    if self.item == 2 {
                                        material::BLUE.c600
                                    } else {
                                        material::BLUE.c900
                                    }
                                } else {
                                    if self.item == 2 {
                                        tailwind::SLATE.c700
                                    } else {
                                        tailwind::SLATE.c800
                                    }
                                }),
                        ),
                        Span::from(" "),
                        Span::from(" Cancel ").style(
                            Style::new()
                                .fg(if self.item == 3 {
                                    tailwind::SLATE.c300
                                } else {
                                    tailwind::RED.c500
                                })
                                .bg(if self.item == 3 {
                                    material::RED.c800
                                } else {
                                    tailwind::SLATE.c950
                                }),
                        ),
                        Span::from("  "),
                    ])
                    .right_aligned(),
                    actions_area,
                );

                let input_selected = self.item == 1;
                self.input.set_style(Style::new().fg(if input_selected {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                }));
                self.input.set_cursor_style(
                    Style::new()
                        .fg(if input_selected {
                            tailwind::SLATE.c300
                        } else {
                            tailwind::STONE.c400
                        })
                        .add_modifier(if input_selected {
                            Modifier::REVERSED
                        } else {
                            Modifier::default()
                        }),
                );
                self.input.set_block(
                    Block::bordered()
                        .border_type(ratatui::widgets::BorderType::Thick)
                        .style(Style::new().fg(if input_selected {
                            if valid {
                                material::BLUE.c500
                            } else {
                                material::RED.c600
                            }
                        } else {
                            tailwind::STONE.c500
                        }))
                        .title(" Rating ")
                        .title_style(Style::new().fg(if input_selected {
                            material::BLUE.c400
                        } else {
                            if valid {
                                material::BLUE.c600
                            } else {
                                material::RED.c600
                            }
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input.set_placeholder_text("Enter a rating");
                self.input
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
                frame.render_widget(
                    &self.input,
                    add_padding(
                        input_area,
                        Padding {
                            left: 2,
                            right: 2,
                            top: 0,
                            bottom: 0,
                        },
                    ),
                );
            }
            Phase::GettingDetails | Phase::Done => {
                self.throbber_visible = true;

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

                let [_, message_area, throbber_area, _] =
                    vertical![==3, ==2, ==1, >=1].areas(popup_area);
                frame.render_widget(Paragraph::new("Getting details").centered(), message_area);

                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                        .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                    center_rect(throbber_area, constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::Error(error) => {
                self.tab = 2;
                key_event_handler.bind_enter((Some(2), None), "Close".into(), |app, _| {
                    app.drawer.close_popups();
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

                let [_, message_area, _, actions_area, _] =
                    vertical![>=1, ==1, >=1, ==1, ==1].areas(popup_area);
                frame.render_widget(Paragraph::new(error.as_str()).centered(), message_area);

                frame.render_widget(
                    Line::from(vec![
                        Span::from(" Ok ").style(
                            Style::new()
                                .fg(tailwind::SLATE.c200)
                                .bg(material::BLUE.c600),
                        ),
                        Span::from("  "),
                    ])
                    .right_aligned(),
                    actions_area,
                );
            }
        }
    }
}
