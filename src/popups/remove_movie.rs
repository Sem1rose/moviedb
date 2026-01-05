use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
};
use ratatui::{layout::*, prelude::*, style::palette::material, widgets::*, Frame};
use ratatui_macros::vertical;
use style::palette::tailwind;

#[derive(Default)]
pub struct RemoveMoviePopup {
    item: usize,
    name: String,
}

impl RemoveMoviePopup {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn new(name: &str) -> Self {
        Self {
            item: 0,
            name: name.to_string(),
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        key_event_handler.bind_horizontal((None, None), |app, data| {
            if let Some(Popups::RemoveMovie(remove_movie_popup)) = app.drawer.active_popup.as_mut()
            {
                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        remove_movie_popup.item += 1;
                        if remove_movie_popup.item >= 2 {
                            remove_movie_popup.item = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        remove_movie_popup.item =
                            remove_movie_popup.item.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_esc((None, None), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_enter((None, Some(0)), |app, _| {
            app.drawer.close_popups();
        });
        key_event_handler.bind_enter((None, Some(1)), |app, _| {
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

        let [_, message_area, actions_area, _] = vertical![ ==1, >=1, ==1, ==1].areas(popup_area);
        frame.render_widget(
            Paragraph::new(format!("Do you really want to remove {}?", self.name))
                .wrap(Wrap { trim: false }),
            add_padding(message_area, Padding::left(2)),
        );

        frame.render_widget(
            Line::from(vec![
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
            ])
            .right_aligned(),
            actions_area,
        );
        Ok(())
    }
}
