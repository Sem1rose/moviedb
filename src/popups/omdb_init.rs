use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::{Alignment, HorizontalAlignment, Margin},
    macros::{constraint, line, vertical},
    style::{
        Style,
        palette::{material, tailwind},
    },
    widgets::Padding,
};
use ratatui_textarea::{TextArea, WrapMode};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    tokens::omdb_tokens::OMDBTokens,
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct OMDBInitPopup {
    item:     usize,
    started:  bool,
    pub tick: u64,
    pub done: bool,

    input:          TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init: Option<Receiver<anyhow::Result<String>>>,

    pub tokens: Option<String>,
}

impl OMDBInitPopup {
    pub fn new(home_dir: &PathBuf) -> Self {
        let (tx_init, rx_init) = channel();
        let home_dir_cloned = home_dir.clone();

        thread::spawn(move || {
            _ = tx_init.send(OMDBTokens::init(&home_dir_cloned));
        });

        Self {
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
            let input_valid = !self.input.is_empty();

            key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(true, _) => {
                            omdb_init_popup.item += 1;
                            if omdb_init_popup.item > 2 {
                                omdb_init_popup.item = 0;
                            }
                        }
                        crate::key_event_handler::Data::Direction(false, _) => {
                            omdb_init_popup.item = omdb_init_popup.item.checked_sub(1).unwrap_or(2);
                        }
                        _ => {}
                    }
                }
            });
            if input_valid {
                key_event_handler.bind_enter((None, None), "Confirm".into(), |app, _| {
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.tokens = Some(omdb_init_popup.input.lines()[0].clone());
                        omdb_init_popup.done = true;
                        omdb_init_popup.started = false;
                    }
                });
            }
            key_event_handler.bind_enter((None, Some(2)), "Skip".into(), |app, _| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    omdb_init_popup.done = true;
                    omdb_init_popup.started = false;
                }
            });
            key_event_handler.bind_esc((None, Some(0)), "".into(), |app, _| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    omdb_init_popup.item = 2;
                }
            });
            key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                if let Some(Popups::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    match data {
                        key_event_handler::Data::Key(key_event) => {
                            omdb_init_popup.input.input(key_event);
                        }
                        _ => (),
                    }
                }
            });

            let popup_area = dynamic_popup(
                frame,
                Some(8),
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

            let skip_mouse_area = widgets::action(
                Action::new(" Skip ", ActionTypes::Normal, self.item == 2, true),
                HorizontalAlignment::Right,
                popup_area,
                frame,
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                skip_mouse_area,
                |app, _| {
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.done = true;
                        omdb_init_popup.started = false;
                    }
                },
            );

            let [_, input_area, _, actions_area] = vertical![==1, ==3, >=1, ==1]
                .areas(add_padding(popup_area, Padding::proportional(1)));

            let input_selected = self.item == 0;
            widgets::input_field(
                input_selected,
                input_valid,
                &mut self.input,
                WrapMode::None,
                frame,
                input_area,
                (6, 6),
                " Key ",
                "Enter the Key",
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                add_padding(input_area, Padding::horizontal(2)),
                |app, _| {
                    if let Some(Popups::OMDBInit(omdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.item = 0;
                    }
                },
            );

            let confirm_mouse_area = widgets::action(
                Action::new(
                    " Confirm ",
                    ActionTypes::Default,
                    self.item == 1,
                    input_valid,
                ),
                HorizontalAlignment::Right,
                actions_area,
                frame,
            );
            if input_valid {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    confirm_mouse_area,
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
            frame.render_widget(line!("Processing").centered(), message_area);

            frame.render_stateful_widget(
                Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                throbber_area.centered(constraint!(==1), constraint!(==1)),
                &mut self.throbber_state,
            );
        }
    }
}
