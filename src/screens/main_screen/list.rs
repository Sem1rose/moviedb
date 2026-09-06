use chrono::Datelike;
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    crossterm::event::KeyModifiers,
    layout::{Offset, Position, Rect, Size},
    macros::{horizontal, line, span},
    style::{
        Style, Styled, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::Text,
    widgets::{Padding, Widget},
};
use ratatui_image::sliced::SignedPosition;

use crate::{
    helpers,
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{self, KeyEventHandler},
    screens::{Screens, main_screen::MainScreen},
};

pub const MOVIE_WIDGET_HEIGHT: usize = 11;
const LIST_POSTER_WIDTH: usize = (MOVIE_WIDGET_HEIGHT - 2) * 4 / 3;

impl MainScreen {
    pub fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
    ) {
        let num_items = self.filtered_movies.len();

        if !self.filtered_movies.is_empty() {
            let num_visible_items = self.movies_list.num_visible_items;

            key_event_handler.bind_tab((Some(0), None), "Change focus".into(), |app, data| {
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
                }
            });

            key_event_handler.bind_key((Some(0), None), "gg", "Jump to top".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(0);
                }
            });
            key_event_handler.bind_key((Some(0), None), 'G', "Jump to bottom".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(-1);
                }
            });

            key_event_handler.bind_vertical((Some(0), None), "Scroll".into(), move |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if let key_event_handler::Data::Direction(direction, modifiers) = data {
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            if direction {
                                main_screen.goto_index(
                                    (main_screen.movies_list.selected_index
                                        + num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.goto_index(
                                    main_screen
                                        .movies_list
                                        .selected_index
                                        .saturating_sub(num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            }
                        } else {
                            main_screen.movies_list.scroll(direction, num_items);
                        }
                    }
                }
            });
        }

        if self.movies_list.selected_index >= num_items {
            self.movies_list.selected_index = num_items.saturating_sub(1);
            self.movies_list.scroll_pos = self
                .movies_list
                .selected_index
                .saturating_sub(self.movies_list.num_visible_items.saturating_sub(1));
        }

        let [movies_area, scrollbar_area] = horizontal![>=0, ==1].areas(area);

        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            scrollbar_area.resize(Size::new(1, 1)),
            move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if main_screen.movies_list.alignment_opposite
                        && main_screen.movies_list.partially_visible
                    {
                        main_screen.movies_list.alignment_opposite = false;
                    } else if main_screen.movies_list.scroll_pos > 0 {
                        main_screen.movies_list.scroll_pos -= 1;
                    }
                }
            },
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            scrollbar_area
                .resize(Size::new(1, 1))
                .offset(Offset::new(0, scrollbar_area.height as i32 - 1)),
            move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if !main_screen.movies_list.alignment_opposite
                        && main_screen.movies_list.partially_visible
                    {
                        main_screen.movies_list.alignment_opposite = true;
                    } else if main_screen.movies_list.scroll_pos
                        < num_items.saturating_sub(main_screen.movies_list.num_visible_items)
                    {
                        main_screen.movies_list.scroll_pos += 1;
                    }
                }
            },
        );

        self.movies_list.update_for_area(area, num_items);
        self.movies_list.render_without_area_update(
            num_items,
            movies_area,
            scrollbar_area,
            frame,
            key_event_handler,
            |buffer,
             num_hidden_rows,
             buffer_y_negative_offset,
             align_bottom,
             index,
             selected,
             key_event_handler| {
                self.draw_movie_widget(
                    index,
                    buffer,
                    selected,
                    num_hidden_rows,
                    buffer_y_negative_offset,
                    align_bottom,
                    image_renderer,
                    key_event_handler,
                );
            },
        );
    }

    fn draw_movie_widget(
        &self,
        movie_index: usize,
        buffer: &mut Buffer,
        selected: bool,
        num_hidden_rows: u16,
        buffer_y_negative_offset: i32,
        align_bottom: bool,
        image_renderer: &mut RatatuiImage,
        key_event_handler: &mut KeyEventHandler,
    ) {
        let buffer_area = *buffer.area();
        let visible_area = helpers::add_padding(
            buffer_area,
            if align_bottom {
                Padding::top(num_hidden_rows)
            } else {
                Padding::bottom(num_hidden_rows)
            },
        );
        let input_area = visible_area.offset(Offset {
            x: 0,
            y: buffer_y_negative_offset,
        });

        let alt = movie_index & 1 == 1;
        let tab_selected = self.tab == 0;
        let movie = &self.filtered_movies[movie_index];

        let num_items = self.filtered_movies.len();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            input_area,
            move |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 0;
                    main_screen.item = 0;

                    main_screen
                        .movies_list
                        .goto_index(movie_index, false, num_items);
                }
            },
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Right,
            input_area,
            move |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 0;
                    main_screen.item = 0;

                    main_screen
                        .movies_list
                        .goto_index(movie_index, false, num_items);

                    if let key_event_handler::Data::Mouse(mouse_event) = data {
                        main_screen.context_menu_pos =
                            Some(Position::new(mouse_event.column, mouse_event.row));
                        main_screen.context_menu.reset_state();
                    }
                }
            },
        );

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

        for position in buffer_area.positions() {
            buffer[position]
                .set_symbol(" ")
                .set_style(Style::new().bg(background).fg(text));
        }

        let area = helpers::add_padding(buffer_area, Padding::proportional(1));
        let [poster_area, _, description_area] =
            horizontal![==LIST_POSTER_WIDTH as u16, ==1, >=0].areas(area);

        let rating = self
            .watched
            .borrow()
            .get(&movie.id)
            .map(|x| x.get_user_rating());
        let rating_color = rating.as_ref().map(|rating| {
            if *rating >= 9.0 {
                tailwind::SKY.c400
            } else if *rating >= 8.0 {
                tailwind::GREEN.c500
            } else if *rating >= 7.5 {
                tailwind::LIME.c400
            } else if *rating >= 7.0 {
                material::AMBER.c400
            } else if *rating >= 6.0 {
                material::DEEP_ORANGE.c300
            } else {
                material::RED.c400
            }
        });

        let mut description = vec![];

        const TITLE_LINES: usize = 2;
        let mut title_lines = helpers::wrap_text(&movie.title, description_area.width as usize - 4);
        for _ in 0..(TITLE_LINES.saturating_sub(title_lines.len())) {
            description.push("".into());
        }
        title_lines.reverse();
        for _ in 0..(TITLE_LINES.min(title_lines.len()) - 1) {
            description.push(title_lines.pop().unwrap().bold().into());
        }
        description.push(line!(
            helpers::ellipsize_string(
                &title_lines.pop().unwrap(),
                description_area.width as usize - 5 - if movie.released { 0 } else { 15 },
            )
            .bold(),
            " ",
            movie.release_date.year().to_string().italic(),
            if !movie.released { span!(" - ") } else { "".into() },
            if !movie.released {
                span!("Not released").italic().fg(tailwind::RED.c300)
            } else {
                "".into()
            }
        ));

        if let (Some(rating), Some(rating_color)) = &(rating, rating_color) {
            description.push(
                format!("{:.1}", rating)
                    .set_style(*rating_color)
                    .bold()
                    .into(),
            );
        } else {
            description.push("Not watched".fg(tailwind::RED.c300).italic().into());
        }

        const TAGLINE_LINES: usize = 2;
        let mut tagline_lines = helpers::wrap_text(&movie.tagline, description_area.width as usize);
        for _ in 0..(TAGLINE_LINES.saturating_sub(tagline_lines.len())) {
            description.push("".into());
        }
        tagline_lines.reverse();
        for _ in 0..TAGLINE_LINES.min(tagline_lines.len()) {
            description.push(tagline_lines.pop().unwrap().into());
        }

        for _ in 0..(area.height.saturating_sub(description.len() as u16)) {
            description.insert(0, "".into());
        }

        line!(format!("#{}", movie_index + 1))
            .right_aligned()
            .bold()
            .fg(if selected {
                tailwind::GRAY.c200
            } else {
                tailwind::GRAY.c400
            })
            .render(area, buffer);

        Text::from_iter(description).render(description_area, buffer);

        // let unfocused_rating_color = if rating >= 9.0 {
        //     tailwind::SKY.c600
        // } else if rating >= 8.0 {
        //     tailwind::GREEN.c700
        // } else if rating >= 7.5 {
        //     tailwind::LIME.c700
        // } else if rating >= 7.0 {
        //     material::YELLOW.c700
        // } else if rating >= 6.0 {
        //     tailwind::AMBER.c600
        // } else {
        //     material::DEEP_ORANGE.c800
        // };
        if tab_selected && selected {
            for y in 0..area.height {
                buffer[(area.x - 2, area.y + y)]
                    .set_symbol(border::QUADRANT_RIGHT_HALF)
                    .set_style(
                        Style::new().fg(*rating_color.as_ref().unwrap_or(&tailwind::RED.c300)),
                    );
            }
        }

        let mut cell = Cell::new(" ");
        cell.set_style(Style::new().bg(tailwind::GRAY.c950));
        let mut image_buffer = Buffer::filled(poster_area.intersection(visible_area), cell);
        image_renderer.draw_image(
            ImageID::Movie(movie.id, movie.override_poster.clone(), false),
            false,
            if num_hidden_rows > 0 {
                Some(SignedPosition {
                    x: 0,
                    y: if align_bottom { -(num_hidden_rows as i16 - 1) } else { 0 },
                })
            } else {
                None
            },
            &mut image_buffer,
        );
        buffer.merge(&image_buffer);

        // frame.render_widget(
        //     line!("1 2 3 4 5 6 7 8 9 a b c d e f "),
        //     poster_area.offset(Offset { x: 0, y: -1 }),
        // );
    }
}
