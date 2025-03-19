use crate::{
    app::{App, Movie, Result},
    draw::Drawer,
    tmdb::{self, TMDBDetailsResponse, TMDBSearchResponse},
    trakt::{self, TraktDetailsResponse},
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use style::palette::tailwind;
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Default)]
pub struct AddMoviePopup {
    pub phase: u32,
    pub failed: Arc<Mutex<bool>>,
    pub finished_search_input: bool,
    pub search_input: Input,
    pub requested_search: bool,
    pub search_result: Arc<Mutex<TMDBSearchResponse>>,
    pub search_finished: Arc<Mutex<bool>>,
    pub movies_visible: u32,
    pub scroll_pos: u32,
    pub selected: u32,
    pub movie_selected: bool,
    pub user_rating_valid: bool,
    pub got_user_rating: bool,
    pub user_rating: f64,
    pub requested_movie_details: bool,
    pub tmdb_movie_details_result: Arc<Mutex<TMDBDetailsResponse>>,
    pub trakt_movie_details_result: Arc<Mutex<TraktDetailsResponse>>,
    pub movie_details_finished: Arc<Mutex<bool>>,
    pub added_movie: bool,
}

impl AddMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    // pub fn handle_key_events(&mut self, drawer: &mut Drawer, event: KeyEvent) -> Result<()> {
    //     if drawer.accepting_input {
    //         self.search_input.handle_event(&Event::Key(event));
    //     } else {
    //         let kind = event.kind;
    //         let code = event.code;

    //         if kind != KeyEventKind::Press {
    //             return Ok(());
    //         }

    //         match code {
    //             KeyCode::Up => {
    //                 if !self.movie_selected {
    //                     self.dec_movie_selection();
    //                 }
    //             }
    //             KeyCode::Down => {
    //                 if !self.movie_selected {
    //                     self.inc_movie_selection();
    //                 }
    //             }
    //             KeyCode::Enter => {
    //                 if *self.failed.lock().unwrap() {
    //                     drawer.close_popups();
    //                 } else if self.phase == 0 && self.search_input.value() != "" {
    //                     self.finished_search_input = true;
    //                 } else if self.phase == 2 {
    //                     self.movie_selected = true;
    //                 } else if self.phase == 3
    //                     && self.search_input.value() != ""
    //                     && self.user_rating_valid
    //                 {
    //                     self.got_user_rating = true;
    //                 }
    //             }
    //             KeyCode::Esc => {
    //                 drawer.close_popups();
    //             }
    //             _ => (),
    //         }
    //     }

    //     Ok(())
    // }

    pub fn inc_movie_selection(&mut self) {
        if self.search_result.lock().unwrap().results.is_empty() {
            return;
        }
        if self.scroll_pos + self.selected
            < self.search_result.lock().unwrap().results.len() as u32 - 1
        {
            if self.selected < self.movies_visible - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }
        }
    }

    pub fn dec_movie_selection(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;
        }
    }
}

impl Drawer {
    pub fn add_movie_popup_handle_key_events(&mut self, event: KeyEvent) -> Result<()> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(());
        }

        match code {
            KeyCode::Up => {
                if !self.add_movie_popup_options.movie_selected {
                    self.add_movie_popup_options.dec_movie_selection();
                }
            }
            KeyCode::Down => {
                if !self.add_movie_popup_options.movie_selected {
                    self.add_movie_popup_options.inc_movie_selection();
                }
            }
            KeyCode::Enter => {
                if *self.add_movie_popup_options.failed.lock().unwrap() {
                    self.close_popups();
                } else if self.add_movie_popup_options.phase == 0
                    && self.add_movie_popup_options.search_input.value() != ""
                {
                    self.add_movie_popup_options.finished_search_input = true;
                } else if self.add_movie_popup_options.phase == 2 {
                    self.add_movie_popup_options.movie_selected = true;
                } else if self.add_movie_popup_options.phase == 3
                    && self.add_movie_popup_options.search_input.value() != ""
                    && self.add_movie_popup_options.user_rating_valid
                {
                    self.add_movie_popup_options.got_user_rating = true;
                }
            }
            KeyCode::Esc => {
                self.close_popups();
            }
            _ => {
                self.add_movie_popup_options
                    .search_input
                    .handle_event(&Event::Key(event));
            }
        }

        Ok(())
    }

    pub(crate) fn draw_add_movie_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = self.center(frame_area, Constraint::Percentage(40), Constraint::Max(7));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Add Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        // frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(popup_area);
        let [_, horiz, _] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .areas(vert);

        if self.add_movie_popup_options.phase == 0 {
            if !self.add_movie_popup_options.finished_search_input {
                let [_, right, left, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(6),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(horiz);
                let prompt_area = Layout::vertical([Constraint::Length(1); 5]).split(right)[2];
                let [_, search_top, search_center, search_bottom, _] =
                    Layout::vertical([Constraint::Length(1); 5]).areas(left);
                let [_, search_input_area, _] = Layout::horizontal([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .areas(search_center);

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
                    .add_movie_popup_options
                    .search_input
                    .visual_scroll(width);
                let cursor_pos = self.add_movie_popup_options.search_input.cursor() - start;
                let mut chars = self
                    .add_movie_popup_options
                    .search_input
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
            } else {
                self.add_movie_popup_options.phase += 1;
                self.accepting_input = false;
            }
        } else if self.add_movie_popup_options.phase == 1 {
            if !self.add_movie_popup_options.requested_search {
                self.add_movie_popup_options.requested_search = true;
                let search_result = Arc::clone(&self.add_movie_popup_options.search_result);
                let tmdb_conf_cloned = app.tmdb_config.clone();
                let search_string = self
                    .add_movie_popup_options
                    .search_input
                    .value()
                    .to_string();
                let search_failed = Arc::clone(&self.add_movie_popup_options.failed);
                let search_finished = Arc::clone(&self.add_movie_popup_options.search_finished);

                thread::spawn(move || {
                    let result = tmdb::find_movie(&tmdb_conf_cloned, &search_string);
                    if result.is_ok() {
                        *search_result.lock().unwrap() = result.unwrap();
                    } else {
                        *search_failed.lock().unwrap() = true;
                    }
                    *search_finished.lock().unwrap() = true;
                });
            }

            if !*self.add_movie_popup_options.search_finished.lock().unwrap() {
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
            } else if *self.add_movie_popup_options.failed.lock().unwrap() {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Error while searching for movie!")
                        .red()
                        .centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            } else {
                self.add_movie_popup_options.phase += 1;
            }
        } else if self.add_movie_popup_options.phase == 2 {
            let results = &self
                .add_movie_popup_options
                .search_result
                .lock()
                .unwrap()
                .results;

            if results.is_empty() {
                *self.add_movie_popup_options.failed.lock().unwrap() = true;

                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Couldn't find movie!").red().centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            } else if !self.add_movie_popup_options.movie_selected {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                self.add_movie_popup_options.movies_visible = 5;

                for (i, area) in areas.iter().enumerate() {
                    if i >= results.len() {
                        break;
                    }
                    let movie = &results[i + self.add_movie_popup_options.scroll_pos as usize];

                    let title_width = (area.width - 20) as usize;

                    let mut name = movie.title.clone();
                    if name.len() > title_width {
                        name.truncate(title_width - 3);
                        name += "...";
                    }

                    let text = format!(
                        "{}{name} - {} - {:.1}",
                        if i == self.add_movie_popup_options.selected as usize {
                            ">"
                        } else {
                            " "
                        },
                        movie.release_date,
                        movie.vote_average
                    );

                    frame.render_widget(Paragraph::new(text), *area);
                }
            } else {
                self.add_movie_popup_options.phase += 1;
                self.accepting_input = true;
                self.add_movie_popup_options.finished_search_input = false;
                self.add_movie_popup_options.search_input = "".into();
            }
        } else if self.add_movie_popup_options.phase == 3 {
            if !self.add_movie_popup_options.got_user_rating {
                let [_, right, left, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(8),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(horiz);
                let prompt_area = Layout::vertical([Constraint::Length(1); 5]).split(right)[2];
                let [_, search_top, search_center, search_bottom, _] =
                    Layout::vertical([Constraint::Length(1); 5]).areas(left);
                let [_, search_input_area, _] = Layout::horizontal([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .areas(search_center);

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
                    .add_movie_popup_options
                    .search_input
                    .visual_scroll(width);
                let cursor_pos = self.add_movie_popup_options.search_input.cursor() - start;
                let mut chars = self
                    .add_movie_popup_options
                    .search_input
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

                let input_parsed = self
                    .add_movie_popup_options
                    .search_input
                    .value()
                    .parse::<f64>();
                self.add_movie_popup_options.user_rating_valid =
                    input_parsed.is_ok() && input_parsed.unwrap() <= 10.0;

                if !self.add_movie_popup_options.user_rating_valid {
                    let error_area = Layout::vertical([Constraint::Length(1); 5]).split(horiz);

                    frame.render_widget(
                        Paragraph::new("Please enter a valid rating!")
                            .red()
                            .centered(),
                        error_area[4],
                    );
                }
            } else {
                self.add_movie_popup_options.user_rating = format!(
                    "{:.1}",
                    self.add_movie_popup_options
                        .search_input
                        .value()
                        .parse::<f32>()
                        .unwrap()
                )
                .parse()
                .unwrap();
                self.add_movie_popup_options.phase += 1;
                self.accepting_input = false;
            }
        } else if self.add_movie_popup_options.phase == 4 {
            if !self.add_movie_popup_options.requested_movie_details {
                self.add_movie_popup_options.requested_movie_details = true;
                let tmdb_details_result =
                    Arc::clone(&self.add_movie_popup_options.tmdb_movie_details_result);
                let trakt_details_result =
                    Arc::clone(&self.add_movie_popup_options.trakt_movie_details_result);
                let tmdb_conf_cloned = app.tmdb_config.clone();
                let trakt_conf_cloned = app.trakt_config.clone();
                let movie_id = self
                    .add_movie_popup_options
                    .search_result
                    .lock()
                    .unwrap()
                    .results[(self.add_movie_popup_options.scroll_pos
                    + self.add_movie_popup_options.selected) as usize]
                    .id;
                let search_failed = Arc::clone(&self.add_movie_popup_options.failed);
                let search_finished =
                    Arc::clone(&self.add_movie_popup_options.movie_details_finished);

                thread::spawn(move || {
                    let tmdb_response = tmdb::get_movie_details(&tmdb_conf_cloned, movie_id);
                    if tmdb_response.is_ok() {
                        let tmdb_response = tmdb_response.unwrap();

                        let trakt_response =
                            trakt::get_movie_details(&trakt_conf_cloned, &tmdb_response.imdb_id);

                        *tmdb_details_result.lock().unwrap() = tmdb_response;

                        if trakt_response.is_ok() {
                            *trakt_details_result.lock().unwrap() = trakt_response.unwrap();
                        } else {
                            *search_failed.lock().unwrap() = true;
                        }
                    } else {
                        *search_failed.lock().unwrap() = true;
                    }
                    *search_finished.lock().unwrap() = true;
                });
            }

            if !*self
                .add_movie_popup_options
                .movie_details_finished
                .lock()
                .unwrap()
            {
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
            } else if *self.add_movie_popup_options.failed.lock().unwrap() {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Error while getting movie details!")
                        .red()
                        .centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            } else {
                if !self.add_movie_popup_options.added_movie {
                    self.add_movie_popup_options.added_movie = true;
                    let tmdb_movie_details = self
                        .add_movie_popup_options
                        .tmdb_movie_details_result
                        .lock()
                        .unwrap()
                        .clone();
                    let trakt_movie_details = self
                        .add_movie_popup_options
                        .trakt_movie_details_result
                        .lock()
                        .unwrap()
                        .clone();

                    // let mut collection: Option<String> = None;
                    // let mut collection_id: Option<u32> = None;
                    // if movie_details.belongs_to_collection.is_some() {
                    //     collection =
                    //         Some(movie_details.belongs_to_collection.clone().unwrap().name);
                    //     collection_id =
                    //         Some(movie_details.belongs_to_collection.clone().unwrap().id);
                    // }
                    // let new_movie = Movie::new(
                    //     movie_details.title,
                    //     self.add_movie_popup_options.user_rating,
                    //     movie_details.vote_average,
                    //     movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
                    //     movie_details.id,
                    //     movie_details
                    //         .genres
                    //         .iter()
                    //         .map(|x| x.name.to_string())
                    //         .collect(),
                    //     movie_details.overview,
                    //     collection,
                    //     collection_id,
                    //     movie_details.runtime,
                    //     movie_details.status == "Released",
                    //     movie_details.tagline,
                    //     movie_details.vote_count,
                    // );

                    app.movies.push(
                        Movie::from(tmdb_movie_details, self.add_movie_popup_options.user_rating)
                            .add_trakt_details(trakt_movie_details),
                    );

                    self.open_fetch_artworks_popup(app)?;
                }

                if self.draw_fetch_artworks_popup(frame, app)? {
                    if app.save_movies().is_err() {
                        *self.add_movie_popup_options.failed.lock().unwrap() = true;
                        let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                        frame.render_widget(
                            Paragraph::new("Couldn't save new rating!").red().centered(),
                            areas[2],
                        );
                        frame.render_widget(
                            Paragraph::new(" Ok ").right_aligned().on_red(),
                            areas[4],
                        );
                    } else {
                        self.close_popups();
                        // self.clear_images(false);
                    }

                    self.main_screen_options.selected =
                        self.main_screen_options.num_visible_movies - 1;
                    self.main_screen_options.scroll_pos =
                        app.movies.len() - self.main_screen_options.selected - 1;
                }
            }
        }

        Ok(())
    }
}
