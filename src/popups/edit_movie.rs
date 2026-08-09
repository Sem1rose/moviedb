use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Alignment, HorizontalAlignment, Margin},
    macros::vertical,
    style::{
        Style,
        palette::{material, tailwind},
    },
    widgets::Padding,
};
use ratatui_textarea::{TextArea, WrapMode};

use crate::{
    helpers::{add_padding, centered_area, create_popup},
    key_event_handler::KeyEventHandler,
    popups::{PopupTrait, Popups},
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct EditMoviePopup {
    item:     usize,
    new_play: bool,

    pub rating_input: TextArea<'static>,
    pub date_input:   TextArea<'static>,
}

impl EditMoviePopup {
    pub fn new(new_play: bool, user_rating: f64, watched_at: Option<DateTime<Local>>) -> Self {
        let mut popup = Self::default();
        popup.new_play = new_play;

        popup.rating_input = TextArea::from([if new_play {
            "".into()
        } else {
            format!("{:.1}", user_rating)
        }]);
        popup.date_input = TextArea::from([watched_at.map(|x| x.to_string()).unwrap_or("".into())]);
        popup
            .rating_input
            .move_cursor(ratatui_textarea::CursorMove::End);
        popup
            .date_input
            .move_cursor(ratatui_textarea::CursorMove::End);

        popup
    }

    pub fn validate_rating(&mut self) -> bool {
        if self.rating_input.is_empty() {
            return false;
        }

        if let Ok(x) = self.rating_input.lines()[0].parse::<f64>() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn validate_input_date(&mut self) -> bool {
        ["now", ""].contains(&self.date_input.lines()[0].trim().to_lowercase().as_str())
            || self.date_input.lines()[0]
                .parse::<DateTime<Local>>()
                .is_ok()
    }
}

impl PopupTrait for EditMoviePopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        let rating_valid = self.validate_rating();
        let date_valid = self.validate_input_date();
        let add_play = self.new_play;

        key_event_handler.bind_esc((None, Some(3)), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                edit_movie_popup.item = 3;
            }
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

        if rating_valid {
            key_event_handler.bind_enter((None, Some(0)), "".into(), |app, _| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    edit_movie_popup.item = 1;
                }
            });
            if date_valid {
                key_event_handler.bind_enter((None, Some(2)), "Confirm".into(), move |app, _| {
                    if add_play {
                        app.add_play();
                    } else {
                        app.edit_movie();
                    }
                    app.drawer.close_popup();
                });
                key_event_handler.bind_enter((None, Some(1)), "Confirm".into(), move |app, _| {
                    if add_play {
                        app.add_play();
                    } else {
                        app.edit_movie();
                    }
                    app.drawer.close_popup();
                });
            }
        }
        key_event_handler.bind_enter((None, Some(3)), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        key_event_handler.bind_vertical((None, Some(0)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        edit_movie_popup.item = 1;
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_vertical((None, Some(1)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false, _) => {
                        edit_movie_popup.item = 0;
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_horizontal((None, Some(2)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        edit_movie_popup.item = 3;
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_horizontal((None, Some(3)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false, _) => {
                        edit_movie_popup.item = 2;
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Key(key_event) => {
                        let parsed = edit_movie_popup.rating_input.lines()[0]
                            .parse::<f64>()
                            .unwrap_or(0.0);
                        if let KeyCode::Char(x) = &key_event.code {
                            if edit_movie_popup.rating_input.lines()[0].len() >= 3 || parsed >= 10.0
                            {
                                return;
                            }

                            if !x.is_ascii_digit() && *x != '.' {
                                return;
                            }
                        }

                        edit_movie_popup.rating_input.input(key_event);
                    }
                    _ => {}
                }
            }
        });
        key_event_handler.bind_input_field((None, Some(1)), "".into(), |app, data| {
            if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Key(key_event) => {
                        edit_movie_popup.date_input.input(key_event);
                    }
                    _ => {}
                }
            }
        });

        let popup_area = create_popup(
            frame,
            centered_area(11, 44, frame.area()),
            if self.new_play {
                " Add a new play "
            } else {
                " Edit rating "
            },
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
            tailwind::BLUE.c950,
            true,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [rating_input_area, date_input_area, _] =
            vertical![==3, ==3, >=1].areas(add_padding(popup_area, Padding::proportional(1)));

        let rating_input_selected = self.item == 0;
        widgets::input_field(
            true,
            rating_input_selected,
            rating_valid,
            &mut self.rating_input,
            WrapMode::None,
            frame,
            rating_input_area,
            " Rating ",
            "Enter a rating",
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            add_padding(rating_input_area, Padding::horizontal(2)),
            |app, _| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    edit_movie_popup.item = 0;
                }
            },
        );

        let date_input_selected = self.item == 1;
        widgets::input_field(
            true,
            date_input_selected,
            date_valid,
            &mut self.date_input,
            WrapMode::None,
            frame,
            date_input_area,
            " Watched At ",
            "Now",
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            add_padding(date_input_area, Padding::horizontal(2)),
            |app, _| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    edit_movie_popup.item = 1;
                }
            },
        );

        let actions_mouse_areas = widgets::actions(
            [
                Action::new(
                    " Confirm ",
                    ActionTypes::Default,
                    self.item == 2,
                    rating_valid && date_valid,
                ),
                Action::new(" Cancel ", ActionTypes::Critical, self.item == 3, true),
            ],
            HorizontalAlignment::Right,
            true,
            1,
            add_padding(popup_area, Padding::right(1)),
            frame,
        );
        for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i == 1 {
                        app.drawer.close_popup();
                    } else {
                        if rating_valid && date_valid {
                            if add_play {
                                app.add_play();
                            } else {
                                app.edit_movie();
                            }

                            app.drawer.close_popup();
                        }
                    }
                },
            );
        }
    }
}
