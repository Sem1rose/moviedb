use crate::{app::App, custom::helpers::ellipsize_string, draw::Drawer, types::*};
use ratatui::style::Stylize;
use ratatui::{layout::Rect, prelude::*, widgets::*, Frame};
use ratatui_macros::{horizontal, vertical};
use style::palette::tailwind;

#[derive(Default)]
pub struct MovieDescription {
    pub selected_tab: usize,
    // pub scroll_pos: usize,
}

const TABS: [&str; 2] = ["Overview", "Review"];
impl MovieDescription {
    // pub fn scroll_up(&mut self) {}
    // pub fn scroll_down(&mut self) {}
    pub fn next_tab(&mut self) {
        self.selected_tab += 1;
        if self.selected_tab >= TABS.len() {
            self.selected_tab = 0;
        }
    }

    pub fn prev_tab(&mut self) {
        self.selected_tab = self.selected_tab.checked_sub(1).unwrap_or(TABS.len() - 1);
    }
}

impl Drawer {
    fn draw_ratings(&self, movie: &Movie, frame: &mut Frame, area: Rect) {
        let imdb_bg = Color::Rgb(245, 197, 24);
        let imdb_fg = Color::Black;
        let imdb_label_fg = Color::Rgb(250, 225, 120);
        let trakt_bg = Color::Rgb(165, 61, 185);
        let trakt_fg = Color::White;
        let trakt_label_fg = Color::Rgb(230, 140, 245);
        let tmdb_bg = Color::Rgb(42, 187, 209);
        let tmdb_fg = Color::Black;
        let tmdb_label_fg = Color::Rgb(140, 205, 215);

        let mut ratings = vec![];
        for rating in movie.ratings {
            if let Rating::IMDB(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
            if let Rating::Trakt(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
            if let Rating::TMDB(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
        }

        if ratings.is_empty() {
            frame.render_widget(Line::from("NA").centered(), area);

            return;
        } else if ratings.len() == 1 {
            let mut bg = Color::default();
            let mut fg = Color::default();
            let mut rating = f64::default();
            if let Rating::IMDB(a, _) = ratings[0] {
                bg = imdb_bg;
                fg = imdb_fg;
                rating = a;
            } else if let Rating::Trakt(a, _) = ratings[0] {
                bg = trakt_bg;
                fg = trakt_fg;
                rating = a;
            } else if let Rating::TMDB(a, _) = ratings[0] {
                bg = tmdb_bg;
                fg = tmdb_fg;
                rating = a;
            }

            let widget = vec![
                "".fg(bg),
                format!("{:.1}", rating).bg(bg).fg(fg).bold(),
                "".fg(bg),
            ];

            frame.render_widget(Line::from(widget).centered(), area);

            return;
        }

        let spaces = ((area.width - 5 * (ratings.len() as u16)) as f64 / (ratings.len() + 1) as f64)
            .ceil() as usize;

        let mut widgets = Line::from(" ".repeat(spaces));
        let mut labels = Line::from(" ".repeat(spaces));
        for (i, rating) in movie.ratings.iter().enumerate() {
            let bg;
            let fg;
            let r;
            if let Rating::IMDB(a, _) = rating {
                labels.push_span(Span::from("IMDB").fg(imdb_label_fg));
                if i != movie.ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces + 1));
                }
                bg = imdb_bg;
                fg = imdb_fg;
                r = a;
            } else if let Rating::Trakt(a, _) = rating {
                labels.push_span(Span::from("Trakt").fg(trakt_label_fg));
                if i != movie.ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces));
                }
                bg = trakt_bg;
                fg = trakt_fg;
                r = a;
            } else if let Rating::TMDB(a, _) = rating {
                labels.push_span(Span::from("TMDB").fg(tmdb_label_fg));
                if i != movie.ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces + 1));
                }
                bg = tmdb_bg;
                fg = tmdb_fg;
                r = a;
            } else {
                continue;
            }

            widgets.push_span("".fg(bg));
            widgets.push_span(format!("{:.1}", r).bg(bg).fg(fg).bold());
            widgets.push_span("".fg(bg));
            if i != movie.ratings.len() - 1 {
                widgets.push_span(" ".repeat(spaces));
            }
        }

        frame.render_widget(Text::from(vec![widgets, labels]), area);
    }

    pub fn draw_movie_description(&mut self, app: &mut App, frame: &mut Frame, area: Rect) {
        let movie = if self.main_screen.filtered_movies.is_empty() {
            None
        } else {
            Some(
                &self.main_screen.filtered_movies
                    [self.main_screen.movies_list.current_movie_index()],
            )
        };

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(area);

        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        let backdrop_height = ((vert.width - 4) as f32 * 9.0 / 32.0).ceil() as u16;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==7, >=1].areas(horiz);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        if let Some(movie) = movie {
            let [title_area, _, ratings_area, tabs_area] =
                vertical![==2, ==1, ==2, ==2].areas(title_area);

            let mut name = movie.name.clone();
            name = ellipsize_string(&name, title_area.width as usize);

            frame.render_widget(
                Text::from(vec![
                    Line::from(name).bold().centered(),
                    Line::from(movie.year.as_str().italic()).centered(),
                ]),
                title_area,
            );
            self.draw_ratings(movie, frame, ratings_area);

            let mut tabs = TABS
                .iter()
                .flat_map(|x| [" ".into(), Span::from(format!(" {} ", *x))])
                .collect::<Vec<_>>();
            tabs[self.main_screen.movie_description.selected_tab * 2 + 1] = tabs
                [self.main_screen.movie_description.selected_tab * 2 + 1]
                .clone()
                .reversed()
                .bold();
            frame.render_widget(
                Text::from(vec![
                    Line::from(tabs),
                    "🮂".repeat(title_area.width as usize).into(),
                ]),
                tabs_area,
            );

            let description = match self.main_screen.movie_description.selected_tab {
                0 => Paragraph::new(movie.overview.as_str())
                    .wrap(Wrap { trim: true })
                    .centered(),
                1 => Paragraph::new(movie.name.as_str())
                    .wrap(Wrap { trim: true })
                    .centered(),
                _ => Paragraph::new("NA").wrap(Wrap { trim: true }).centered(),
            };
            frame.render_widget(description, description_area);
        } else {
            // TODO
        }

        if movie.is_some() {
            self.image_backend.draw_image(
                app,
                self.main_screen.current_movie().id.tmdb,
                true,
                backdrop_area,
                frame,
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), backdrop_area);
        }
    }
}
