use crate::{app::App, config_tmdb::Conf, tmdb};
use crossterm::ExecutableCommand;
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use std::{
    collections::HashMap,
    error::Error,
    io::stdout,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use style::palette::tailwind;

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
}

#[derive(Clone, Copy, PartialEq)]
pub enum CurrentScreen {
    InitScreen,
    MainScreen(Option<VisiblePopup>),
    TermSizeWarn,
}

#[derive(Clone, Copy, PartialEq)]
pub enum VisiblePopup {
    FetchingMoviesPosters,
}

pub struct Drawer {
    previous_screen: Option<CurrentScreen>,
    ticks_since_throbber: u8,
    throbber_state: throbber_widgets_tui::ThrobberState,
    images_displayed: Vec<u32>,

    pub init_ed: bool,
    pub init_progress: Arc<Mutex<u32>>,
    pub movie_artwork: Arc<Mutex<HashMap<(u32, u32), String>>>,
    pub movie_artworks_requested: Vec<(bool, u32)>,
    pub current_screen: CurrentScreen,
    pub mainscreen_options: MainScreen,
    pub backdrop_displayed: bool,
    pub all_movies_displayed: bool,
    pub clear_images: bool,
    pub throbber_visible: bool,
}

const MINTERMSIZE: [u32; 2] = [80, 22];
impl Drawer {
    pub fn default() -> Self {
        Self {
            init_progress: Arc::new(Mutex::new(0)),
            init_ed: false,
            movie_artwork: Arc::new(Mutex::new(HashMap::new())),
            movie_artworks_requested: vec![],
            images_displayed: vec![],
            all_movies_displayed: false,
            backdrop_displayed: false,
            current_screen: CurrentScreen::InitScreen,
            previous_screen: None,
            mainscreen_options: MainScreen::default(),
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
        if let CurrentScreen::MainScreen(_) = self.current_screen {
            self.inc_movie_selection(app.movies.len());
        }
    }

    pub fn dec_selection(&mut self, app: &App) {
        if let CurrentScreen::MainScreen(_) = self.current_screen {
            self.dec_movie_selection();
        }
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) {
        if self.mainscreen_options.scroll_pos + self.mainscreen_options.selected
            < num_movies as u32 - 1
        {
            if self.mainscreen_options.selected < self.mainscreen_options.movies_visible - 1 {
                self.mainscreen_options.selected += 1;
            } else {
                self.mainscreen_options.scroll_pos += 1;
            }
            self.clear_images(false);
        }
    }

    pub fn dec_movie_selection(&mut self) {
        if self.mainscreen_options.selected > 0 {
            self.mainscreen_options.selected -= 1;
            self.clear_images(false);
        } else if self.mainscreen_options.scroll_pos > 0 {
            self.mainscreen_options.scroll_pos -= 1;
            self.clear_images(false);
        }
    }

    pub fn ui(
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

        self.throbber_visible = false;
        match self.current_screen {
            CurrentScreen::InitScreen => {
                self.render_init_screen(frame, config, app)?;
            }
            CurrentScreen::MainScreen(popup) => {
                self.render_movies_list(frame, config, app)?;
            }
            CurrentScreen::TermSizeWarn => {
                self.render_term_size_warning(frame)?;
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
        if !conf.cache.join(".cached_posters").is_file() {
            std::fs::write(conf.cache.join(".cached_posters"), "")?;
        }
        let contents = std::fs::read_to_string(conf.cache.join(".cached_posters"))?;
        let mut posters_cached: Vec<_> = contents
            .split_ascii_whitespace()
            .map(|x| x.to_string())
            .collect();
        let progress = *self.init_progress.lock().unwrap();
        let frame_area = frame.area();
        let num_movies = app.movies.len();

        self.throbber_visible = true;

        if !self.init_ed {
            self.init_ed = true;

            for movie in &app.movies {
                let movie_id = movie.id;
                if !posters_cached.contains(&movie_id.to_string()) {
                    let conf_owned = conf.clone();
                    let init_prog_owned = Arc::clone(&self.init_progress);

                    thread::spawn(move || {
                        if let Err(err) = tmdb::get_movie_poster_banner(&conf_owned, movie_id) {
                            panic!("{}", err);
                        }
                        *init_prog_owned.lock().unwrap() += 1;
                    });

                    posters_cached.push(movie_id.to_string());
                } else {
                    *self.init_progress.lock().unwrap() += 1;
                }
            }

            std::fs::write(
                conf.cache.join(".cached_posters"),
                posters_cached.join("\n"),
            )?;
        }

        if progress == num_movies as u32 {
            self.current_screen = CurrentScreen::MainScreen(None);
            return Ok(());
        }

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);

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
            if (i + self.mainscreen_options.scroll_pos as usize) < app.movies.len() {
                let display_poster = !self.images_displayed.iter().any(|x| *x == i as u32);
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

        // Must be called after the draw_movie_widget for reasons....
        self.draw_movie_description(config, app, frame, horiz_lay[0], !self.backdrop_displayed);

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

        Ok(())
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
        let [poster_area, _, title_area, description_area] = Layout::vertical(vec![
            Constraint::Length(backdrop_height),
            Constraint::Length(1),
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
                    self.request_artwork_async(config, app, movie_id, poster_area, false, 1);
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
            name.truncate(title_area.width as usize - 5);
            name += "...";
        }

        // "released: ".italic().into_left_aligned_line(),
        // "rating: ".italic().into_centered_line(),
        // movie
        //     .rating
        //     .to_string()
        //     .as_str()
        //     .italic()
        //     .bold()
        //     .into_centered_line(),
        let lines = vec![
            Line::from(name.as_str().bold()),
            subtitle,
            Line::from("─".repeat(title_area.width as usize)).dim(),
        ];

        let description = Paragraph::new(movie.overview.as_str()).wrap(Wrap { trim: true });

        frame.render_widget(Text::from(lines), title_area);
        frame.render_widget(description, description_area);
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

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2;
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
            name.truncate(description_area.width as usize - 11);
            name += "...";
        }

        let text = vec![
            (name.bold() + " ".into() + movie.year.clone().italic().dim()),
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

                let _ = stdout().execute(crossterm::cursor::MoveTo(poster_area.x, poster_area.y));
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
                        "--align",
                        "top",
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
