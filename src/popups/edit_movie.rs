use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::KeyEventHandler,
    popups::Popups,
};
use ratatui::{layout::*, prelude::*, style::palette::material, widgets::*, Frame};
use ratatui_macros::vertical;
use style::palette::tailwind;
use tui_textarea::TextArea;

#[derive(Default)]
pub struct EditMoviePopup {
    item: usize,

    pub user_rating_input: TextArea<'static>,
}

impl EditMoviePopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new(user_rating: f64) -> Self {
        let mut popup = Self::default();

        popup.user_rating_input = TextArea::from([format!("{:.1}", user_rating)]);
        popup
            .user_rating_input
            .move_cursor(tui_textarea::CursorMove::End);

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

    pub fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        key_event_handler.clear();
        let valid = self.validate_rating();
        key_event_handler.bind_enter((None, Some(0)), move |app, _| {
            if valid {
                app.edit_movie();
                app.drawer.close_popups();
            }
        });
        key_event_handler.bind_enter((None, Some(1)), move |app, _| {
            if valid {
                app.edit_movie();
                app.drawer.close_popups();
            }
        });
        key_event_handler.bind_enter((None, Some(2)), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_esc((None, None), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_tab((None, None), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        edit_movie_popup.item += 1;
                        if edit_movie_popup.item > 2 {
                            edit_movie_popup.item = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false) => {
                        edit_movie_popup.item = edit_movie_popup.item.checked_sub(1).unwrap_or(2);
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_input_field((None, Some(0)), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Key(key_event) => {
                        edit_movie_popup.user_rating_input.input(key_event);
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_horizontal((None, Some(1)), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true) => {
                        edit_movie_popup.item = 2;
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_horizontal((None, Some(2)), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false) => {
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
            "  Edit rating  ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
        );

        let [_, input_area, _, actions_area, _] =
            vertical![==1, ==3, ==1, ==1, ==1].areas(popup_area);

        frame.render_widget(
            Line::from(vec![
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
                Span::from(" No ").style(
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
            ])
            .right_aligned(),
            actions_area,
        );

        let search_selected = self.item == 0;
        if search_selected {
            key_event_handler.bind_input_field((Some(2), Some(0)), |app, data| {
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
                .title(" New rating ")
                .title_style(Style::new().fg(if search_selected {
                    if valid {
                        material::BLUE.c600
                    } else {
                        material::RED.c600
                    }
                } else {
                    tailwind::SLATE.c600
                })),
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

        Ok(())
    }
}
