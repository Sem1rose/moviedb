use crate::{
    app::{App, Movie},
    config_tmdb::Conf,
    tmdb::{self, DetailsResponse, SearchResponse},
};
use ratatui::{
    crossterm::{
        event::{Event, KeyCode, KeyEvent},
        style::{Attribute, SetAttribute},
        ExecutableCommand,
    },
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use std::{
    collections::HashMap,
    error::Error,
    io::{stdout, Write},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use style::palette::tailwind;
use tui_input::backend::crossterm as backend;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub struct MainScreen {
    pub movies_visible: u32,
    pub scroll_pos: u32,
    pub selected: u32,
    // pub search_str: String,
}

impl MainScreen {
    pub fn default() -> Self {
        Self {
            movies_visible: 0,
            scroll_pos: 0,
            selected: 0,
            // search_str: String::default(),
        }
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) -> bool {
        if num_movies == 0 {
            return false;
        }
        if self.scroll_pos + self.selected < num_movies as u32 - 1 {
            if self.selected < self.movies_visible - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }
            return true;
        }

        return false;
    }

    pub fn dec_movie_selection(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;
            return true;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;
            return true;
        }
        return false;
    }
}

#[derive(Default)]
pub struct FetchArtworkPopup {
    pub init_ed: bool,
    pub init_progress: Arc<Mutex<u32>>,
}

impl FetchArtworkPopup {
    pub fn begin(&mut self) {
        self.init_ed = false;
        self.init_progress = Arc::new(Mutex::new(0));
    }
}

#[derive(Default)]
pub struct AddMoviePopup {
    pub phase: u32,
    pub failed: Arc<Mutex<bool>>,
    pub finished_search_input: bool,
    pub search_input: Input,
    pub requested_search: bool,
    pub search_result: Arc<Mutex<SearchResponse>>,
    pub search_finished: Arc<Mutex<bool>>,
    pub movies_visible: u32,
    pub scroll_pos: u32,
    pub selected: u32,
    pub movie_selected: bool,
    pub user_rating_valid: bool,
    pub got_user_rating: bool,
    pub user_rating: f64,
    pub requested_movie_details: bool,
    pub movie_details_result: Arc<Mutex<DetailsResponse>>,
    pub movie_details_finished: Arc<Mutex<bool>>,
    pub added_movie: bool,
}

impl AddMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

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

#[derive(Default)]
pub struct EditMoviePopup {
    pub init_ed: bool,
    pub errored: bool,
    pub user_rating_input: Input,
    pub user_rating_valid: bool,
    pub got_user_rating: bool,
    pub user_rating: f64,
}

impl EditMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }
}

#[derive(Default)]
pub struct RemoveMoviePopup {
    pub errored: bool,
    pub confirmed: bool,
    pub selected: i32,
    pub finished: bool,
}

impl RemoveMoviePopup {
    pub const BUTTONS: i32 = 2;
    pub fn begin(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CurrentScreen {
    InitScreen,
    MainScreen,
    TermSizeWarn,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Popup {
    FetchArtwork,
    AddMovie,
    EditMovie,
    RemoveMovie,
}

pub struct Drawer {
    previous_screen: Option<CurrentScreen>,
    ticks_since_throbber: u8,
    throbber_state: throbber_widgets_tui::ThrobberState,
    images_displayed: Vec<u32>,

    pub update: bool,
    pub accepting_input: bool,
    pub movie_artwork: Arc<Mutex<HashMap<(u32, u32), String>>>,
    pub movie_artworks_requested: Vec<(bool, u32)>,
    pub current_screen: CurrentScreen,
    pub popup: Option<Popup>,
    pub mainscreen_options: MainScreen,
    pub add_movie_popup_options: AddMoviePopup,
    pub edit_movie_popup_options: EditMoviePopup,
    pub remove_movie_popup_options: RemoveMoviePopup,
    pub fetch_artwork_popup_options: FetchArtworkPopup,
    pub backdrop_displayed: bool,
    pub all_movies_displayed: bool,
    pub clear_images: bool,
    pub throbber_visible: bool,
}

const MINTERMSIZE: [u32; 2] = [80, 22];
impl Drawer {
    pub fn default() -> Self {
        Self {
            update: false,
            accepting_input: false,
            movie_artwork: Arc::new(Mutex::new(HashMap::new())),
            movie_artworks_requested: vec![],
            images_displayed: vec![],
            all_movies_displayed: false,
            backdrop_displayed: false,
            current_screen: CurrentScreen::InitScreen,
            popup: Some(Popup::FetchArtwork),
            previous_screen: None,
            mainscreen_options: MainScreen::default(),
            add_movie_popup_options: AddMoviePopup::default(),
            edit_movie_popup_options: EditMoviePopup::default(),
            remove_movie_popup_options: RemoveMoviePopup::default(),
            fetch_artwork_popup_options: FetchArtworkPopup::default(),
            ticks_since_throbber: 0,
            throbber_state: throbber_widgets_tui::ThrobberState::default(),
            clear_images: false,
            throbber_visible: false,
        }
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: u32) {
        if self.mainscreen_options.movies_visible == 0
            || self.mainscreen_options.movies_visible == num_movies_visible
        {
            self.mainscreen_options.movies_visible = num_movies_visible;
        } else {
            self.clear_images(true);

            // don't know why i did all of this
            let current_pos = self.mainscreen_options.scroll_pos + self.mainscreen_options.selected;
            self.mainscreen_options.movies_visible = num_movies_visible;
            if self.mainscreen_options.selected >= num_movies_visible {
                self.mainscreen_options.selected = num_movies_visible - 1;
            }

            self.mainscreen_options.scroll_pos = current_pos - self.mainscreen_options.selected;
        }
    }

    pub fn inc_selection(&mut self, app: &App) {
        if CurrentScreen::MainScreen == self.current_screen {
            if self.popup.is_none() && self.mainscreen_options.movies_visible > 0 {
                if self
                    .mainscreen_options
                    .inc_movie_selection(app.movies.len())
                {
                    self.clear_images(false);
                }
            } else if !self.add_movie_popup_options.movie_selected {
                self.add_movie_popup_options.inc_movie_selection();
                self.update = true;
            }
        }
    }

    pub fn dec_selection(&mut self, app: &App) {
        if CurrentScreen::MainScreen == self.current_screen {
            if self.popup.is_none() {
                if self.mainscreen_options.dec_movie_selection() {
                    self.clear_images(false);
                }
            } else if !self.add_movie_popup_options.movie_selected {
                self.add_movie_popup_options.dec_movie_selection();
                self.update = true;
            }
        }
    }

    pub fn inc_selection_horiz(&mut self, app: &App) {
        if let Some(Popup::RemoveMovie) = self.popup {
            self.remove_movie_popup_options.selected += 1;
            if self.remove_movie_popup_options.selected >= RemoveMoviePopup::BUTTONS {
                self.remove_movie_popup_options.selected = 0;
            }
            self.update = true;
        }
    }

    pub fn dec_selection_horiz(&mut self, app: &App) {
        if let Some(Popup::RemoveMovie) = self.popup {
            self.remove_movie_popup_options.selected -= 1;
            if self.remove_movie_popup_options.selected < 0 {
                self.remove_movie_popup_options.selected = RemoveMoviePopup::BUTTONS - 1;
            }
            self.update = true;
        }
    }

    pub fn handle_input(&mut self, event: &Event) {
        self.update = true;
        match self.popup {
            Some(Popup::AddMovie) => {
                self.add_movie_popup_options
                    .search_input
                    .handle_event(event);
            }
            Some(Popup::EditMovie) => {
                self.edit_movie_popup_options
                    .user_rating_input
                    .handle_event(event);
            }
            _ => {}
        }
    }

    pub fn render_app(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
        config: &Conf,
        // frame_time: f64,
    ) -> Result<(), Box<dyn Error>> {
        self.ticks_since_throbber += 1;
        if self.ticks_since_throbber > 20 {
            self.throbber_state.calc_next();
            self.ticks_since_throbber = 0;
        }

        if !self.check_term_size(frame) {
            if self.current_screen != CurrentScreen::TermSizeWarn {
                self.previous_screen = Some(self.current_screen);
                self.current_screen = CurrentScreen::TermSizeWarn;
            }
        } else if let CurrentScreen::TermSizeWarn = self.current_screen {
            self.current_screen = self.previous_screen.unwrap();
            self.previous_screen = None;
        }

        // if self.popup.is_some() {
        //     match self.popup.unwrap() {
        //         Popup::AddMovie => {
        //             self.draw_add_movie_popup(frame, config, app);
        //         }
        //         _ => {}
        //     }
        // } else {
        //     match self.current_screen {
        //         CurrentScreen::InitScreen => {
        //             self.render_init_screen(frame, config, app)?;
        //         }
        //         CurrentScreen::MainScreen => {
        //             self.render_movies_list(frame, config, app)?;
        //         }
        //         CurrentScreen::TermSizeWarn => {
        //             self.render_term_size_warning(frame)?;
        //         }
        //     }
        // }

        match self.current_screen {
            CurrentScreen::InitScreen => {
                self.render_init_screen(frame, config, app)?;
            }
            CurrentScreen::MainScreen => {
                self.render_movies_list(frame, config, app)?;
            }
            CurrentScreen::TermSizeWarn => {
                self.render_term_size_warning(frame)?;
            }
        }
        if self.popup.is_some() {
            match self.popup.unwrap() {
                Popup::FetchArtwork => {
                    self.draw_fetch_artwork_popup(frame, config, app)?;
                }
                Popup::AddMovie => {
                    self.draw_add_movie_popup(frame, config, app)?;
                }
                Popup::EditMovie => {
                    self.draw_edit_movie_popup(frame, config, app);
                }
                Popup::RemoveMovie => {
                    self.draw_remove_movie_popup(frame, config, app);
                }
                _ => {}
            }
        }

        // frame.render_widget(
        //     Paragraph::new(format!("{:.1}", 1.0 / frame_time)),
        //     // Paragraph::new(format!("{}", app.clear_images)),
        //     frame.area(),
        // );
        Ok(())
    }

    pub fn clear_images(&mut self, clear_cache: bool) {
        self.clear_images = true;
        self.images_displayed.clear();
        self.backdrop_displayed = false;
        if clear_cache {
            self.movie_artwork.lock().unwrap().clear();
            self.movie_artworks_requested.clear();
        }
    }

    pub fn open_fetch_artworks_popup(&mut self) {
        self.popup = Some(Popup::FetchArtwork);
        self.fetch_artwork_popup_options.begin();
    }

    pub fn close_fetch_artworks_popup(&mut self) {
        self.popup = None;
    }

    pub fn open_add_movie_popup(&mut self) {
        self.popup = Some(Popup::AddMovie);
        self.add_movie_popup_options.begin();
        self.accepting_input = true;
    }

    pub fn close_add_movie_popup(&mut self) {
        self.popup = None;
        self.accepting_input = false;
    }

    pub fn open_edit_movie_popup(&mut self) {
        self.popup = Some(Popup::EditMovie);
        self.edit_movie_popup_options.begin();
        self.accepting_input = true;
    }

    pub fn close_edit_movie_popup(&mut self) {
        self.popup = None;
        self.accepting_input = false;
    }

    pub fn open_remove_movie_popup(&mut self) {
        self.popup = Some(Popup::RemoveMovie);
        self.remove_movie_popup_options.begin();
        self.accepting_input = false;
    }

    pub fn close_remove_movie_popup(&mut self) {
        self.popup = None;
        self.accepting_input = false;
    }

    fn center(&self, area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
        let [area] = Layout::horizontal([horizontal])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
        area
    }

    fn check_term_size(&self, frame: &Frame) -> bool {
        if (frame.area().width as u32) < MINTERMSIZE[0]
            || (frame.area().height as u32) < MINTERMSIZE[1]
        {
            return false;
        }
        true
    }
}

impl Drawer {
    fn render_term_size_warning(&mut self, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        let frame_area = frame.area();
        let lines = vec![
            Line::from_iter([
                "Terminal is too small: ".into(),
                frame_area.width.to_string().red(),
                "X".into(),
                frame_area.height.to_string().red(),
            ]),
            Line::default(),
            Line::from_iter([
                "Minimum size is: ".into(),
                MINTERMSIZE[0].to_string().green(),
                "X".into(),
                MINTERMSIZE[1].to_string().green(),
            ]),
        ];
        let area = self.center(
            frame_area,
            Constraint::Min(0),
            Constraint::Length(lines.len() as u16),
        );
        let text = Text::from(lines).centered();

        frame.render_widget(text, area);

        Ok(())
    }

    fn render_init_screen(
        &mut self,
        frame: &mut Frame,
        conf: &Conf,
        app: &mut App,
    ) -> Result<(), Box<dyn Error>> {
        let frame_area = frame.area();
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);

        if *self
            .fetch_artwork_popup_options
            .init_progress
            .lock()
            .unwrap()
            == app.movies.len() as u32
        {
            self.close_fetch_artworks_popup();
            self.current_screen = CurrentScreen::MainScreen;
        }

        Ok(())
    }
}

impl Drawer {
    fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        config: &Conf,
        app: &mut App,
    ) -> Result<(), Box<dyn Error>> {
        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;
        let vert_lay = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(footer_height),
            ],
        )
        .split(frame_area);

        let horiz_lay = Layout::new(
            Direction::Horizontal,
            [Constraint::Min(30), Constraint::Ratio(2, 3)],
        )
        .split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        let movies_lay = Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies])
            .split(horiz_lay[1]);
        self.set_num_movies_visible(num_movies as u32);

        self.all_movies_displayed = true;
        for (i, area) in movies_lay.iter().enumerate() {
            if !app.movies.is_empty()
                && (i + self.mainscreen_options.scroll_pos as usize) < app.movies.len()
            {
                let display_poster =
                    !self.images_displayed.iter().any(|x| *x == i as u32) && self.popup.is_none();
                if display_poster {
                    self.all_movies_displayed = false;
                }
                self.draw_movie_widget(i, config, app, frame, *area, display_poster);
            } else {
                frame.render_widget(
                    Block::new().bg(if i % 2 == 0 {
                        tailwind::NEUTRAL.c900
                    } else {
                        tailwind::STONE.c900
                    }),
                    *area,
                );
            }
        }

        if !app.movies.is_empty() {
            // Must be called after the draw_movie_widget for reasons....
            self.draw_movie_description(
                config,
                app,
                frame,
                horiz_lay[0],
                !self.backdrop_displayed && self.popup.is_none(),
            );

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("🢑"))
                .end_symbol(Some("🢓"))
                .track_symbol(Some("│"))
                .thumb_symbol("▉")
                .thumb_style(Style::new().fg(Color::White))
                .track_style(Style::new().fg(Color::DarkGray).bold())
                .begin_style(Style::new().fg(Color::DarkGray).bold())
                .end_style(Style::new().fg(Color::DarkGray).bold());

            let mut scrollbar_state = ScrollbarState::new(app.movies.len() - num_movies)
                .position(self.mainscreen_options.scroll_pos as usize);

            frame.render_stateful_widget(scrollbar, horiz_lay[1], &mut scrollbar_state);
        }

        Ok(())
    }

    fn draw_movie_widget(
        &mut self,
        id: usize,
        config: &Conf,
        app: &mut App,
        frame: &mut Frame,
        area: Rect,
        draw_poster: bool,
    ) {
        let selected = self.mainscreen_options.selected as usize == id;
        let alt = (self.mainscreen_options.scroll_pos as usize + id) % 2 == 0;
        let movie_id = id as u32 + self.mainscreen_options.scroll_pos;
        let movie = app.movies[movie_id as usize].clone();
        // let poster = get_movie_poster(movie);

        let (background, text, border, selection_highlight) = if selected {
            (
                Color::Rgb(16, 48, 16),
                Color::Rgb(48, 144, 48),
                Color::Rgb(64, 192, 64),
                Color::Rgb(32, 96, 32),
            )
        } else if alt {
            (
                Color::Rgb(48, 16, 16),
                Color::Rgb(144, 48, 48),
                Color::Rgb(192, 64, 64),
                Color::Rgb(96, 32, 32),
            )
        } else {
            (
                Color::Rgb(16, 24, 48),
                Color::Rgb(48, 72, 144),
                Color::Rgb(64, 96, 192),
                Color::Rgb(32, 48, 96),
            )
        };

        let vert_lay = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [highlight_area, poster_area, description_area, _] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Length(movie_width),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .areas(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let mut name = movie.name.clone();
        if name.len() > (description_area.width as usize - 11) {
            name.truncate(description_area.width as usize - 14);
            name += "...";
        }

        let text = vec![
            (name.bold() + " ".into() + movie.year.clone().italic()),
            format!("{:.1}", movie.user_rating).into(),
            "".into(),
            movie.tagline.into(),
        ];

        frame.render_widget(Paragraph::new(text), description_area);

        if selected {
            frame.render_widget(
                Paragraph::new("▐\n".repeat(highlight_area.height as usize))
                    .fg(selection_highlight),
                highlight_area,
            );
        } else {
            frame.render_widget(
                Paragraph::new("▔".repeat(vert_lay[0].width as usize)).fg(border),
                vert_lay[0],
            );
            frame.render_widget(
                Paragraph::new("▁".repeat(vert_lay[2].width as usize)).fg(border),
                vert_lay[2],
            );
        }

        if draw_poster {
            let posters = self.movie_artwork.lock().unwrap();
            if posters.contains_key(&(0, movie_id)) {
                let poster = posters.get(&(0, movie_id));

                let _ = stdout().execute(ratatui::crossterm::cursor::MoveTo(
                    poster_area.x,
                    poster_area.y,
                ));
                println!("{}", poster.cloned().unwrap());

                self.images_displayed.push(id as u32);
            } else {
                drop(posters);

                if !self
                    .movie_artworks_requested
                    .iter()
                    .any(|(_, x)| *x == movie_id)
                {
                    self.movie_artworks_requested.push((false, movie_id));
                    self.request_artwork_async(config, app, movie_id, poster_area, true, 0);
                }
            }
        }
    }

    fn draw_movie_description(
        &mut self,
        config: &Conf,
        app: &mut App,
        frame: &mut Frame,
        area: Rect,
        draw_backdrop: bool,
    ) {
        let movie_id = self.mainscreen_options.selected + self.mainscreen_options.scroll_pos;
        let movie = &app.movies[movie_id as usize];

        let [_, vert, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [_, horiz, _] = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .areas(vert);

        let backdrop_height = ((vert.width - 4) as f32 * 9.0 / 32.0).ceil() as u16;
        let [poster_area, title_area, description_area] = Layout::vertical(vec![
            Constraint::Length(backdrop_height),
            // Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .areas(horiz);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        if draw_backdrop {
            let backdrops = self.movie_artwork.lock().unwrap();
            if backdrops.contains_key(&(1, movie_id)) {
                let poster = backdrops.get(&(1, movie_id));

                let _ = stdout().execute(crossterm::cursor::MoveTo(poster_area.x, poster_area.y));
                println!("{}", poster.cloned().unwrap());

                self.backdrop_displayed = true;
            } else {
                drop(backdrops);

                if !self
                    .movie_artworks_requested
                    .iter()
                    .any(|(y, x)| *x == movie_id && *y)
                {
                    self.movie_artworks_requested.push((true, movie_id));
                    self.request_artwork_async(config, app, movie_id, poster_area, false, 0);
                }
            }
        }

        let subtitle = Line::from_iter([
            "released: ".italic(),
            movie.year.as_str().bold().italic(),
            " ".repeat((title_area.width - 11 - 14).into()).into(),
            "rating: ".italic(),
            format!("{:.1}", movie.vote_average).italic().bold(),
        ]);
        let mut name = movie.name.clone();
        if name.len() > (title_area.width as usize - 5) {
            name.truncate(title_area.width as usize - 8);
            name += "...";
        }

        let lines = vec![
            Line::from(name.as_str().bold()).centered(),
            subtitle,
            Line::from("─".repeat(title_area.width as usize)).dim(),
        ];

        let description = Paragraph::new(movie.overview.as_str()).wrap(Wrap { trim: true });

        frame.render_widget(Text::from(lines), title_area);
        frame.render_widget(description, description_area);
    }

    fn request_artwork_async(
        &mut self,
        config: &Conf,
        app: &App,
        id: u32,
        area: Rect,
        poster: bool,
        expand_width: u16,
    ) {
        let artworks = Arc::clone(&self.movie_artwork);
        let path = config
            .cache
            .join(if poster { "posters" } else { "backdrops" })
            .join(format!("{}.jpg", app.movies[id as usize].id))
            .to_str()
            .unwrap()
            .to_string();

        thread::spawn(move || {
            let data = String::from_utf8_lossy(
                &Command::new("chafa")
                    .args([
                        // "--align",
                        // "top,center",
                        "--relative",
                        "on",
                        "--fit-width",
                        "--view-size",
                        &format!("{}x{}", area.width + expand_width, area.height),
                        &path,
                    ])
                    .stdout(Stdio::piped())
                    .output()
                    .unwrap()
                    .stdout,
            )
            .to_string();

            artworks
                .lock()
                .unwrap()
                .insert((if poster { 0 } else { 1 }, id), data);
        });
    }
}

impl Drawer {
    fn draw_fetch_artwork_popup(
        &mut self,
        frame: &mut Frame,
        conf: &Conf,
        app: &mut App,
    ) -> Result<bool, Box<dyn Error>> {
        if !conf.cache.join(".cached_posters").is_file() {
            std::fs::write(conf.cache.join(".cached_posters"), "")?;
        }
        let contents = std::fs::read_to_string(conf.cache.join(".cached_posters"))?;
        let mut posters_cached: Vec<_> = contents
            .split_ascii_whitespace()
            .map(|x| x.to_string())
            .collect();
        let progress = *self
            .fetch_artwork_popup_options
            .init_progress
            .lock()
            .unwrap();
        let frame_area = frame.area();
        let num_movies = app.movies.len();

        self.throbber_visible = true;

        if !self.fetch_artwork_popup_options.init_ed {
            self.fetch_artwork_popup_options.init_ed = true;

            for movie in &app.movies {
                let movie_id = movie.id;
                if !posters_cached.contains(&movie_id.to_string()) {
                    let conf_owned = conf.clone();
                    let init_prog_owned =
                        Arc::clone(&self.fetch_artwork_popup_options.init_progress);

                    thread::spawn(move || {
                        if let Err(err) = tmdb::get_movie_poster_banner(&conf_owned, movie_id) {
                            panic!("{}", err);
                        }
                        *init_prog_owned.lock().unwrap() += 1;
                    });

                    posters_cached.push(movie_id.to_string());
                } else {
                    *self
                        .fetch_artwork_popup_options
                        .init_progress
                        .lock()
                        .unwrap() += 1;
                }
            }

            std::fs::write(
                conf.cache.join(".cached_posters"),
                posters_cached.join("\n"),
            )?;
        }

        if progress == num_movies as u32 {
            return Ok(true);
        }

        let popup_area = self.center(
            frame_area,
            Constraint::Percentage(50),
            Constraint::Length(11),
        );
        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Working...")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(popup.inner(popup_area));

        let info_text = "Getting movie posters...";
        let [text_lay, throbber_lay] = Layout::horizontal(vec![
            Constraint::Length(info_text.len() as u16),
            Constraint::Length(1),
        ])
        .flex(Flex::Center)
        .areas(layout[1]);

        let throbber = throbber_widgets_tui::Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

        frame.render_widget(info_text, text_lay);
        frame.render_stateful_widget(throbber, throbber_lay, &mut self.throbber_state);

        let [progress_lay] = Layout::horizontal(vec![Constraint::Length(layout[3].width - 6)])
            .flex(Flex::Center)
            .areas(layout[3]);
        let progress_guage = Gauge::default()
            .ratio(progress as f64 / num_movies as f64)
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(format!("{}/{}", progress, num_movies).fg(tailwind::PINK.c500))
            .use_unicode(true);

        frame.render_widget(progress_guage, progress_lay);

        Ok(false)
    }

    fn draw_add_movie_popup(
        &mut self,
        frame: &mut Frame,
        config: &Conf,
        app: &mut App,
    ) -> Result<(), Box<dyn Error>> {
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
                    Paragraph::new("🮂".repeat(search_bottom.width as usize)).fg(tailwind::RED.c700),
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
                self.update = true;
            }
        } else if self.add_movie_popup_options.phase == 1 {
            if !self.add_movie_popup_options.requested_search {
                self.add_movie_popup_options.requested_search = true;
                let search_result = Arc::clone(&self.add_movie_popup_options.search_result);
                let conf_cloned = config.clone();
                let search_string = self
                    .add_movie_popup_options
                    .search_input
                    .value()
                    .to_string();
                let search_failed = Arc::clone(&self.add_movie_popup_options.failed);
                let search_finished = Arc::clone(&self.add_movie_popup_options.search_finished);

                thread::spawn(move || {
                    let result = tmdb::find_movie(&conf_cloned, &search_string);
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

                self.throbber_visible = true;
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
                self.update = true;
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
                self.update = true;
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
                self.update = true;
            }
        } else if self.add_movie_popup_options.phase == 4 {
            if !self.add_movie_popup_options.requested_movie_details {
                self.add_movie_popup_options.requested_movie_details = true;
                let movie_details_result =
                    Arc::clone(&self.add_movie_popup_options.movie_details_result);
                let conf_cloned = config.clone();
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
                    let result = tmdb::get_movie_details(&conf_cloned, movie_id);
                    if result.is_ok() {
                        *movie_details_result.lock().unwrap() = result.unwrap();
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

                self.throbber_visible = true;
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
                    let mut collection: Option<String> = None;
                    let mut collection_id: Option<u32> = None;
                    let movie_details = self
                        .add_movie_popup_options
                        .movie_details_result
                        .lock()
                        .unwrap()
                        .clone();

                    if movie_details.belongs_to_collection.is_some() {
                        collection =
                            Some(movie_details.belongs_to_collection.clone().unwrap().name);
                        collection_id =
                            Some(movie_details.belongs_to_collection.clone().unwrap().id);
                    }
                    let new_movie = Movie::new(
                        movie_details.title,
                        self.add_movie_popup_options.user_rating,
                        movie_details.vote_average,
                        movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
                        movie_details.id,
                        movie_details
                            .genres
                            .iter()
                            .map(|x| x.name.to_string())
                            .collect(),
                        movie_details.overview,
                        collection,
                        collection_id,
                        movie_details.runtime,
                        movie_details.status == "Released",
                        movie_details.tagline,
                        movie_details.vote_count,
                    );
                    app.movies.push(new_movie);
                    self.fetch_artwork_popup_options.begin();
                }

                if self.draw_fetch_artwork_popup(frame, config, app)? {
                    if app.save_movies(config).is_err() {
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
                        self.close_add_movie_popup();
                        self.clear_images(false);
                    }

                    self.mainscreen_options.selected = self.mainscreen_options.movies_visible - 1;
                    self.mainscreen_options.scroll_pos =
                        app.movies.len() as u32 - self.mainscreen_options.selected - 1;
                }
            }
        }

        Ok(())
    }

    fn draw_edit_movie_popup(&mut self, frame: &mut Frame, config: &Conf, app: &mut App) {
        let frame_area = frame.area();
        let popup_area = self.center(frame_area, Constraint::Percentage(40), Constraint::Max(8));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Edit Movie")
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

        if !self.edit_movie_popup_options.init_ed {
            self.edit_movie_popup_options.init_ed = true;
            self.edit_movie_popup_options.user_rating_input = app.movies
                [(self.mainscreen_options.scroll_pos + self.mainscreen_options.selected) as usize]
                .user_rating
                .to_string()
                .into();
        }
        if !self.edit_movie_popup_options.got_user_rating {
            let [_, right, left, _] = Layout::horizontal([
                Constraint::Length(2),
                Constraint::Length(12),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .areas(horiz);
            let prompt_area = Layout::vertical([Constraint::Length(1); 6]).split(right)[2];
            let [_, input_top, input_center, input_bottom, _, _] =
                Layout::vertical([Constraint::Length(1); 6]).areas(left);
            let [_, input_area, _] = Layout::horizontal([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(input_center);

            // ▄▀█ ▂🮂▗▖▘▝
            frame.render_widget(
                Paragraph::new("🮂".repeat(input_bottom.width as usize)).fg(tailwind::RED.c700),
                input_bottom,
            );
            frame.render_widget(
                Paragraph::new("▂".repeat(input_top.width as usize)).fg(tailwind::RED.c700),
                input_top,
            );
            frame.render_widget(Paragraph::new("New rating: "), prompt_area);
            frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

            let areas = Layout::vertical([Constraint::Length(1); 6]).split(horiz);
            let [_, button_area] =
                Layout::horizontal([Constraint::Min(1), Constraint::Length(4)]).areas(areas[5]);
            frame.render_widget(Paragraph::new(" Ok ").on_red().right_aligned(), button_area);

            let width = input_area.width as usize - 1;
            let start = self
                .edit_movie_popup_options
                .user_rating_input
                .visual_scroll(width);
            let cursor_pos = self.edit_movie_popup_options.user_rating_input.cursor() - start;
            let mut chars = self
                .edit_movie_popup_options
                .user_rating_input
                .value()
                .chars()
                .skip(start);

            let mut input_string: Vec<Span> = vec![];
            for i in 0..=(start + width) {
                let c = chars.next().unwrap_or(' ');
                if i == cursor_pos {
                    input_string.push(c.to_string().reversed());
                } else {
                    input_string.push(c.to_string().into());
                }
            }
            frame.render_widget(Line::from_iter(input_string), input_area);

            let input_parsed = self
                .edit_movie_popup_options
                .user_rating_input
                .value()
                .parse::<f64>();
            self.edit_movie_popup_options.user_rating_valid =
                input_parsed.is_ok() && input_parsed.unwrap() <= 10.0;

            if !self.edit_movie_popup_options.user_rating_valid {
                frame.render_widget(
                    Paragraph::new("Please enter a valid rating!")
                        .red()
                        .centered(),
                    areas[4],
                );
            }
        } else {
            self.edit_movie_popup_options.user_rating = format!(
                "{:.1}",
                self.edit_movie_popup_options
                    .user_rating_input
                    .value()
                    .parse::<f32>()
                    .unwrap()
            )
            .parse()
            .unwrap();
            app.movies[(self.mainscreen_options.scroll_pos + self.mainscreen_options.selected)
                as usize]
                .user_rating = self.edit_movie_popup_options.user_rating;

            if app.save_movies(config).is_err() {
                self.edit_movie_popup_options.errored = true;
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Couldn't save new rating!").red().centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            } else {
                self.close_edit_movie_popup();
                self.clear_images(false);
            }
        }
    }

    fn draw_remove_movie_popup(
        &mut self,
        frame: &mut Frame,
        config: &Conf,
        app: &mut App,
    ) -> Result<(), Box<dyn Error>> {
        let frame_area = frame.area();
        let popup_area = self.center(frame_area, Constraint::Percentage(40), Constraint::Max(7));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Remove Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

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

        if !self.remove_movie_popup_options.confirmed {
            let areas = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(horiz);

            frame.render_widget(
                Paragraph::new(format!(
                    "Do you really want to remove {}?",
                    app.movies[(self.mainscreen_options.scroll_pos
                        + self.mainscreen_options.selected)
                        as usize]
                        .name
                ))
                .wrap(Wrap { trim: false }),
                areas[1],
            );

            let button_areas =
                Layout::horizontal([Constraint::Min(1), Constraint::Min(1), Constraint::Min(1)])
                    .split(areas[2]);
            frame.render_widget(
                Paragraph::new(format!(
                    "{}Cancel{}",
                    if self.remove_movie_popup_options.selected == 0 {
                        ">"
                    } else {
                        " "
                    },
                    if self.remove_movie_popup_options.selected == 0 {
                        "<"
                    } else {
                        " "
                    },
                ))
                .centered()
                .on_green(),
                button_areas[2],
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "{}Yes{}",
                    if self.remove_movie_popup_options.selected == 1 {
                        ">"
                    } else {
                        " "
                    },
                    if self.remove_movie_popup_options.selected == 1 {
                        "<"
                    } else {
                        " "
                    },
                ))
                .centered()
                .on_red(),
                button_areas[0],
            );
        } else {
            if !self.remove_movie_popup_options.finished {
                self.remove_movie_popup_options.finished = true;
                app.movies.remove(
                    (self.mainscreen_options.scroll_pos + self.mainscreen_options.selected)
                        as usize,
                );
                if self.mainscreen_options.selected + self.mainscreen_options.scroll_pos
                    >= app.movies.len() as u32
                {
                    if self.mainscreen_options.scroll_pos > 0 {
                        self.mainscreen_options.scroll_pos -= 1;
                    } else if self.mainscreen_options.selected > 0 {
                        self.mainscreen_options.selected -= 1;
                    }
                }
                self.fetch_artwork_popup_options.begin();
            }

            if self.draw_fetch_artwork_popup(frame, config, app)? {
                if app.save_movies(config).is_err() {
                    self.remove_movie_popup_options.errored = true;
                    let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                    frame.render_widget(
                        Paragraph::new("Couldn't save new rating!").red().centered(),
                        areas[2],
                    );
                    frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
                } else {
                    self.close_remove_movie_popup();
                    self.clear_images(false);
                }
            }
        }
        Ok(())
    }
}
