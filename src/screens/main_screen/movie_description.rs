use crate::{
    app::{App, Rating},
    draw::Drawer,
};
use ratatui::style::Stylize;
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use ratatui_image::thread::ThreadImage;
use style::palette::tailwind;

#[derive(Default)]
pub struct MovieDescription {
    pub scroll_pos: usize,
}

impl MovieDescription {
    pub fn scroll_up(&mut self) {}
    pub fn scroll_down(&mut self) {}
}

impl Drawer {
    pub fn draw_movie_description(&mut self, app: &mut App, frame: &mut Frame, area: Rect) {
        let movie = if app.movies.is_empty() {
            None
        } else {
            Some(
                &app.movies[self
                    .main_screen_options
                    .movies_list_options
                    .current_movie_index()],
            )
        };

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
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .areas(horiz);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        if let Some(movie) = movie {
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
        } else {
            // TODO
        }

        if let Some(crate::popups::Popups::FetchArtwork) = self.active_popup {
        } else if movie.is_some() {
            let key = (
                self.main_screen_options
                    .movies_list_options
                    .current_movie_index(),
                true,
            );
            if !self.main_screen_options.hashed_images.contains_key(&key) {
                self.main_screen_options.hash_image(key.0, key.1, app);
            }

            frame.render_stateful_widget(
                ThreadImage::new().resize(ratatui_image::Resize::Scale(Some(
                    ratatui_image::FilterType::Triangle,
                ))),
                poster_area,
                self.main_screen_options
                    .hashed_images
                    .get_mut(&key)
                    .unwrap(),
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), poster_area);
        }
    }
}
