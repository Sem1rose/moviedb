use ratatui::{
    Frame,
    layout::{HorizontalAlignment, Margin},
    macros::vertical,
    text::Text,
    widgets::Padding,
};

use crate::{
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::{Popup, PopupTrait},
    widgets::{self, Action, ActionType},
};

#[derive(Default)]
pub struct DeleteMoviePopup {
    item: usize,
    name: String,
}

impl DeleteMoviePopup {
    pub fn new(name: &str) -> Self {
        Self {
            item: 0,
            name: name.to_string(),
        }
    }
}

impl PopupTrait for DeleteMoviePopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        key_event_handler.bind_horizontal((None, None), "Navigate".into(), |app, data| {
            if let Some(Popup::DeleteMovie(delete_movie_popup)) = app.drawer.active_popup.as_mut() {
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
            app.drawer.close_popup();
        });
        key_event_handler.bind_enter((None, Some(0)), "Cancel".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_enter((None, Some(1)), "Confirm".into(), |app, _| {
            app.remove_movie();
            app.drawer.close_popup();
        });

        let popup_area = widgets::window(
            frame,
            helpers::centered_area(8, 40, frame.area()),
            " Remove movie ",
            true,
        );
        image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [message_area] =
            vertical![>=3].areas(helpers::add_padding(popup_area, Padding::proportional(1)));
        frame.render_widget(
            Text::from_iter(helpers::wrap_text(
                &format!("Do you really want to remove {}?", self.name),
                message_area.width as usize,
            )),
            message_area,
        );

        let actions_mouse_areas = widgets::actions(
            [
                Action::new(" Confirm ", ActionType::Normal, self.item == 1, true),
                Action::new(" Cancel ", ActionType::Critical, self.item == 0, true),
            ],
            HorizontalAlignment::Right,
            true,
            1,
            helpers::add_padding(popup_area, Padding::right(1)),
            frame.buffer_mut(),
        );
        for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i == 0 {
                        app.remove_movie();
                    }
                    app.drawer.close_popup();
                },
            );
        }
    }
}
