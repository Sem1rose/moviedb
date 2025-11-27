use crate::{
    app::App,
    custom::helpers::{center_rect, ellipsize_string},
    draw::Drawer,
    omdb::{self, OMDBDetailsResponse},
    tmdb::{self, TMDBDetailsResponse, TMDBSearchResponse, TMDBSearchResult},
    trakt::{self, TraktDetailsResponse},
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, vertical};
use std::{
    sync::mpsc::{self, Receiver},
    thread,
};
use style::palette::tailwind;
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Default)]
pub enum Phase {
    #[default]
    GetName,
    Searching,
    SelectMovie,
    GetRating,
    GettingDetails,
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

    pub search_rating_input: Input,

    pub rx_search_result: Option<Receiver<anyhow::Result<TMDBSearchResponse>>>,
    pub search_results: Option<Vec<TMDBSearchResult>>,
    pub num_results: usize,

    pub scroll_pos: usize,
    pub selected: usize,

    pub user_rating: f64,

    pub rx_details_response: Option<Receiver<anyhow::Result<DetailsResponse>>>,
    pub tmdb_movie_details_result: Option<TMDBDetailsResponse>,
    pub trakt_movie_details_result: Option<TraktDetailsResponse>,
    pub omdb_movie_details_result: Option<OMDBDetailsResponse>,
}

const NUMVISMOVIES: usize = 5;
impl AddMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn current_movie_index(&self) -> usize {
        self.scroll_pos + self.selected
    }

    pub fn inc_movie_selection(&mut self) {
        if self.num_results == 0 {
            return;
        }

        if self.current_movie_index() < self.num_results - 1 {
            if self.selected < NUMVISMOVIES - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }
        }
    }

    pub fn dec_movie_selection(&mut self) {
        if self.num_results == 0 {
            return;
        }

        if self.selected > 0 {
            self.selected -= 1;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;
        }
    }

    pub fn request_search(&mut self, app: &App) {
        let (tx_search_results, rx_search_results) = mpsc::channel();
        let tmdb_conf_cloned = app.tmdb_config.clone();
        let search_string = self.search_rating_input.value().to_string();

        thread::spawn(move || {
            _ = tx_search_results.send(tmdb::find_movie(&tmdb_conf_cloned, &search_string));
        });

        self.rx_search_result = Some(rx_search_results);
    }

    pub fn request_details(&mut self, app: &App) {
        let (tx_details_request, rx_details_request) = mpsc::channel();

        let tmdb_conf_cloned = app.tmdb_config.clone();
        let trakt_conf_cloned: crate::config::config_trakt::TraktConfig = app.trakt_config.clone();
        let omdb_conf_cloned = app.omdb_config.clone();
        let movie_id = self.search_results.as_ref().unwrap()[self.current_movie_index()].id;

        thread::spawn(move || {
            let tmdb_result = tmdb::get_movie_details(&tmdb_conf_cloned, movie_id);

            if let Ok(tmdb_response) = tmdb_result {
                let trakt_result =
                    trakt::get_movie_details(&trakt_conf_cloned, &tmdb_response.imdb_id);
                let omdb_result =
                    omdb::get_movie_details(&omdb_conf_cloned, &tmdb_response.imdb_id);

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

    pub fn advance_phase(&mut self, app: &App) {
        self.phase = match self.phase {
            Phase::GetName => {
                self.request_search(app);
                Phase::Searching
            }
            Phase::Searching => Phase::SelectMovie,
            Phase::SelectMovie => {
                self.search_rating_input.reset();
                Phase::GetRating
            }
            Phase::GetRating => {
                self.user_rating = format!(
                    "{:.1}",
                    self.search_rating_input.value().parse::<f64>().unwrap()
                )
                .parse()
                .unwrap();

                self.request_details(app);

                Phase::GettingDetails
            }
            Phase::GettingDetails => Phase::Done,
            _ => Phase::GetName,
        };
    }

    pub fn check_input_rating(&mut self) -> bool {
        if self.search_rating_input.value().is_empty() {
            return false;
        }

        if let Ok(x) = self.search_rating_input.value().parse::<f64>() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn read_channels(&mut self, app: &App) -> anyhow::Result<()> {
        match self.phase {
            Phase::Searching => {
                let result = self.rx_search_result.as_ref().unwrap().try_recv();
                if let Ok(search_result) = result {
                    self.search_results = Some(search_result.unwrap().results);
                    self.num_results = self.search_results.as_ref().unwrap().len();

                    self.advance_phase(app);

                    self.rx_search_result = None;
                } else if let Err(mpsc::TryRecvError::Disconnected) = result {
                    self.rx_search_result = None;
                }
            }
            Phase::GettingDetails => {
                let result = self.rx_details_response.as_ref().unwrap().try_recv();
                if let Ok(details_response) = result {
                    let details_response = details_response?;

                    self.tmdb_movie_details_result = Some(details_response.tmdb);
                    self.trakt_movie_details_result = details_response.trakt;
                    self.omdb_movie_details_result = details_response.omdb;

                    self.advance_phase(app);

                    self.rx_details_response = None;
                } else if let Err(mpsc::TryRecvError::Disconnected) = result {
                    self.rx_details_response = None;
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn handle_key_events(&mut self, app: &App, event: KeyEvent) -> bool {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return false;
        }

        match code {
            KeyCode::Up => {
                if let Phase::SelectMovie = self.phase {
                    self.dec_movie_selection();
                }
            }
            KeyCode::Down => {
                if let Phase::SelectMovie = self.phase {
                    self.inc_movie_selection();
                }
            }
            KeyCode::Enter => match self.phase {
                Phase::GetName => {
                    if !self.search_rating_input.value().is_empty() {
                        self.advance_phase(app);
                    }
                }
                Phase::SelectMovie => {
                    self.advance_phase(app);
                }
                Phase::GetRating => {
                    if self.check_input_rating() {
                        self.advance_phase(app);
                    }
                }
                _ => (),
            },
            KeyCode::Esc => {
                return true;
            }
            _ => match self.phase {
                Phase::GetRating | Phase::GetName => {
                    self.search_rating_input.handle_event(&Event::Key(event));
                }
                _ => (),
            },
        }

        false
    }
}

impl Drawer {
    pub(crate) fn draw_add_movie_popup(&mut self, frame: &mut Frame) -> anyhow::Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(40), Constraint::Max(7));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Add Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1 ,==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match self.add_movie_popup.phase {
            Phase::GetName => {
                let [_, left, right, _] = horizontal![==2, ==6, >=1, ==2].areas(horiz);

                let prompt_area = Layout::vertical([Constraint::Length(1); 5]).split(left)[2];

                let [_, search_top, search_center, search_bottom, _] =
                    Layout::vertical([Constraint::Length(1); 5]).areas(right);

                let [_, search_input_area, _] = horizontal![==1, >=1, ==1].areas(search_center);

                // ▄▀█ ▂🮂▗▖▘▝
                frame.render_widget(
                    Paragraph::new("🮃".repeat(search_bottom.width as usize)).fg(tailwind::RED.c700),
                    search_bottom,
                );
                frame.render_widget(
                    Paragraph::new("▂".repeat(search_top.width as usize)).fg(tailwind::RED.c700),
                    search_top,
                );
                frame.render_widget(Paragraph::new("Name: "), prompt_area);
                frame.render_widget(Block::new().bg(tailwind::RED.c700), search_center);

                let width = search_input_area.width as usize - 1;
                let start = self
                    .add_movie_popup
                    .search_rating_input
                    .visual_scroll(width);
                let cursor_pos = self.add_movie_popup.search_rating_input.cursor() - start;
                let mut chars = self
                    .add_movie_popup
                    .search_rating_input
                    .value()
                    .chars()
                    .skip(start);

                let mut search_string: Vec<Span> = vec![];
                for i in 0..=(start + width) {
                    let c = chars.next().unwrap_or(' ');
                    if i == cursor_pos {
                        search_string.push(c.to_string().reversed());
                    } else {
                        search_string.push(c.to_string().into());
                    }
                }
                frame.render_widget(Line::from_iter(search_string), search_input_area);
            }
            Phase::Searching => {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                let [_, throbber_area, text_area, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(areas[2]);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);
                frame.render_widget(Paragraph::new(" Searching for movie..."), text_area);
            }
            Phase::SelectMovie => {
                let results = self.add_movie_popup.search_results.as_ref().unwrap();

                if results.is_empty() {
                    self.open_error_popup("Couldn't find movie!".into());
                    return Ok(());
                }

                let areas =
                    Layout::vertical(vec![Constraint::Length(1); NUMVISMOVIES]).split(horiz);

                for (i, area) in areas.iter().enumerate() {
                    if i >= results.len() {
                        break;
                    }
                    let movie = &results[self.add_movie_popup.scroll_pos + i];

                    let title_width = (area.width - 20) as usize;

                    let name = ellipsize_string(&movie.title, title_width);

                    let text = format!(
                        "{}{name} - {} - {:.1}",
                        if i == self.add_movie_popup.selected {
                            ">"
                        } else {
                            " "
                        },
                        movie.release_date,
                        movie.vote_average
                    );

                    frame.render_widget(Paragraph::new(text), *area);
                }
            }
            Phase::GetRating => {
                let [_, left, right, _] = horizontal![==2, ==8, >=1, ==2].areas(horiz);

                let prompt_area = Layout::vertical([Constraint::Length(1); 5]).split(left)[2];

                let [_, search_top, search_center, search_bottom, _] =
                    Layout::vertical([Constraint::Length(1); 5]).areas(right);

                let [_, search_input_area, _] = horizontal![==1, >=1, ==1].areas(search_center);

                // ▄▀█ ▂🮂▗▖▘▝
                frame.render_widget(
                    Paragraph::new("🮂".repeat(search_bottom.width as usize)).fg(tailwind::RED.c700),
                    search_bottom,
                );
                frame.render_widget(
                    Paragraph::new("▂".repeat(search_top.width as usize)).fg(tailwind::RED.c700),
                    search_top,
                );
                frame.render_widget(Paragraph::new("Rating: "), prompt_area);
                frame.render_widget(Block::new().bg(tailwind::RED.c700), search_center);

                let width = search_input_area.width as usize - 1;
                let start = self
                    .add_movie_popup
                    .search_rating_input
                    .visual_scroll(width);
                let cursor_pos = self.add_movie_popup.search_rating_input.cursor() - start;
                let mut chars = self
                    .add_movie_popup
                    .search_rating_input
                    .value()
                    .chars()
                    .skip(start);

                let mut search_string: Vec<Span> = vec![];
                for i in 0..=(start + width) {
                    let c = chars.next().unwrap_or(' ');
                    if i == cursor_pos {
                        search_string.push(c.to_string().reversed());
                    } else {
                        search_string.push(c.to_string().into());
                    }
                }
                frame.render_widget(Line::from_iter(search_string), search_input_area);

                if !self.add_movie_popup.check_input_rating() {
                    let error_area = Layout::vertical([Constraint::Length(1); 5]).split(horiz);

                    frame.render_widget(
                        Paragraph::new("Please enter a valid rating!")
                            .red()
                            .centered(),
                        error_area[4],
                    );
                }
            }
            _ => {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                let [_, throbber_area, text_area, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(areas[2]);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);
                frame.render_widget(Paragraph::new(" Processing..."), text_area);
            }
        }

        Ok(())
    }
}
