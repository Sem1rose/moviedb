use crate::{
    app::App,
    custom::helpers::{center_rect, ellipsize_string},
    draw::Drawer,
    omdb::{self, OMDBDetailsResponse},
    tmdb::{self, TMDBDetailsResponse, TMDBSearchResponse, TMDBSearchResult},
    trakt::{self, TraktDetailsResponse},
    types::*,
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
enum Phase {
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
    phase: Phase,

    pub input: Input,

    pub rx_search_result: Option<Receiver<Result<TMDBSearchResponse>>>,
    pub search_result: Option<TMDBSearchResponse>,

    pub scroll_pos: usize,
    pub selected: usize,

    pub user_rating: f64,

    pub rx_details_response: Option<Receiver<Result<DetailsResponse>>>,
    pub tmdb_movie_details_result: Option<TMDBDetailsResponse>,
    pub trakt_movie_details_result: Option<TraktDetailsResponse>,
    pub omdb_movie_details_result: Option<OMDBDetailsResponse>,
}

impl AddMoviePopup {
    const NUMVISMOVIES: usize = 5;
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn current_movie_index(&self) -> usize {
        self.scroll_pos + self.selected
    }

    pub fn inc_movie_selection(&mut self) {
        let search_result = self.try_get_search_result();
        if self.try_get_search_result().is_none() || search_result.unwrap().is_empty() {
            return;
        }

        if self.current_movie_index() < search_result.unwrap().len() - 1 {
            if self.selected < AddMoviePopup::NUMVISMOVIES - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }
        }
    }

    pub fn dec_movie_selection(&mut self) {
        let search_result = self.try_get_search_result();
        if self.try_get_search_result().is_none() || search_result.unwrap().is_empty() {
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
        let search_string = self.input.value().to_string();

        thread::spawn(move || {
            let _ = tx_search_results.send(tmdb::find_movie(&tmdb_conf_cloned, &search_string));
        });

        self.rx_search_result = Some(rx_search_results);
    }

    pub fn try_get_search_result(&self) -> Option<&Vec<TMDBSearchResult>> {
        Some(&self.search_result.as_ref()?.results)
    }

    pub fn request_details(&mut self, app: &App) {
        let (tx_details_request, rx_details_request) = mpsc::channel();

        let tmdb_conf_cloned = app.tmdb_config.clone();
        let trakt_conf_cloned = app.trakt_config.clone();
        let omdb_conf_cloned = app.omdb_config.clone();
        let movie_id = self.try_get_search_result().unwrap()[self.current_movie_index()].id;

        thread::spawn(move || {
            let tmdb_result = tmdb::get_movie_details(&tmdb_conf_cloned, movie_id);

            if let Ok(tmdb_response) = tmdb_result {
                let trakt_result =
                    trakt::get_movie_details(&trakt_conf_cloned, &tmdb_response.imdb_id);
                let omdb_result =
                    omdb::get_movie_details(&omdb_conf_cloned, &tmdb_response.imdb_id);

                let _ = tx_details_request.send(Ok(DetailsResponse {
                    trakt: trakt_result.map(Some).unwrap_or(None),
                    tmdb: tmdb_response,
                    omdb: omdb_result.map(Some).unwrap_or(None),
                }));
            } else if let Err(error) = tmdb_result {
                let _ = tx_details_request.send(Err(error));
            }
        });

        self.rx_details_response = Some(rx_details_request);
    }

    pub fn advance_phase(&mut self, app: Option<&App>) {
        self.phase = match self.phase {
            Phase::GetName => {
                self.request_search(app.unwrap());
                Phase::Searching
            }
            Phase::Searching => Phase::SelectMovie,
            Phase::SelectMovie => {
                self.input.reset();
                Phase::GetRating
            }
            Phase::GetRating => {
                self.user_rating = format!("{:.1}", self.input.value().parse::<f64>().unwrap())
                    .parse()
                    .unwrap();

                self.request_details(app.unwrap());

                Phase::GettingDetails
            }
            Phase::GettingDetails => Phase::Done,
            _ => Phase::GetName,
        };
    }

    pub fn check_input_rating(&mut self) -> bool {
        if self.input.value() == "" {
            return false;
        }

        let input_parsed = self.input.value().parse::<f64>();
        input_parsed.is_ok() && input_parsed.unwrap() <= 10.0
    }

    pub fn read_channels(&mut self) -> std::result::Result<(), String> {
        match self.phase {
            Phase::Searching => {
                let result = self.rx_search_result.as_ref().unwrap().try_recv();
                if let Ok(search_result) = result {
                    if let Ok(search_response) = search_result {
                        self.search_result = Some(search_response);
                        self.advance_phase(None);
                    } else if let Err(error) = search_result {
                        return Err("Error while searching for movie!".into());
                    }
                } else if let Err(mpsc::TryRecvError::Disconnected) = result {
                    self.rx_search_result = None;
                }
            }
            Phase::GettingDetails => {
                let result = self.rx_details_response.as_ref().unwrap().try_recv();
                if let Ok(details_response) = result {
                    if let Ok(search_response) = details_response {
                        let details_response = search_response;

                        self.tmdb_movie_details_result = Some(details_response.tmdb);
                        self.trakt_movie_details_result = details_response.trakt;
                        self.omdb_movie_details_result = details_response.omdb;

                        self.advance_phase(None);
                    } else if let Err(error) = details_response {
                        return Err("Error while getting movie details!".into());
                    }
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
                    if self.input.value() != "" {
                        self.advance_phase(Some(app));
                    }
                }
                Phase::SelectMovie => {
                    self.advance_phase(Some(app));
                }
                Phase::GetRating => {
                    if self.check_input_rating() {
                        self.advance_phase(Some(app));
                    }
                }
                _ => (),
            },
            KeyCode::Esc => {
                return true;
            }
            _ => match self.phase {
                Phase::GetRating | Phase::GetName => {
                    self.input.handle_event(&Event::Key(event));
                }
                _ => (),
            },
        }

        false
    }
}

impl Drawer {
    pub(crate) fn draw_add_movie_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        if let Err(error) = self.add_movie_popup.read_channels() {
            self.open_error_popup(error);

            return Ok(());
        }

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
                let start = self.add_movie_popup.input.visual_scroll(width);
                let cursor_pos = self.add_movie_popup.input.cursor() - start;
                let mut chars = self.add_movie_popup.input.value().chars().skip(start);

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
                let results = self.add_movie_popup.try_get_search_result().unwrap();

                if results.is_empty() {
                    self.open_error_popup("Couldn't find movie!".into());
                    return Ok(());
                }

                let areas =
                    Layout::vertical(vec![Constraint::Length(1); AddMoviePopup::NUMVISMOVIES])
                        .split(horiz);

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
                let start = self.add_movie_popup.input.visual_scroll(width);
                let cursor_pos = self.add_movie_popup.input.cursor() - start;
                let mut chars = self.add_movie_popup.input.value().chars().skip(start);

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
            Phase::GettingDetails => {
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
                frame.render_widget(Paragraph::new(" Getting movie details..."), text_area);
            }
            Phase::Done => {
                let tmdb_movie_details = self
                    .add_movie_popup
                    .tmdb_movie_details_result
                    .take()
                    .unwrap();
                let trakt_movie_details = self.add_movie_popup.trakt_movie_details_result.take();
                let omdb_movie_details = self.add_movie_popup.omdb_movie_details_result.take();

                let mut movie = Movie::from(tmdb_movie_details, self.add_movie_popup.user_rating);
                if let Some(trakt) = trakt_movie_details {
                    movie.add_trakt_details(trakt);
                }
                if let Some(omdb) = omdb_movie_details {
                    movie.add_omdb_details(omdb);
                }
                app.movies.push(movie);

                self.main_screen.filter_sort_movies(app);

                if app.save_movies().is_err() {
                    self.open_error_popup("Couldn't save new rating!".into());
                } else {
                    self.open_fetch_artworks_popup(app)?;

                    self.main_screen.movies_list.selected = self
                        .main_screen
                        .movies_list
                        .num_visible_movies
                        .min(self.main_screen.filtered_movies.len())
                        - 1;

                    self.main_screen.movies_list.scroll_pos =
                        self.main_screen.filtered_movies.len()
                            - self.main_screen.movies_list.selected
                            - 1;

                    // self.image_backend
                    //     .reload_images(app, self.main_screen.movies_list.selected);
                }
            }
        }

        Ok(())
    }
}
