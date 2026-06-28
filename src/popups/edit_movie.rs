use ratatui::{
    Frame, layout::*, macros::vertical, prelude::*, style::palette::material, widgets::*,
};
use ratatui_textarea::{TextArea, WrapMode};
use style::palette::tailwind;

use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::KeyEventHandler,
    popups::Popups,
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct EditMoviePopup {
    item:     usize,
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
            .move_cursor(ratatui_textarea::CursorMove::End);

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
                "  Add a new play  "
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
        let [input_area, _, actions_area] =
            vertical![==3, ==1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));

        let search_selected = self.item == 0;
        widgets::input_field(
            search_selected,
            valid,
            &mut self.user_rating_input,
            WrapMode::None,
            frame,
            input_area,
            (0, 0),
            " Rating ",
            "Enter a rating",
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            add_padding(input_area, Padding::horizontal(2)),
            |app, _| {
                if let Some(Popups::EditMovie(edit_movie_popup)) = app.drawer.active_popup.as_mut()
                {
                    edit_movie_popup.item = 0;
                }
            },
        );

        let actions_mouse_areas = widgets::actions(
            [
                Action::new(" Confirm ", ActionTypes::Default, self.item == 1, valid),
                Action::new(" Cancel ", ActionTypes::Critical, self.item == 2, true),
            ],
            HorizontalAlignment::Right,
            1,
            actions_area,
            frame,
        );
        for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i == 1 {
                        app.drawer.close_popups();
                    } else {
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
    }
}
