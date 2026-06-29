use ratatui::{
    Frame,
    layout::{Alignment, HorizontalAlignment, Margin},
    macros::vertical,
    style::{Style, palette::tailwind},
    text::Text,
    widgets::Padding,
};

use crate::{
    helpers::{add_padding, dynamic_popup, wrap_text},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct DeleteMoviePopup {
    item: usize,
    name: String,
}

impl DeleteMoviePopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new(name: &str) -> Self {
        Self {
            item: 0,
            name: name.to_string(),
        }
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
        key_event_handler.bind_horizontal((None, None), "Navigate".into(), |app, data| {
            if let Some(Popups::DeleteMovie(delete_movie_popup)) = app.drawer.active_popup.as_mut()
            {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        delete_movie_popup.item += 1;
                        if delete_movie_popup.item >= 2 {
                            delete_movie_popup.item = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        delete_movie_popup.item =
                            delete_movie_popup.item.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_esc((None, None), "Cancel".into(), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_enter((None, Some(0)), "Cancel".into(), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_enter((None, Some(1)), "Confirm".into(), |app, _| {
            app.remove_movie();
            app.drawer.close_popups();
        });

        let popup_area = dynamic_popup(
            frame,
            Some(7),
            5.0,
            tailwind::BLUE.c950,
            "  Remove movie  ",
            Style::new().fg(tailwind::AMBER.c500),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [message_area, actions_area] =
            vertical![ >=1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));
        frame.render_widget(
            Text::from_iter(wrap_text(
                &format!("Do you really want to remove {}?", self.name),
                message_area.width as usize,
            )),
            message_area,
        );

        let actions_mouse_areas = widgets::actions(
            [
                Action::new(" Confirm ", ActionTypes::Critical, self.item == 1, true),
                Action::new(" Cancel ", ActionTypes::Normal, self.item == 0, true),
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
                    if i == 0 {
                        app.remove_movie();
                    }
                    app.drawer.close_popups();
                },
            );
        }
    }
}
