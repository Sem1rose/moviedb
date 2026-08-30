use std::{
    path::Path,
    sync::mpsc::{Receiver, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::{HorizontalAlignment, Margin},
    macros::{constraint, line, vertical},
    style::{Style, palette::tailwind},
    widgets::Padding,
};
use ratatui_textarea::{TextArea, WrapMode};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::{Popup, PopupTrait},
    tokens::omdb_tokens::OMDBTokens,
    widgets::{self, Action, ActionType},
};

#[derive(Default)]
pub struct OMDBInitPopup {
    item:      usize,
    started:   bool,
    can_close: bool,
    pub tick:  u64,
    pub done:  bool,

    input:          TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init: Option<Receiver<anyhow::Result<String>>>,

    pub tokens: Option<String>,
}

impl OMDBInitPopup {
    pub fn new(home_dir: &Path, can_close: bool) -> Self {
        let (tx_init, rx_init) = channel();

        let home_dir_cloned = home_dir.to_path_buf();
        thread::spawn(move || {
            _ = tx_init.send(OMDBTokens::init(&home_dir_cloned));
        });

        Self {
            can_close,
            rx_init: Some(rx_init),
            ..Default::default()
        }
    }
}

impl PopupTrait for OMDBInitPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        !self.started
    }

    fn update(&mut self) {
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

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        key_event_handler.clear();
        if self.can_close {
            key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
                app.drawer.close_popup();
            });
            key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
                app.drawer.close_popup();
            });
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    app.drawer.close_popup();
                },
            );
        } else {
            // key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            //     app.quit = true;
            // });
            key_event_handler.bind_key((None, None), 'q', "Quit".into(), |app, _| {
                app.quit = true;
            });
        }

        if self.started {
            let input_valid = !self.input.is_empty();

            key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                if let Some(Popup::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(true, _) => {
                            omdb_init_popup.item += 1;
                            if omdb_init_popup.item > 1 {
                                omdb_init_popup.item = 0;
                            }
                        }
                        crate::key_event_handler::Data::Direction(false, _) => {
                            omdb_init_popup.item = omdb_init_popup.item.checked_sub(1).unwrap_or(1);
                        }
                        _ => {}
                    }
                }
            });
            if input_valid {
                key_event_handler.bind_enter((None, None), "Confirm".into(), |app, _| {
                    if let Some(Popup::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.tokens = Some(omdb_init_popup.input.lines()[0].clone());
                        omdb_init_popup.done = true;
                        omdb_init_popup.started = false;
                    }
                });
            }
            key_event_handler.bind_esc((None, Some(0)), "".into(), |app, _| {
                if let Some(Popup::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    omdb_init_popup.item = 1;
                }
            });
            key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                if let Some(Popup::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut() {
                    if let key_event_handler::Data::Key(key_event) = data {
                        omdb_init_popup.input.input(key_event);
                    }
                }
            });

            let popup_area = widgets::window(
                frame,
                helpers::centered_area(8, 40, frame.area()),
                " OMDB Authentication ",
                true,
            );
            image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                popup_area.outer(Margin::new(1, 1)),
                |_, _| {},
            );

            let [input_area, _] = vertical![==3, ==1]
                .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

            let input_selected = self.item == 0;
            widgets::input_field(
                true,
                input_selected,
                input_valid,
                &mut self.input,
                WrapMode::None,
                frame,
                input_area,
                " Key ",
                "Enter the Key",
                None,
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                helpers::add_padding(input_area, Padding::horizontal(2)),
                |app, _| {
                    if let Some(Popup::OMDBInit(omdb_init_popup)) = app.drawer.active_popup.as_mut()
                    {
                        omdb_init_popup.item = 0;
                    }
                },
            );

            let confirm_mouse_area = widgets::action(
                Action::new(
                    " Confirm ",
                    ActionType::Default,
                    self.item == 1,
                    input_valid,
                ),
                HorizontalAlignment::Right,
                true,
                helpers::add_padding(popup_area, Padding::right(1)),
                frame,
            );
            if input_valid {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    confirm_mouse_area,
                    |app, _| {
                        if let Some(Popup::OMDBInit(omdb_init_popup)) =
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
            let popup_area = widgets::window(
                frame,
                helpers::centered_area(7, 30, frame.area()),
                " OMDB Authentication ",
                false,
            );
            image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                popup_area.outer(Margin::new(1, 1)),
                |_, _| {},
            );
            let [_, message_area, throbber_area] = vertical![>=1, ==2, ==1].areas(popup_area);
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
