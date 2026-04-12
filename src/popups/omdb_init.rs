use crate::{
    helpers::{add_padding, center_rect, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    tokens::omdb_tokens::{OMDBTokens},
};
use ratatui::{
    layout::*,
    macros::{constraint, vertical},
    prelude::*,
    style::palette::{material, tailwind},
    widgets::*,
    Frame,
};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
    thread,
};
use ratatui_textarea::TextArea;
use throbber_widgets_tui::{Throbber, ThrobberState};

#[derive(Default)]
pub struct OMDBInitPopup {
    item: usize,
    started: bool,
    pub tick: u64,
    pub done: bool,
    home_dir: PathBuf,

    input: TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init: Option<Receiver<anyhow::Result<String>>>,

    pub tokens: Option<String>
}

impl OMDBInitPopup {
    pub fn new(home_dir: &PathBuf) -> Self {
        let (tx_init, rx_init) = channel();
        let home_dir_cloned = home_dir.clone();

        thread::spawn(move || {
            _ = tx_init.send(OMDBTokens::init(&home_dir_cloned));
        });

        Self {
            home_dir: home_dir.clone(),
            rx_init: Some(rx_init),
            ..Default::default()
        }
    }

    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    pub fn update_next_frame(&self) -> bool {
        !self.started
    }

    pub fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        if !(self.started || self.done) {
            if let Some(rx_init_response) = self.rx_init.as_ref() {
                if let Ok(result) = rx_init_response.try_recv() {
                    if let Ok(tokens) = result {
                        self.done = !tokens.is_empty();
                        self.started = !self.done;
                        self.tokens = Some(tokens);
                    } else {
                        self.done = false;
                        self.started = true;
                    }
                }
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.quit = true;
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.quit = true;
        });

        if self.started {
            key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    match data {
                        crate::key_event_handler::Data::Direction(true, _) => {
                            omdb_init_popup.item += 1;
                            if omdb_init_popup.item > 1 {
                                omdb_init_popup.item = 0;
                            }
                        }
                        crate::key_event_handler::Data::Direction(false, _) => {
                            omdb_init_popup.item =
                                omdb_init_popup.item.checked_sub(1).unwrap_or(1);
                        }
                        _ => {}
                    }
                }
            });
            key_event_handler.bind_enter((None, Some(0)), "".into(), |app, _| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    omdb_init_popup.item = 1;
                }
            });
            key_event_handler.bind_esc((None, Some(0)), "".into(), |app, _| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    omdb_init_popup.item = 1;
                }
            });
            key_event_handler.bind_enter(
                (None, Some(1)),
                "Confirm".into(),
                |app, _| {
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.tokens = Some(omdb_init_popup.input.lines()[0].clone());
                        omdb_init_popup.done = true;
                        omdb_init_popup.started = false;
                    }
                },
            );
            key_event_handler.bind_input_field(
                (None, Some(0)),
                "".into(),
                |app, data| {
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Key(key_event) => {
                                omdb_init_popup.input.input(key_event);
                            }
                            _ => (),
                        }
                    }
                },
            );

            let popup_area = dynamic_popup(
                frame,
                Some(7),
                4.0,
                tailwind::BLUE.c950,
                "  OMDB Authentication  ",
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
                vertical![==1, ==3, >=1, ==1, ==1].areas(popup_area);
            let actions = vec![
                Span::from(" Confirm ").style(
                    Style::new()
                        .fg(if self.item == 1 {
                            tailwind::SLATE.c200
                        } else {
                            tailwind::SLATE.c300
                        })
                        .bg(if self.item == 1 {
                            material::BLUE.c600
                        } else {
                            material::BLUE.c900
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
                    |app, _| {
                        if let Some(Popups::OMDBInit(omdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            omdb_init_popup.tokens = Some(omdb_init_popup.input.lines()[0].clone());
                            omdb_init_popup.done = true;
                            omdb_init_popup.started = false;
                        }
                    },
                );
            }
            frame.render_widget(Line::from(actions).right_aligned(), actions_area);

            let input_selected = self.item == 0;
            self.input.set_style(Style::new().fg(if input_selected {
                tailwind::SLATE.c300
            } else {
                tailwind::STONE.c400
            }));
            self.input.set_cursor_style(
                Style::new()
                    .fg(if input_selected {
                        tailwind::SLATE.c300
                    } else {
                        tailwind::STONE.c400
                    })
                    .add_modifier(if input_selected {
                        Modifier::REVERSED
                    } else {
                        Modifier::default()
                    }),
            );
            self.input.set_block(
                Block::bordered()
                    .border_type(ratatui::widgets::BorderType::Thick)
                    .style(Style::new().fg(if input_selected {
                        material::BLUE.c500
                    } else {
                        tailwind::STONE.c500
                    }))
                    .title(" Key ")
                    .title_style(Style::new().fg(if input_selected {
                        material::BLUE.c400
                    } else {
                        material::BLUE.c600
                    }))
                    .padding(Padding::symmetric(1, 0)),
            );
            self.input.set_placeholder_text("Enter the Key");
            self.input
                .set_placeholder_style(Style::new().fg(material::GRAY.c700));
            frame.render_widget(
                &self.input,
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
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.item = 0;
                    }
                },
            );
        } else {
            let popup_area = dynamic_popup(
                frame,
                Some(5),
                4.0,
                tailwind::BLUE.c950,
                "  OMDB Authentication  ",
                Style::new().fg(material::YELLOW.c800),
                Alignment::Center,
                Style::new().fg(tailwind::VIOLET.c950),
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                popup_area.outer(Margin::new(1, 1)),
                |_, _| {},
            );
            let [_, message_area, throbber_area, _] =
                vertical![>=1, ==2, ==1, >=1].areas(popup_area);
            frame.render_widget(Paragraph::new("Processing").centered(), message_area);

            frame.render_stateful_widget(
                Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                center_rect(throbber_area, constraint!(==1), constraint!(==1)),
                &mut self.throbber_state,
            );
        }
    }
}
