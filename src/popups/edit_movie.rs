use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::KeyEventHandler,
    popups::Popups,
};
use ratatui::{
    layout::*, macros::vertical, prelude::*, style::palette::material, widgets::*, Frame,
};
use style::palette::tailwind;
use tui_textarea::TextArea;

#[derive(Default)]
pub struct EditMoviePopup {
    item: usize,
    new_play: bool,

    pub user_rating_input: TextArea<'static>,
}

impl EditMoviePopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new(new_play: bool, user_rating: f64) -> Self {
        let mut popup = Self::default();

        popup.user_rating_input = TextArea::from([if new_play {
            "".into()
        } else {
            format!("{:.1}", user_rating)
        }]);
        popup
            .user_rating_input
            .move_cursor(tui_textarea::CursorMove::End);

        popup.new_play = new_play;
        popup
    }

    pub fn validate_rating(&mut self) -> bool {
        if self.user_rating_input.is_empty() {
            return false;
        }

        if let Ok(x) = self.user_rating_input.lines()[0].parse::<f64>() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popups();
            },
        );
        let valid = self.validate_rating();
        let add_play = self.new_play;
        key_event_handler.bind_enter((None, None), "Confirm".into(), move |app, _| {
            if valid {
                if add_play {
                    app.add_play();
                } else {
                    app.edit_movie();
                }
                app.drawer.close_popups();
            }
        });
        key_event_handler.bind_enter((None, Some(2)), "Close".into(), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_tab((None, None), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        edit_movie_popup.item += 1;
                        if edit_movie_popup.item > 2 {
                            edit_movie_popup.item = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        edit_movie_popup.item = edit_movie_popup.item.checked_sub(1).unwrap_or(2);
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Key(key_event) => {
                        edit_movie_popup.user_rating_input.input(key_event);
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_horizontal((None, Some(1)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        edit_movie_popup.item = 2;
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_horizontal((None, Some(2)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false, _) => {
                        edit_movie_popup.item = 1;
                    }
                    _ => {}
                }
            }
        });

        let popup_area = dynamic_popup(
            frame,
            Some(7),
            5.0,
            tailwind::BLUE.c950,
            if self.new_play {
                "  Add new play  "
            } else {
                "  Edit rating  "
            },
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [_, input_area, _, actions_area, _] =
            vertical![==1, ==3, ==1, ==1, ==1].areas(popup_area);

        let actions = vec![
            Span::from(" Confirm ").style(
                Style::new()
                    .fg(if valid {
                        if self.item == 1 {
                            tailwind::SLATE.c200
                        } else {
                            tailwind::SLATE.c300
                        }
                    } else {
                        tailwind::SLATE.c500
                    })
                    .bg(if valid {
                        if self.item == 1 {
                            material::BLUE.c600
                        } else {
                            material::BLUE.c900
                        }
                    } else {
                        if self.item == 1 {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    }),
            ),
            Span::from(" "),
            Span::from(" Cancel ").style(
                Style::new()
                    .fg(if self.item == 2 {
                        tailwind::SLATE.c300
                    } else {
                        tailwind::RED.c500
                    })
                    .bg(if self.item == 2 {
                        material::RED.c800
                    } else {
                        tailwind::SLATE.c950
                    }),
            ),
            Span::from("  "),
        ];
        let mut mouse_area = actions_area
            .offset(Offset::new(actions_area.width as i32, 0))
            .resize(Size::new(1, 1));
        for (i, action) in actions.iter().rev().enumerate() {
            mouse_area = mouse_area.offset(Offset::new(-(action.width() as i32), 0));
            if i & 1 == 0 {
                continue;
            }

            mouse_area = mouse_area.resize(Size {
                width: action.width() as u16,
                height: 1,
            });

            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i / 2 == 0 {
                        app.drawer.close_popups();
                    } else if i / 2 == 1 {
                        if valid {
                            if add_play {
                                app.add_play();
                            } else {
                                app.edit_movie();
                            }

                            app.drawer.close_popups();
                        }
                    }
                },
            );
        }
        frame.render_widget(Line::from(actions).right_aligned(), actions_area);

        let search_selected = self.item == 0;
        if search_selected {
            key_event_handler.bind_input_field((Some(2), Some(0)), "".into(), |app, data| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    match data {
                        crate::key_event_handler::Data::Key(key_event) => {
                            edit_movie_popup.user_rating_input.input(key_event);
                        }
                        _ => {}
                    }
                }
            });
        }
        self.user_rating_input
            .set_style(Style::new().fg(if search_selected {
                tailwind::SLATE.c200
            } else {
                tailwind::STONE.c400
            }));
        self.user_rating_input.set_cursor_style(
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
        self.user_rating_input.set_block(
            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Thick)
                .style(Style::new().fg(if search_selected {
                    if valid {
                        material::BLUE.c500
                    } else {
                        material::RED.c600
                    }
                } else {
                    tailwind::STONE.c600
                }))
                .title(" Rating ")
                .title_style(Style::new().fg(if search_selected {
                    if valid {
                        material::BLUE.c600
                    } else {
                        material::RED.c600
                    }
                } else {
                    tailwind::SLATE.c600
                }))
                .padding(Padding::symmetric(1, 0)),
        );
        self.user_rating_input.set_placeholder_text("Enter rating");
        self.user_rating_input
            .set_placeholder_style(Style::new().fg(material::GRAY.c700));
        frame.render_widget(
            &self.user_rating_input,
            add_padding(
                input_area,
                Padding {
                    left: 2,
                    right: 2,
                    top: 0,
                    bottom: 0,
                },
            ),
        );

        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            add_padding(
                input_area,
                Padding {
                    left: 2,
                    right: 2,
                    top: 0,
                    bottom: 0,
                },
            ),
            |app, _| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    edit_movie_popup.item = 0;
                }
            },
        );
    }
}
