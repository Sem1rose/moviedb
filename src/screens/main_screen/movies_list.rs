use crate::{
    app::{App, Result},
    draw::Drawer,
    helpers::ellipsize_string,
};
use ratatui::style::{Color, Style, Stylize};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use ratatui_image::thread::ThreadImage;
use ratatui_macros::{horizontal, line, text, vertical};
use style::palette::tailwind;

#[derive(Default)]
pub struct MoviesList {
    pub num_visible_movies: usize,
    pub scroll_pos: usize,
    pub selected: usize,
}

impl MoviesList {
    pub fn current_movie_index(&self) -> usize {
        self.scroll_pos + self.selected
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) -> bool {
        if num_movies == 0 {
            return false;
        }

        if self.current_movie_index() < num_movies - 1 {
            if self.selected < self.num_visible_movies - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }

            return true;
        }

        false
    }

    pub fn dec_movie_selection(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;

            return true;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;

            return true;
        }

        false
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: usize) {
        if self.num_visible_movies == 0 {
            self.num_visible_movies = num_movies_visible;
        } else if self.num_visible_movies != num_movies_visible {
            self.num_visible_movies = num_movies_visible;

            if self.selected >= num_movies_visible {
                self.selected = num_movies_visible - 1;
            }

            self.scroll_pos = self.current_movie_index() - self.selected;
        }
    }

    pub fn goto_index(&mut self, num_movies: usize, index: usize) -> bool {
        if index >= num_movies {
            return false;
        }

        if self.num_visible_movies >= num_movies {
            self.scroll_pos = 0;
            self.selected = index;
        } else if num_movies - index < self.num_visible_movies {
            self.scroll_pos = num_movies - self.num_visible_movies;
            self.selected = index - self.scroll_pos;
        } else {
            self.selected = index % self.num_visible_movies;
            self.scroll_pos = (index / self.num_visible_movies) * self.num_visible_movies;
        }

        true
    }
}

impl Drawer {
    pub fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
        area: Rect,
        num_movies: usize,
    ) -> Result<()> {
        let movies_lay = Layout::vertical(vec![Constraint::Min(8); num_movies]).split(area);

        self.main_screen_options
            .movies_list_options
            .set_num_movies_visible(num_movies);

        for (i, area) in movies_lay.iter().enumerate() {
            if !app.movies.is_empty()
                && (i + self.main_screen_options.movies_list_options.scroll_pos) < app.movies.len()
            {
                self.draw_movie_widget(i, app, frame, *area);
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

        if app.movies.len() > num_movies {
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
                .position(self.main_screen_options.movies_list_options.scroll_pos);

            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }

        Ok(())
    }

    fn draw_movie_widget(&mut self, id: usize, app: &mut App, frame: &mut Frame, area: Rect) {
        let selected = self.main_screen_options.movies_list_options.selected == id;
        let alt = (self.main_screen_options.movies_list_options.scroll_pos + id) % 2 == 0;
        let movie_id = id + self.main_screen_options.movies_list_options.scroll_pos;
        let movie = app.movies[movie_id].clone();

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

        let vert_lay = vertical![==1, >=0, ==1].split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [highlight_area, poster_area, description_area, _] =
            horizontal![==2, ==movie_width, >=0, ==2].areas(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let name = ellipsize_string(movie.name.as_str(), description_area.width as usize - 11);

        let text = text![
            (name.bold() + " ".into() + movie.year.italic()),
            format!("{:.1}", movie.user_rating),
            "",
            movie.tagline,
        ];

        frame.render_widget(text, description_area);

        if selected {
            frame.render_widget(
                text![line!["▐"]; highlight_area.height as usize].fg(selection_highlight),
                highlight_area,
            );
        }

        if let Some(crate::popups::Popups::FetchArtwork) = self.active_popup {
        } else {
            let key = (
                self.main_screen_options.movies_list_options.scroll_pos + id,
                false,
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
        }
    }
}
