use chrono::{DateTime, Datelike, Utc};
use itertools::{Itertools, izip};
use log::info;
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Layout, Offset, Rect, Size},
    macros::{constraint, line, span, text, vertical},
    style::{
        Color, Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    text::{Line, Span, Text},
    widgets::{Fill, Padding, Widget},
};
use ratatui_image::sliced::SignedPosition;

use crate::{
    helpers,
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{self, KeyEventHandler},
    screens::{
        Screens,
        main_screen::{self, MainScreen},
    },
    types::Movie,
    widgets::{self, ScrollGallery, ScrolledList},
};

#[derive(Default)]
pub struct PlaysTab {
    scroll_pos:        usize,
    alignment_bottom:  bool,
    num_visible_items: usize,
}

const DESCRIPTION_TABS: [&str; 3] = ["Overview", "Plays", "Credits"];
#[derive(Default)]
pub struct MoviesDescription {
    pub available_tabs: Vec<usize>,
    pub selected_tab:   usize,

    pub overview_scroll: usize,
    pub plays_tab:       PlaysTab,
    pub credits_list:    ScrolledList,
}

impl MainScreen {
    pub fn render_movie_description(
        &mut self,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
    ) {
        key_event_handler.bind_tab((Some(1), None), "Change focus".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.tab += 1;
                        if main_screen.tab > 1 {
                            main_screen.tab = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }

                main_screen.item = 0;
            }
        });

        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 1;
                    main_screen.item = 0;
                }
            },
        );

        let description_selected = self.tab == 1;
        let movie = if self.filtered_movies.is_empty() {
            None
        } else {
            self.current_movie().cloned()
        };

        let inner = helpers::add_padding(area, Padding::proportional(1));
        let backdrop_height = (inner.width as f32 * 9.0 / 16.0).ceil() as u16 >> 1;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==8, >=1].areas(inner);

        frame.render_widget(Fill::new(" ").bg(tailwind::GRAY.c950), backdrop_area);
        if let Some(movie) = movie {
            let [title_area, ratings_area, _, tabs_area] =
                vertical![==3, ==2, ==1, ==2].areas(title_area);

            let mut name = movie.title.clone();
            name = helpers::ellipsize_string(&name, title_area.width as usize);

            let rating = self
                .watched
                .borrow()
                .get(&movie.id)
                .map(|x| x.get_user_rating());
            let user_rating_widget_bg = rating
                .as_ref()
                .map(|rating| {
                    if *rating >= 9.0 {
                        tailwind::SKY.c400
                    } else if *rating >= 8.0 {
                        tailwind::GREEN.c500
                    } else if *rating >= 7.5 {
                        tailwind::LIME.c400
                    } else if *rating >= 7.0 {
                        material::AMBER.c400
                    } else if *rating >= 6.0 {
                        tailwind::ORANGE.c500
                    } else {
                        material::RED.c400
                    }
                })
                .unwrap_or(tailwind::BLACK);
            let user_rating_widget_fg = rating
                .as_ref()
                .map(|rating| {
                    if *rating >= 7.0 {
                        tailwind::STONE.c950
                    } else {
                        tailwind::STONE.c200
                    }
                })
                .unwrap_or(tailwind::GRAY.c300);

            frame.render_widget(
                line![
                    span!("     "),
                    name.clone().bold(),
                    span!(" "),
                    movie.release_date.year().to_string().italic()
                ]
                .centered(),
                title_area.resize(Size::new(title_area.width, 1)),
            );
            frame.render_widget(
                line!(
                    span!("").fg(user_rating_widget_bg),
                    if let Some(rating) = rating {
                        format!("{rating:.1}")
                    } else {
                        "Not watched".into()
                    }
                    .bold()
                    .fg(user_rating_widget_fg)
                    .bg(user_rating_widget_bg),
                    span!("").fg(user_rating_widget_bg)
                )
                .centered(),
                helpers::add_padding(title_area, Padding::top(1))
                    .resize(Size::new(title_area.width, 1)), // .centered(constraint!(==5), constraint!(==1)),
            );
            if movie.released {
                self.draw_ratings(&movie, frame, ratings_area);
            } else {
                if movie.release_date > Default::default() {
                    frame.render_widget(
                        text![
                            line!(
                                span!("").fg(user_rating_widget_bg),
                                "Not released"
                                    .bold()
                                    .fg(user_rating_widget_fg)
                                    .bg(user_rating_widget_bg),
                                span!("").fg(user_rating_widget_bg)
                            ),
                            line!(
                                "Releases: ".bold().fg(tailwind::SKY.c400),
                                span!(movie.release_date.format("%A, %-d %B, %C%y"))
                                    .italic()
                                    .fg(tailwind::INDIGO.c200),
                            )
                        ]
                        .centered(),
                        ratings_area,
                    );
                } else {
                    frame.render_widget(
                        line!(
                            span!("").fg(user_rating_widget_bg),
                            "Not released"
                                .bold()
                                .fg(user_rating_widget_fg)
                                .bg(user_rating_widget_bg),
                            span!("").fg(user_rating_widget_bg)
                        )
                        .centered(),
                        helpers::add_padding(ratings_area, Padding::top(1)),
                    );
                }
            }

            self.movies_description.available_tabs = DESCRIPTION_TABS
                .iter()
                .enumerate()
                .filter_map(|(i, x)| {
                    if *x == "Overview" {
                        Some(i)
                    } else if *x == "Plays" {
                        self.watched.borrow().contains_key(&movie.id).then_some(i)
                    } else if *x == "Credits" {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect_vec();
            let num_available_tabs = self.movies_description.available_tabs.len();
            if self.movies_description.selected_tab >= num_available_tabs {
                self.movies_description.selected_tab = 0;
            }

            if num_available_tabs > 1 {
                key_event_handler.bind_horizontal(
                    (Some(1), None),
                    "Change tab".into(),
                    move |app, data| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Direction(true, _) => {
                                    main_screen.movies_description.selected_tab =
                                        (main_screen.movies_description.selected_tab + 1)
                                            .min(num_available_tabs - 1);
                                }
                                key_event_handler::Data::Direction(false, _) => {
                                    main_screen.movies_description.selected_tab = main_screen
                                        .movies_description
                                        .selected_tab
                                        .saturating_sub(1);
                                }
                                _ => (),
                            }
                        }
                    },
                );
            }

            const BGS: [Color; DESCRIPTION_TABS.len()] = [
                material::GREEN.c600,
                material::LIGHT_BLUE.c600,
                tailwind::RED.c500,
            ];
            const FGS: [Color; DESCRIPTION_TABS.len()] = [
                material::BLUE.c100,
                material::YELLOW.c100,
                tailwind::GRAY.c400,
            ];
            const _BGS: [Color; DESCRIPTION_TABS.len()] = [
                material::TEAL.c800,
                material::INDIGO.c600,
                tailwind::RED.c700,
            ];
            const _FGS: [Color; DESCRIPTION_TABS.len()] = [
                material::BLUE_GRAY.c200,
                material::BLUE_GRAY.c200,
                tailwind::BLUE.c200,
            ];
            let mut tabs = self
                .movies_description
                .available_tabs
                .iter()
                .enumerate()
                .flat_map(|(i, x)| {
                    [
                        format!(" {} ", DESCRIPTION_TABS[*x])
                            .fg(if description_selected { FGS[*x] } else { _FGS[*x] })
                            .bg(if description_selected { BGS[*x] } else { _BGS[*x] })
                            .add_modifier(if i == self.movies_description.selected_tab {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                        " ".into(),
                    ]
                })
                .dropping_back(1)
                .collect_vec();
            let mut mouse_area = tabs_area;
            for (i, tab) in tabs.iter_mut().enumerate() {
                if i & 1 == 1 {
                    mouse_area = mouse_area.offset(Offset { x: 1, y: 0 });
                    continue;
                }
                mouse_area = mouse_area.resize(Size {
                    width:  tab.width() as u16,
                    height: 1,
                });

                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 1;
                            main_screen.item = 0;
                            main_screen.movies_description.selected_tab = i / 2;
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset {
                    x: mouse_area.width as i32,
                    y: 0,
                });
            }
            frame.render_widget(
                text![
                    tabs,
                    line!("🮂".repeat(title_area.width as usize)).fg(if description_selected {
                        BGS[self.movies_description.selected_tab]
                    } else {
                        _BGS[self.movies_description.selected_tab]
                    }),
                ],
                tabs_area,
            );

            match DESCRIPTION_TABS
                [self.movies_description.available_tabs[self.movies_description.selected_tab]]
            {
                "Overview" => {
                    frame.render_widget(Fill::new(" ").bg(tailwind::SLATE.c900), description_area);

                    let mut overview_lines =
                        helpers::wrap_text(&movie.overview, description_area.width as usize)
                            .into_iter()
                            .map(|x| line!(x))
                            .collect_vec();

                    let genres_widgets = movie
                        .genres
                        .iter()
                        .sorted_by(|a, b| a.len().cmp(&b.len()))
                        .map(|genre| {
                            vec![
                                span!("").fg(tailwind::SLATE.c100),
                                span!(genre)
                                    .bold()
                                    .fg(tailwind::SLATE.c950)
                                    .bg(tailwind::SLATE.c100),
                                span!("").fg(tailwind::SLATE.c100),
                            ]
                        });
                    let mut genres_lines = vec![];
                    for widget in genres_widgets {
                        let mut line: Vec<Span<'_>> = genres_lines.pop().unwrap_or(vec![]);

                        if widget.iter().fold(0, |a, b| a + b.content.chars().count())
                            + line.iter().fold(0, |a, b| a + b.content.chars().count())
                            + if !line.is_empty() { 1 } else { 0 }
                            <= description_area.width as usize
                        {
                            if !line.is_empty() {
                                line.push(" ".into());
                            }

                            line.extend(widget);
                            genres_lines.push(line);
                        } else {
                            genres_lines.push(line);
                            genres_lines.push(widget);
                        }
                    }

                    overview_lines.push(line!());
                    overview_lines.extend(
                        genres_lines
                            .into_iter()
                            .map(|x| Line::from_iter(x).centered()),
                    );
                    self.movies_description.overview_scroll =
                        self.movies_description.overview_scroll.min(
                            overview_lines
                                .len()
                                .saturating_sub(description_area.height as usize),
                        );
                    let text = Text::from_iter(
                        overview_lines.split_off(self.movies_description.overview_scroll),
                    );

                    frame.render_widget(text, description_area);

                    key_event_handler.bind_vertical(
                        (Some(1), Some(self.movies_description.selected_tab << 9)),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Screens::MainScreen(main_screen)) =
                                app.drawer.current_screen.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Direction(false, _) => {
                                        main_screen.movies_description.overview_scroll =
                                            main_screen
                                                .movies_description
                                                .overview_scroll
                                                .saturating_sub(1);
                                    }
                                    key_event_handler::Data::Direction(true, _) => {
                                        main_screen.movies_description.overview_scroll += 1;
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                "Plays" => self.draw_plays_tab(description_area, &movie, frame, key_event_handler),
                "Credits" => self.draw_credits_tab(
                    description_area,
                    &movie,
                    image_renderer,
                    frame,
                    key_event_handler,
                ),
                _ => (),
            };

            let mut cell = Cell::new(" ");
            cell.set_style(Style::new().bg(tailwind::GRAY.c950));
            let mut image_buffer = Buffer::filled(backdrop_area, cell);
            image_renderer.draw_image(
                ImageID::Movie(movie.id, movie.override_backdrop.clone(), true),
                false,
                None,
                &mut image_buffer,
            );
            frame.buffer_mut().merge(&image_buffer);
        }
    }

    fn draw_ratings(&self, movie: &Movie, frame: &mut Frame, area: Rect) {
        const IMDB_COLORS: (Color, Color, Color) = (
            Color::Rgb(245, 197, 24),
            Color::Black,
            Color::Rgb(250, 225, 120),
        );
        const LETTERBOXD_COLORS: (Color, Color, Color) = (
            Color::Rgb(0, 192, 48),
            Color::Black,
            Color::Rgb(115, 226, 122),
        );
        const TRAKT_COLORS: (Color, Color, Color) = (
            Color::Rgb(165, 61, 185),
            Color::White,
            Color::Rgb(230, 140, 245),
        );
        const TMDB_COLORS: (Color, Color, Color) = (
            Color::Rgb(42, 187, 209),
            Color::Black,
            Color::Rgb(140, 205, 215),
        );
        const POPCORN_COLORS: (Color, Color, Color) = (
            Color::Rgb(255, 114, 33),
            Color::White,
            Color::Rgb(242, 165, 121),
        );
        const TOMATOES_COLORS: (Color, Color, Color) = (
            Color::Rgb(216, 44, 60),
            Color::White,
            Color::Rgb(247, 100, 103),
        );

        let mut ratings = vec![];
        if movie.external_ratings.imdb.0 > 0.0 {
            ratings.push(("imdb", format!("{:.1}", movie.external_ratings.imdb.0)));
        }
        if movie.external_ratings.letterboxd.0 > 0.0 {
            ratings.push((
                "letterboxd",
                format!("{:.1}", movie.external_ratings.letterboxd.0),
            ));
        }
        if movie.external_ratings.trakt.0 > 0 {
            ratings.push(("trakt", movie.external_ratings.trakt.0.to_string()));
        }
        if movie.external_ratings.tmdb.0 > 0.0 {
            ratings.push(("tmdb", format!("{:.1}", movie.external_ratings.tmdb.0)));
        }
        if movie.external_ratings.popcorn.0 > 0 {
            ratings.push(("popcorn", movie.external_ratings.popcorn.0.to_string()));
        }
        if movie.external_ratings.tomatoes.0 > 0 {
            ratings.push(("tomatoes", movie.external_ratings.tomatoes.0.to_string()));
        }

        if ratings.is_empty() {
            frame.render_widget(line!("NA").centered(), area);

            return;
        }

        let widget_areas = Layout::horizontal(vec![constraint!(==5); ratings.len()])
            .flex(ratatui::layout::Flex::SpaceEvenly)
            .split(helpers::add_padding(area, Padding::top(1)));
        let mut widgets = vec![];
        let mut links = vec![];
        let mut labels = line!();
        for (name, rating) in ratings {
            let (bg, fg) = if name == "imdb" {
                labels.push_span(span!("IMDB").fg(IMDB_COLORS.2));
                links.push("".to_string());

                (IMDB_COLORS.0, IMDB_COLORS.1)
            } else if name == "letterboxd" {
                labels.push_span(span!("Letterboxd").fg(LETTERBOXD_COLORS.2));
                links.push("".to_string());

                (LETTERBOXD_COLORS.0, LETTERBOXD_COLORS.1)
            } else if name == "trakt" {
                labels.push_span(span!("Trakt").fg(TRAKT_COLORS.2));
                links.push("".to_string());

                (TRAKT_COLORS.0, TRAKT_COLORS.1)
            } else if name == "tmdb" {
                labels.push_span(span!("TMDB").fg(TMDB_COLORS.2));
                links.push(format!("https://www.themoviedb.org/movie/{}", movie.id));

                (TMDB_COLORS.0, TMDB_COLORS.1)
            } else if name == "popcorn" {
                labels.push_span(span!("Popcorn").fg(POPCORN_COLORS.2));
                links.push("".to_string());

                (POPCORN_COLORS.0, POPCORN_COLORS.1)
            } else if name == "tomatoes" {
                labels.push_span(span!("Tomatoes").fg(TOMATOES_COLORS.2));
                links.push("".to_string());

                (TOMATOES_COLORS.0, TOMATOES_COLORS.1)
            } else {
                continue;
            };

            widgets.push(line!["".fg(bg), rating.bg(bg).fg(fg).bold(), "".fg(bg)]);
        }

        for ((widget, label, link), &area) in
            izip!(widgets.into_iter(), labels, links).zip(widget_areas.iter())
        {
            frame.render_widget(label, area.offset(Offset::new(0, -1)));
            frame.render_widget(
                widgets::Hyperlink {
                    text: widget.into(),
                    url:  link,
                },
                area,
            );
        }
    }

    fn draw_plays_tab(
        &mut self,
        area: Rect,
        movie: &Movie,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) {
        let movie_plays = &self.watched.borrow()[&movie.id].history;
        let tab_selected = self.tab == 1;
        let num_plays = movie_plays.len();
        let num_visible_plays = area.height as usize / 3;
        let partially_visible_play_height = area.height as usize - num_visible_plays * 3;
        let render_partially_visible_play = partially_visible_play_height > 0;
        self.movies_description.plays_tab.num_visible_items =
            num_visible_plays + if render_partially_visible_play { 1 } else { 0 };

        if num_plays > num_visible_plays {
            key_event_handler.bind_vertical(
                (Some(1), Some(self.movies_description.selected_tab << 9)),
                "Scroll".into(),
                move |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Direction(false, _) => {
                                if main_screen.movies_description.plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description.plays_tab.alignment_bottom =
                                        false;
                                } else {
                                    main_screen.movies_description.plays_tab.scroll_pos =
                                        main_screen
                                            .movies_description
                                            .plays_tab
                                            .scroll_pos
                                            .saturating_sub(1);
                                }
                            }
                            key_event_handler::Data::Direction(true, _) => {
                                if !main_screen.movies_description.plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description.plays_tab.alignment_bottom =
                                        true;
                                } else {
                                    main_screen.movies_description.plays_tab.scroll_pos += 1;
                                }
                            }
                            _ => (),
                        }
                    }
                },
            );

            self.movies_description.plays_tab.scroll_pos =
                self.movies_description.plays_tab.scroll_pos.min(
                    num_plays.saturating_sub(self.movies_description.plays_tab.num_visible_items),
                );
            self.movies_description.plays_tab.alignment_bottom &= render_partially_visible_play;
        } else {
            self.movies_description.plays_tab.scroll_pos = 0;
            self.movies_description.plays_tab.alignment_bottom = false;
        }

        frame.render_widget(Fill::new(" ").bg(tailwind::SLATE.c900), area);

        let mut remaining_area = helpers::add_padding(area, Padding::left(1));
        for i in 0..self.movies_description.plays_tab.num_visible_items {
            let [area, remaining] = if render_partially_visible_play
                && i == (!self.movies_description.plays_tab.alignment_bottom as usize
                    * (self.movies_description.plays_tab.num_visible_items - 1))
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else {
                vertical![==3, >= 0]
            }
            .areas(remaining_area);

            let index = self.movies_description.plays_tab.scroll_pos + i;
            if index < num_plays {
                let partially_visible = area.height < 3;
                let play = &movie_plays[num_plays - 1 - index];

                let alternate = i & 1 == 1;
                let latest = index == 0;
                let last = index == num_plays - 1;

                frame.render_widget(
                    Fill::new(" ").bg(if latest {
                        if tab_selected {
                            tailwind::ZINC.c600
                        } else {
                            tailwind::ZINC.c700
                        }
                    } else if !alternate {
                        if tab_selected {
                            tailwind::GRAY.c600
                        } else {
                            tailwind::GRAY.c700
                        }
                    } else {
                        if tab_selected {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    }),
                    helpers::add_padding(area, Padding::left(2)),
                );

                let areas = (0..area.height)
                    .map(|i| Rect::new(area.x, area.y + i, area.width, 1))
                    .collect_vec();

                let rating_color = if play.rating >= 9.0 {
                    tailwind::SKY.c400
                } else if play.rating >= 8.0 {
                    tailwind::GREEN.c500
                } else if play.rating >= 7.5 {
                    tailwind::LIME.c400
                } else if play.rating >= 7.0 {
                    material::AMBER.c400
                } else if play.rating >= 6.0 {
                    material::DEEP_ORANGE.c300
                } else {
                    material::RED.c400
                };

                let local_date = play.date.with_timezone(&chrono::Local);
                for i in 0..area.height {
                    let index = if partially_visible {
                        if self.movies_description.plays_tab.alignment_bottom {
                            i + 3 - area.height
                        } else {
                            i
                        }
                    } else {
                        i
                    };
                    match index {
                        0 =>
                            if !latest {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            } else {
                                frame.render_widget(
                                    span!("▔".repeat(area.width as usize)).fg(if tab_selected {
                                        tailwind::ZINC.c500
                                    } else {
                                        tailwind::ZINC.c600
                                    }),
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
                                );
                            },
                        1 => {
                            frame.render_widget(
                                span!("●").fg(if latest {
                                    if tab_selected {
                                        material::YELLOW.c800
                                    } else {
                                        material::CYAN.c500
                                    }
                                } else {
                                    if tab_selected {
                                        material::CYAN.c500
                                    } else {
                                        material::CYAN.c700
                                    }
                                }),
                                areas[i as usize],
                            );
                            frame.render_widget(
                                line![
                                    format!("{:.1}", play.rating).fg(rating_color).add_modifier(
                                        if latest { Modifier::BOLD } else { Modifier::empty() }
                                    ),
                                    span!(" @ "),
                                    if play.date == DateTime::<Utc>::default() {
                                        "Unknown".into()
                                    } else {
                                        local_date.format("%d/%m/%Y %H:%M").to_string()
                                    }
                                    .fg(if latest {
                                        if tab_selected {
                                            material::YELLOW.c700
                                        } else {
                                            material::CYAN.c600
                                        }
                                    } else {
                                        if tab_selected {
                                            material::CYAN.c500
                                        } else {
                                            material::CYAN.c700
                                        }
                                    }),
                                ],
                                helpers::add_padding(areas[i as usize], Padding::left(4)),
                            );
                        }
                        2 => {
                            if !last {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            }
                            if latest {
                                frame.render_widget(
                                    span!("▁".repeat(area.width as usize)).fg(if tab_selected {
                                        tailwind::ZINC.c500
                                    } else {
                                        tailwind::ZINC.c600
                                    }),
                                    helpers::add_padding(areas[i as usize], Padding::left(2)),
                                );
                            }
                        }
                        _ => (),
                    }
                }
            }

            remaining_area = remaining;
        }
    }

    fn draw_credits_tab(
        &mut self,
        area: Rect,
        movie: &Movie,
        image_renderer: &mut RatatuiImage,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) {
        let num_cast = movie.credits.cast.len();

        key_event_handler.bind_vertical(
            (Some(1), Some(self.movies_description.selected_tab << 9)),
            "Scroll".into(),
            move |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(true, _) => {
                            if main_screen
                                .movies_description
                                .credits_list
                                .alignment_opposite
                            {
                                main_screen
                                    .movies_description
                                    .credits_list
                                    .scroll(true, num_cast);
                            } else {
                                main_screen.movies_description.credits_list.selected_index =
                                    (main_screen.movies_description.credits_list.scroll_pos
                                        + main_screen
                                            .movies_description
                                            .credits_list
                                            .num_visible_items
                                        - 1);
                            }
                        }
                        key_event_handler::Data::Direction(false, _) => {
                            if !main_screen
                                .movies_description
                                .credits_list
                                .alignment_opposite
                            {
                                main_screen
                                    .movies_description
                                    .credits_list
                                    .scroll(false, num_cast);
                            } else {
                                main_screen.movies_description.credits_list.selected_index =
                                    main_screen.movies_description.credits_list.scroll_pos;
                            }
                        }
                        _ => (),
                    }
                }
            },
        );

        let scrollbar_area = area
            .offset(Offset::new(area.width as i32 - 1, 0))
            .resize(Size::new(1, area.height));
        self.movies_description.credits_list.render(
            num_cast,
            helpers::add_padding(area, Padding::right(1)),
            scrollbar_area,
            frame,
            key_event_handler,
            |buffer,
             num_hidden_rows,
             buffer_negative_offset,
             align_opposite,
             index,
             _selected,
             key_event_handler| {
                let alternate = index & 1 == 1;
                let buffer_area = *buffer.area();
                let visible_area = helpers::add_padding(
                    buffer_area,
                    if align_opposite {
                        Padding::left(num_hidden_rows)
                    } else {
                        Padding::right(num_hidden_rows)
                    },
                );
                let input_area = visible_area.offset(Offset {
                    x: buffer_negative_offset,
                    y: 0,
                });

                info!("{num_hidden_rows} {buffer_negative_offset} {buffer_area:?} {visible_area:?} {input_area:?}");
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    input_area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen
                                .movies_description
                                .credits_list
                                .goto_index(index, false, num_cast);
                        }
                    },
                );
                for position in buffer_area.positions() {
                    buffer[position]
                        .set_symbol(" ")
                        .set_style(Style::new().bg(if alternate {
                            tailwind::SLATE.c950
                        } else {
                            tailwind::GRAY.c900
                        }));
                }

                let mut cell = Cell::new(" ");
                cell.set_style(Style::new().bg(tailwind::GRAY.c950));
                let mut image_buffer = Buffer::filled(
                    helpers::add_padding(buffer_area, Padding::proportional(1))
                        .resize(Size::new(buffer_area.width - 4, 9))
                        .intersection(visible_area).offset(Offset {
                            x: buffer_negative_offset,
                            y: 0,
                        }),
                    cell,
                );
                image_renderer.draw_image(
                    ImageID::Person(movie.credits.cast[index].id),
                    false,
                    if num_hidden_rows > 0 {
                        Some(SignedPosition {
                            x: if align_opposite {
                                -(num_hidden_rows as i16 - 2)
                            } else {
                                0
                            },
                            y: 0,
                        })
                    } else {
                        None
                    },
                    &mut image_buffer,
                );
                image_buffer.area = image_buffer.area.offset(Offset {
                    x: -buffer_negative_offset,
                    y: 0,
                });
                buffer.merge(&image_buffer);

                line![helpers::ellipsize_string(
                    &self.persons.borrow()[&movie.credits.cast[index].id].name,
                    buffer_area.width as usize
                )]
                .centered()
                .render(
                    helpers::add_padding(buffer_area, Padding::top(buffer_area.height - 2)),
                    buffer,
                );
                line![helpers::ellipsize_string(
                    &movie.credits.cast[index].job_or_character,
                    buffer_area.width as usize
                )]
                .centered()
                .render(
                    helpers::add_padding(buffer_area, Padding::top(buffer_area.height - 1)),
                    buffer,
                );
            },
        );
    }
}
