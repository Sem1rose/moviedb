use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
};
use ratatui::{
    layout::*, macros::vertical, prelude::*, style::palette::material, widgets::*, Frame,
};
use style::palette::tailwind;

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
            Some(5),
            10.0,
            tailwind::BLUE.c950,
            "  Remove movie  ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [_, message_area, actions_area, _] = vertical![ ==1, >=1, ==1, ==1].areas(popup_area);
        frame.render_widget(
            Paragraph::new(format!("Do you really want to remove {}?", self.name))
                .wrap(Wrap { trim: false }),
            add_padding(message_area, Padding::left(2)),
        );

        let actions = vec![
            Span::from(" Cancel ").style(
                Style::new()
                    .fg(if self.item == 0 {
                        tailwind::SLATE.c200
                    } else {
                        tailwind::SLATE.c300
                    })
                    .bg(if self.item == 0 {
                        material::BLUE.c600
                    } else {
                        material::BLUE.c900
                    }),
            ),
            Span::from(" "),
            Span::from(" Confirm ").style(
                Style::new()
                    .fg(if self.item == 1 {
                        tailwind::SLATE.c300
                    } else {
                        tailwind::RED.c500
                    })
                    .bg(if self.item == 1 {
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
                        app.remove_movie();
                    }
                    app.drawer.close_popups();
                },
            );
        }
        frame.render_widget(Line::from(actions).right_aligned(), actions_area);
    }
}
