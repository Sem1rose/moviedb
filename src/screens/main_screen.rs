use std::cmp::Ordering;
use std::ops::Add;
use std::path::PathBuf;

use crate::helpers::{add_padding, ellipsize_string};
use crate::image_backend::RatatuiImage;
use crate::screens::Screens;
use crate::types::{Movie, Rating};
use crate::KeyEventHandler;
use nucleo_matcher::{pattern::Atom, Config, Matcher};
use ratatui::style::palette::material;
use ratatui::style::Styled;
use ratatui::symbols::block;
use ratatui::symbols::scrollbar::Set;
use ratatui::widgets::{Block, Padding, Paragraph, Scrollbar, ScrollbarState, Wrap};
use ratatui::{prelude::*, style::palette::tailwind};
use ratatui_macros::{horizontal, line, text, vertical};
use tui_textarea::TextArea;

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub enum Sort {
    #[default]
    AddedDate,
    UserRating,
    Relevance,
    Rating,
    Name,
    ReleaseDate,
}

pub struct MainScreen {
    tab: usize,
    item: usize,
    pub redraw_images: u8,
    pub drawing_images: bool,

    pub image_renderer: RatatuiImage,
    movies: Vec<Movie>,

    movies_list_selected_item: usize,
    movies_list_visible_items: usize,
    movies_list_scroll_pos: usize,
    movies_description_selected_tab: usize,

    search_input: TextArea<'static>,
    pub sort: Sort,
    pub sort_ascending: bool,
}

impl MainScreen {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    pub fn new(cache_dir: &PathBuf) -> Self {
        Self {
            tab: 0,
            item: 0,
            redraw_images: 0,
            drawing_images: false,
            image_renderer: RatatuiImage::new(cache_dir),
            movies: vec![],
            movies_list_selected_item: 0,
            movies_list_visible_items: 0,
            movies_list_scroll_pos: 0,
            movies_description_selected_tab: 0,
            search_input: TextArea::default(),
            sort: Sort::default(),
            sort_ascending: false,
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        for tab in 0..=1 {
            key_event_handler.bind_key((Some(tab), None), '/', |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 0;
                }
            });
            key_event_handler.bind_key((Some(tab), None), 'f', |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 0;
                }
            });
            key_event_handler.bind_key((Some(tab), None), 'e', |app, _| {
                app.drawer.open_edit_movie_popup();
            });
            key_event_handler.bind_key((Some(tab), None), 'd', |app, _| {
                app.drawer.open_remove_movie_popup();
            });
        }
        key_event_handler.bind_esc((Some(2), None), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_enter((Some(2), None), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);
        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), horiz_lay[0]);

        let [_, input_area, _] = horizontal![>=1, <=25, ==1].areas(vert_lay[0]);

        let search_selected = self.tab == 2;
        if search_selected {
            key_event_handler.bind_input_field((Some(2), Some(0)), |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Key(key_event) => {
                            main_screen.search_input.input(key_event);
                        }
                        _ => {}
                    }
                }
            });
        }
        self.search_input
            .set_style(Style::new().fg(if search_selected {
                tailwind::SLATE.c200
            } else {
                tailwind::STONE.c400
            }));
        self.search_input.set_cursor_style(
            Style::new()
                .fg(if search_selected {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                })
                .add_modifier(if search_selected {
                    Modifier::REVERSED
                } else {
                    Modifier::default()
                }),
        );
        self.search_input.set_block(
            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Thick)
                .style(Style::new().fg(if search_selected {
                    material::BLUE.c500
                } else {
                    tailwind::STONE.c600
                }))
                .padding(Padding {
                    left: 1,
                    right: 1,
                    top: 0,
                    bottom: 0,
                }), // .title_style(Style::new().fg(if selected {
                    //     material::BLUE.c600
                    // } else {
                    //     tailwind::SLATE.c600
                    // })),
        );
        self.search_input.set_placeholder_text("Search");
        self.search_input
            .set_placeholder_style(Style::new().fg(material::GRAY.c700));
        frame.render_widget(&self.search_input, input_area);

        self.drawing_images = false;
        self.render_movies_list(frame, horiz_lay[1], num_movies, key_event_handler)?;
        self.draw_movie_description(frame, horiz_lay[0], key_event_handler);
        self.redraw_images = self.redraw_images.saturating_sub(1);

        Ok(())
    }

    pub fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        num_movies: usize,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        key_event_handler.bind_vertical((Some(0), None), move |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        if num_movies > 0 {
                            main_screen.movies_list_selected_item = main_screen
                                .movies_list_selected_item
                                .add(1)
                                .min(main_screen.movies.len() - 1);
                            if main_screen.movies_list_selected_item
                                - main_screen.movies_list_scroll_pos
                                >= main_screen.movies_list_visible_items
                            {
                                main_screen.movies_list_scroll_pos += 1;
                            }
                        }
                    }
                    crate::key_event_handler::Data::Direction(false) => {
                        main_screen.movies_list_selected_item =
                            main_screen.movies_list_selected_item.saturating_sub(1);
                        if main_screen.movies_list_selected_item
                            < main_screen.movies_list_scroll_pos
                        {
                            main_screen.movies_list_scroll_pos -= 1;
                        }
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_key((Some(0), None), 'g', |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.goto_index(0);
            }
        });
        key_event_handler.bind_key((Some(0), None), 'G', |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.goto_index(main_screen.movies.len() - 1);
            }
        });
        key_event_handler.bind_tab((Some(0), None), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        main_screen.tab += 1;
                        if main_screen.tab > 1 {
                            main_screen.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false) => {
                        main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }
            }
        });

        if self.movies_list_selected_item >= self.movies.len() {
            self.movies_list_selected_item = self.movies.len() - 1;
            self.movies_list_scroll_pos =
                self.movies_list_selected_item - self.movies_list_visible_items + 1;
        }

        let [movies_area, scrollbar_area] = horizontal![>=0, ==1].areas(area);
        let movies_lay = Layout::vertical(vec![Constraint::Min(8); num_movies]).split(movies_area);
        if self.movies_list_visible_items == 0 {
            self.movies_list_visible_items = num_movies;
        } else if self.movies_list_visible_items != num_movies {
            self.movies_list_visible_items = num_movies;

            if self.movies_list_selected_item - self.movies_list_scroll_pos >= num_movies {
                self.movies_list_scroll_pos = self.movies_list_selected_item - num_movies + 1;
            }
        }

        for (i, area) in movies_lay.iter().enumerate() {
            if !self.movies.is_empty() && (i + self.movies_list_scroll_pos) < self.movies.len() {
                self.draw_movie_widget(i, frame, *area);
            } else {
                frame.render_widget(
                    Block::new().bg(if i & 1 == 1 {
                        tailwind::NEUTRAL.c900
                    } else {
                        tailwind::STONE.c900
                    }),
                    *area,
                );
            }
        }

        if self.movies.len() > num_movies {
            let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .symbols(Set {
                    track: block::FULL,
                    thumb: block::FULL,
                    begin: "▲",
                    end: "▼",
                })
                .begin_style(
                    Style::new()
                        .bg(material::LIGHT_BLUE.c700)
                        .fg(tailwind::INDIGO.c900),
                )
                .end_style(
                    Style::new()
                        .bg(material::LIGHT_BLUE.c700)
                        .fg(tailwind::INDIGO.c900),
                )
                .track_style(Style::new().fg(tailwind::SLATE.c900))
                .thumb_style(
                    Style::new()
                        .fg(material::BLUE.c800)
                        .bg(tailwind::SLATE.c900),
                );

            let mut scrollbar_state = ScrollbarState::new(self.movies.len() - num_movies)
                .position(self.movies_list_scroll_pos);

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }

        Ok(())
    }

    fn draw_movie_widget(&mut self, id: usize, frame: &mut Frame, area: Rect) {
        let movie_id = self.movies_list_scroll_pos + id;
        let selected = self.movies_list_selected_item == movie_id;
        let tab_selected = self.tab == 0;
        let alt = movie_id & 1 == 1;
        let movie = &self.movies[movie_id];

        let (background, text) = if selected {
            if tab_selected {
                (tailwind::EMERALD.c800, tailwind::EMERALD.c200)
            } else {
                (tailwind::TEAL.c900, tailwind::BLUE.c200)
            }
        } else if !alt {
            (tailwind::ZINC.c800, material::BLUE_GRAY.c200)
        } else {
            (tailwind::GRAY.c800, material::GRAY.c400)
        };

        let vert_lay = vertical![==1, >=0, ==1].split(area);

        let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [highlight_area, poster_area, _, description_area, _] =
            horizontal![==2, ==movie_width, ==2, >=0, ==2].areas(vert_lay[1]);

        let block = Block::new().bg(background).fg(text);
        frame.render_widget(&block, area);

        let name = ellipsize_string(&movie.name, description_area.width as usize - 11);

        let rating = movie.get_user_rating();
        let rating_color = if rating >= 9.0 {
            tailwind::SKY.c400
        } else if rating >= 8.0 {
            tailwind::GREEN.c500
        } else if rating >= 7.5 {
            tailwind::LIME.c400
        } else if rating >= 7.0 {
            material::LIME.c400
        } else if rating >= 6.0 {
            tailwind::AMBER.c300
        } else {
            material::DEEP_ORANGE.c300
        };
        let text = text![
            name.bold() + " ".into() + movie.year.clone().italic(),
            format!("{:.1}", rating).set_style(rating_color).bold(),
            "",
            movie.tagline.to_string(),
        ];

        frame.render_widget(
            text,
            add_padding(description_area, Padding::top(poster_area.height - 4)),
        );

        let unfocused_rating_color = if rating >= 9.0 {
            tailwind::SKY.c600
        } else if rating >= 8.0 {
            tailwind::GREEN.c700
        } else if rating >= 7.5 {
            tailwind::LIME.c700
        } else if rating >= 7.0 {
            material::LIME.c700
        } else if rating >= 6.0 {
            tailwind::YELLOW.c600
        } else {
            tailwind::ORANGE.c800
        };
        if selected {
            frame.render_widget(
                text![line!["▐"]; highlight_area.height as usize].fg(if tab_selected {
                    rating_color
                } else {
                    unfocused_rating_color
                }),
                highlight_area,
            );
        }

        if self.redraw_images < 1 {
            self.drawing_images |= !self.image_renderer.draw_image(
                self.movies[movie_id].id.tmdb,
                false,
                poster_area,
                frame,
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), poster_area);
        }
    }

    pub fn draw_movie_description(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) {
        const TABS: [&str; 2] = ["Overview", "Review"];
        const TABS_COUNT: usize = TABS.len();
        key_event_handler.bind_horizontal((Some(1), None), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        main_screen.movies_description_selected_tab = main_screen
                            .movies_description_selected_tab
                            .add(1)
                            .min(TABS_COUNT - 1);
                    }
                    crate::key_event_handler::Data::Direction(false) => {
                        main_screen.movies_description_selected_tab = main_screen
                            .movies_description_selected_tab
                            .checked_sub(1)
                            .unwrap_or(0);
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_tab((Some(1), None), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        main_screen.tab += 1;
                        if main_screen.tab > 1 {
                            main_screen.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false) => {
                        main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }
            }
        });

        let description_selected = self.tab == 1;
        let movie = if self.movies.is_empty() {
            None
        } else {
            Some(&self.movies[self.movies_list_selected_item])
        };

        let inner = Block::new()
            .padding(Padding {
                left: 2,
                right: 2,
                top: 1,
                bottom: 1,
            })
            .inner(area);

        let backdrop_height = ((inner.width - 4) as f32 * 9.0 / 32.0).ceil() as u16;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==9, >=1].areas(inner);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        if let Some(movie) = movie {
            let [_, title_area, _, ratings_area, _, tabs_area] =
                vertical![==1, ==2, ==1, ==2, ==1, ==2].areas(title_area);

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

            const BGS: [Color; 2] = [material::GREEN.c600, material::LIGHT_BLUE.c600];
            const FGS: [Color; 2] = [material::BLUE.c100, material::YELLOW.c100];
            const _BGS: [Color; 2] = [material::TEAL.c800, material::INDIGO.c600];
            const _FGS: [Color; 2] = [material::BLUE_GRAY.c200, material::BLUE_GRAY.c200];
            let mut tabs = TABS
                .iter()
                .flat_map(|x| {
                    [
                        " ".into(),
                        Span::from(format!(" {} ", *x)).style(Style::new().fg(material::GRAY.c600)),
                    ]
                })
                .collect::<Vec<_>>();
            tabs[self.movies_description_selected_tab * 2 + 1] = tabs
                [self.movies_description_selected_tab * 2 + 1]
                .clone()
                .style(if description_selected {
                    Style::new()
                        .fg(FGS[self.movies_description_selected_tab])
                        .bg(BGS[self.movies_description_selected_tab])
                } else {
                    Style::new()
                        .fg(_FGS[self.movies_description_selected_tab])
                        .bg(_BGS[self.movies_description_selected_tab])
                })
                .bold();
            frame.render_widget(
                Text::from(vec![
                    Line::from(tabs),
                    Line::from("🮂".repeat(title_area.width as usize)).style(
                        if description_selected {
                            Style::new().fg(BGS[self.movies_description_selected_tab])
                        } else {
                            Style::new().fg(_BGS[self.movies_description_selected_tab])
                        },
                    ),
                ]),
                tabs_area,
            );

            let description = match self.movies_description_selected_tab {
                0 => Paragraph::new(movie.overview.as_str())
                    .wrap(Wrap { trim: true })
                    .centered(),
                1 => Paragraph::new(
                    movie
                        .plays
                        .iter()
                        .map(|x| {
                            Line::from(format!("{}: {:.1}", x.0.format("%d/%m/%Y %H:%M"), x.1))
                        })
                        .collect::<Vec<_>>(),
                )
                .wrap(Wrap { trim: true }),
                _ => Paragraph::new("NA").wrap(Wrap { trim: true }).centered(),
            };
            frame.render_widget(description, description_area);
        }

        if self.redraw_images < 1 && movie.is_some() {
            self.drawing_images |= !self.image_renderer.draw_image(
                self.current_movie().id.tmdb,
                true,
                backdrop_area,
                frame,
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), backdrop_area);
        }
    }

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

    pub fn goto_index(&mut self, index: usize) {
        let index = index.min(self.movies.len());

        self.movies_list_selected_item = index;
        self.movies_list_scroll_pos = self
            .movies_list_scroll_pos
            .min(self.movies_list_selected_item);
        if self.movies_list_selected_item - self.movies_list_scroll_pos
            >= self.movies_list_visible_items
        {
            self.movies_list_scroll_pos =
                self.movies_list_selected_item - self.movies_list_visible_items + 1;
        }
    }

    pub fn current_movie(&self) -> &Movie {
        &self.movies[self.movies_list_selected_item]
    }

    fn filter_movies(&mut self, movies: &[Movie]) {
        let search_text = &self.search_input.lines()[0];
        if search_text.is_empty() {
            self.movies = movies.iter().map(|x| x.clone()).collect();
            return;
        }

        let mut conf = Config::DEFAULT;
        conf.prefer_prefix = true;
        let mut matcher = Matcher::new(conf);
        let pattern = Atom::parse(
            search_text,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Never,
        );
        let mut scores = vec![];
        for movie in movies {
            if let Some(score) = pattern.score(
                nucleo_matcher::Utf32Str::Ascii(
                    (movie.name.clone() + " " + &movie.year)
                        .to_string()
                        .as_bytes(),
                ),
                &mut matcher,
            ) {
                scores.push((score, movie));
            }
        }

        self.movies = if let Sort::Relevance = self.sort {
            scores.sort_by_key(|x| x.0);
            scores.reverse();
            scores.iter().map(|&(_, movie)| movie.clone()).collect()
        } else {
            scores.iter().map(|&(_, movie)| movie.clone()).collect()
        }
    }

    fn sort_movies(&mut self) {
        match self.sort {
            Sort::UserRating => {
                self.movies.sort_by(|x, y| {
                    x.get_user_rating()
                        .partial_cmp(&y.get_user_rating())
                        .unwrap()
                });
                if !self.sort_ascending {
                    self.movies.reverse();
                }
            }
            Sort::Rating => {
                self.movies.sort_by(MainScreen::cmp_ratings);
                if !self.sort_ascending {
                    self.movies.reverse();
                }
            }
            Sort::Name => {
                self.movies.sort_by_key(|x| x.name.clone());
                if self.sort_ascending {
                    self.movies.reverse();
                }
            }
            Sort::ReleaseDate => {
                self.movies.sort_by_key(|x| x.year.clone());
                if self.sort_ascending {
                    self.movies.reverse();
                }
            }
            _ => (),
        }
    }

    pub fn filter_sort_movies(&mut self, movies: &[Movie]) {
        self.filter_movies(movies);

        self.image_renderer
            .preload_images(&movies.iter().map(|x| x.id.tmdb).collect::<Vec<_>>());
        if let Sort::AddedDate = self.sort {
            return;
        } else if let Sort::Relevance = self.sort {
            return;
        }

        self.movies_list_selected_item = 0;
        self.movies_list_scroll_pos = 0;
        self.sort_movies();
    }

    fn cmp_ratings(a: &Movie, b: &Movie) -> Ordering {
        let mut rating_a: f64 = 0.0;
        let mut rating_b: f64 = 0.0;

        for i in (0..a.ratings.len()).rev() {
            if let Rating::IMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::IMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::Trakt(r_a, c_a) = a.ratings[i] {
                if let Rating::Trakt(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::TMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::TMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
        }

        rating_a.partial_cmp(&rating_b).unwrap()
    }
}
