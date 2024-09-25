use crate::app::{App, Movie};
use crossterm::ExecutableCommand;
use rand::prelude::*;
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::stdout,
    path::PathBuf,
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

pub struct Drawer {
    movie_posters: Arc<Mutex<HashMap<u32, String>>>,
    movie_posters_requested: Vec<u32>,
    images_displayed: Vec<u32>,
    mainscreen_options: MainScreen,

    // TODO: temp delete
    paths: Vec<PathBuf>,

    pub clear_images: bool,
}

impl Drawer {
    pub fn default() -> Self {
        Self {
            movie_posters: Arc::new(Mutex::new(HashMap::new())),
            movie_posters_requested: vec![],
            images_displayed: vec![],
            clear_images: false,
            paths: fs::read_dir("./src")
                .unwrap()
                .filter_map(|x| x.ok())
                .map(|x| x.path())
                .filter(|x| x.extension().map_or(false, |x| x == "jpg"))
                .collect::<Vec<_>>(),
            mainscreen_options: MainScreen::default(),
        }
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: u32) {
        if self.mainscreen_options.movies_visible == 0
            || num_movies_visible == self.mainscreen_options.movies_visible
        {
            self.mainscreen_options.movies_visible = num_movies_visible;
        } else {
            self.clear_images = true;
            self.movie_posters_requested.clear();
            self.movie_posters.lock().unwrap().clear();

            // don't know why i did all of this
            let current_pos = self.mainscreen_options.scroll_pos + self.mainscreen_options.selected;
            self.mainscreen_options.movies_visible = num_movies_visible;
            if self.mainscreen_options.selected >= num_movies_visible {
                self.mainscreen_options.selected = num_movies_visible - 1;
            }

            self.mainscreen_options.scroll_pos = current_pos - self.mainscreen_options.selected;
        }
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) {
        if self.mainscreen_options.scroll_pos + self.mainscreen_options.selected
            < num_movies as u32 - 1
        {
            if self.mainscreen_options.selected < self.mainscreen_options.movies_visible - 1 {
                self.mainscreen_options.selected += 1;
            } else {
                self.clear_images = true;
                self.images_displayed.clear();
                self.mainscreen_options.scroll_pos += 1;
            }
        }
    }

    pub fn dec_movie_selection(&mut self) {
        if self.mainscreen_options.selected > 0 {
            self.mainscreen_options.selected -= 1;
        } else if self.mainscreen_options.scroll_pos > 0 {
            self.clear_images = true;
            self.images_displayed.clear();
            self.mainscreen_options.scroll_pos -= 1;
        }
    }
}

impl Drawer {
    pub fn ui(&mut self, frame: &mut Frame, app: &mut App) -> Result<(), Box<dyn Error>> {
        let now = std::time::Instant::now();
        self.render_movies_list(frame, app)?;
        frame.render_widget(
            Paragraph::new(format!("{:.1}", 1.0 / now.elapsed().as_secs_f32())),
            // Paragraph::new(format!("{}", app.clear_images)),
            frame.area(),
        );
        Ok(())
    }

    fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
    ) -> Result<(), Box<dyn Error>> {
        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 3) as f32 / 8.0).floor() as usize;
        let status_height = (((frame_area.height - 3) % 8) % num_movies as u16) + 2;
        let vert_lay = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(status_height),
                Constraint::Min(12),
                Constraint::Length(1),
            ],
        )
        .split(frame_area);

        let horiz_lay = Layout::new(
            Direction::Horizontal,
            [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)],
        )
        .split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), horiz_lay[0]);

        let movies_lay = Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies])
            .split(horiz_lay[1]);
        self.set_num_movies_visible(num_movies as u32);

        for (i, area) in movies_lay.iter().enumerate() {
            if (i + self.mainscreen_options.scroll_pos as usize) < app.movies.len() {
                self.draw_movie_widget(
                    i,
                    app,
                    frame,
                    *area,
                    !self.images_displayed.iter().any(|x| *x == i as u32),
                );
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

    fn draw_movie_widget(
        &mut self,
        id: usize,
        app: &mut App,
        frame: &mut Frame,
        area: Rect,
        draw_poster: bool,
    ) {
        let selected = self.mainscreen_options.selected as usize == id;
        let alt = (self.mainscreen_options.scroll_pos as usize + id) % 2 == 0;
        let movie = app.movies[self.mainscreen_options.scroll_pos as usize + id].clone();
        let movie_id = id as u32 + self.mainscreen_options.scroll_pos;
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

        let vert_lay = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ],
        )
        .split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2;
        let horiz_lay = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(2),
                Constraint::Length(movie_width),
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(2),
            ],
        )
        .split(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let mut name = movie.name.clone();
        if name.len() > 50 {
            name.truncate(50);
            name += "...";
        }

        let text = vec![
            (name.bold() + " ".into() + movie.year.clone().italic().dim()),
            format!("{:.1}", movie.rating).into(),
        ];

        frame.render_widget(Paragraph::new(text), horiz_lay[3]);

        if selected {
            frame.render_widget(
                Paragraph::new("▐\n".repeat(horiz_lay[0].height as usize)).fg(selection_highlight),
                horiz_lay[0],
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
            let posters = self.movie_posters.lock().unwrap();
            if posters.contains_key(&movie_id) {
                let poster = posters.get(&movie_id);

                let _ = stdout().execute(crossterm::cursor::MoveTo(horiz_lay[1].x, horiz_lay[1].y));
                println!("{}", poster.cloned().unwrap());

                self.images_displayed.push(id as u32);
            } else {
                drop(posters);

                if !self.movie_posters_requested.iter().any(|x| *x == movie_id) {
                    self.movie_posters_requested.push(movie_id);
                    self.request_poster_async(app, movie_id, horiz_lay[1]);
                }
            }
        }
    }

    fn request_poster_async(&mut self, app: &App, id: u32, area: Rect) {
        let posters = Arc::clone(&self.movie_posters);
        let path = self
            .get_movie_poster(&app.movies[id as usize])
            .to_str()
            .unwrap()
            .to_string();

        thread::spawn(move || {
            let data = String::from_utf8_lossy(
                &Command::new("chafa")
                    .args([
                        "--align",
                        "top,right",
                        "--relative",
                        "on",
                        "--view-size",
                        &format!("{}x{}", area.width, area.height),
                        &path,
                    ])
                    .stdout(Stdio::piped())
                    .output()
                    .unwrap()
                    .stdout,
            )
            .to_string();

            posters.lock().unwrap().insert(id, data);
        });
    }

    fn get_movie_poster(&mut self, movie: &Movie) -> PathBuf {
        // TODO: implement proper poster fetching.
        self.paths[rand::thread_rng().gen_range(0..self.paths.len())].clone()
    }
}
