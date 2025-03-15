use crate::{
    app::{App, Errors, Rating},
    draw::Drawer,
    helpers::ellipsize_string,
};
use ratatui::style::{Color, Style, Stylize};
use ratatui::{crossterm::ExecutableCommand, layout::*, prelude::*, widgets::*, Frame};
use ratatui_macros::{constraints, horizontal, line, span, text, vertical};
use std::{
    io::stdout,
    process::{Command, Stdio},
    sync::Arc,
    thread,
};
use style::palette::tailwind;

impl Drawer {
    pub fn render_movies_list(&mut self, frame: &mut Frame, app: &mut App) -> Result<(), Errors> {
        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        let movies_lay = Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies])
            .split(horiz_lay[1]);

        self.set_num_movies_visible(num_movies as u32);

        self.all_movies_displayed = true;
        for (i, area) in movies_lay.iter().enumerate() {
            if !app.movies.is_empty()
                && (i + self.main_screen_options.scroll_pos as usize) < app.movies.len()
            {
                let display_poster = !self.images_displayed.iter().any(|x| *x == i as u32)
                    && self.active_popup.is_none();
                if display_poster {
                    self.all_movies_displayed = false;
                }

                self.draw_movie_widget(i, app, frame, *area, display_poster);
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
                app,
                frame,
                horiz_lay[0],
                !self.backdrop_displayed && self.active_popup.is_none(),
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
                .position(self.main_screen_options.scroll_pos as usize);

            frame.render_stateful_widget(scrollbar, horiz_lay[1], &mut scrollbar_state);
        }

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
        let selected = self.main_screen_options.selected as usize == id;
        let alt = (self.main_screen_options.scroll_pos as usize + id) % 2 == 0;
        let movie_id = id as u32 + self.main_screen_options.scroll_pos;
        let movie = app.movies[movie_id as usize].clone();
        // let poster = get_movie_poster(movie);

        // TODO: create a themes framework, maybe in the config
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

        // let vert_lay = Layout::vertical([
        //     Constraint::Length(1),
        //     Constraint::Min(0),
        //     Constraint::Length(1),
        // ])
        // .split(area);
        let vert_lay = vertical![==1, >=0, ==1].split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [highlight_area, poster_area, description_area, _] =
            horizontal![==2, ==movie_width, >=0, ==2].areas(vert_lay[1]);
        // let [highlight_area, poster_area, description_area, _] = Layout::horizontal([
        //     Constraint::Length(2),
        //     Constraint::Length(movie_width),
        //     Constraint::Min(0),
        //     Constraint::Length(2),
        // ])
        // .areas(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let name = ellipsize_string(movie.name.as_str(), description_area.width as usize - 11);

        // let text = vec![
        //     (name.bold() + " ".into() + movie.year.clone().italic()),
        //     format!("{:.1}", movie.user_rating).into(),
        //     "".into(),
        //     movie.tagline.into(),
        // ];

        let text = text![
            (name.bold() + " ".into() + movie.year.as_str().italic()),
            format!("{:.1}", movie.user_rating),
            "",
            movie.tagline,
        ];

        // frame.render_widget(Paragraph::new(text), description_area);
        frame.render_widget(text, description_area);

        if selected {
            frame.render_widget(
                text![line!["▐"]; highlight_area.height as usize].fg(selection_highlight),
                highlight_area,
            );
            // } else {
            //     frame.render_widget(
            //         text!["▔".repeat(vert_lay[0].width as usize)].fg(border),
            //         vert_lay[0],
            //     );
            //     frame.render_widget(
            //         text!["▁".repeat(vert_lay[2].width as usize)].fg(border),
            //         vert_lay[2],
            //     );
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
                    self.request_artwork_async(app, movie_id, poster_area, true, 0);
                }
            }
        }
    }

    fn draw_movie_description(
        &mut self,
        app: &mut App,
        frame: &mut Frame,
        area: Rect,
        draw_backdrop: bool,
    ) {
        let movie_id = self.main_screen_options.selected + self.main_screen_options.scroll_pos;
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
                    self.request_artwork_async(app, movie_id, poster_area, false, 0);
                }
            }
        }

        let subtitle = Line::from_iter([
            "released: ".italic(),
            movie.year.as_str().bold().italic(),
            " ".repeat((title_area.width - 11 - 14).into()).into(),
            "rating: ".italic(),
            if let Rating::TMDB(rating, count) = movie.ratings[1] {
                format!("{:.1}", rating).italic().bold()
            } else if let Rating::Trakt(rating, count) = movie.ratings[1] {
                format!("{:.1}", rating).italic().bold()
            } else {
                "nan".into()
            },
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
        app: &App,
        id: u32,
        area: Rect,
        poster: bool,
        expand_width: u16,
    ) {
        let artworks = Arc::clone(&self.movie_artwork);
        let path = if poster {
            &app.config.dirs.poster_cache
        } else {
            &app.config.dirs.backdrop_cache
        }
        .join(format!("{}.jpg", app.movies[id as usize].tmdb_id))
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
