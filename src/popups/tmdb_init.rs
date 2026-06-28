use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::*,
    macros::{constraint, horizontal, text, vertical},
    prelude::*,
    style::palette::{material, tailwind},
    widgets::*,
};
use ratatui_textarea::{TextArea, WrapMode};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    helpers::{add_padding, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    tmdb,
    tokens::tmdb_tokens::{TMDBTokens, UserTokens},
    widgets::{self, Action, ActionTypes},
};

#[derive(Default, Debug)]
pub enum Phase {
    #[default]
    Initializing,
    GetAccessToken,
    GettingAuthorizationUrl,
    Authorize(String),
    Finalize,
    Error(String),
    Done,
}

#[derive(Default)]
pub struct TMDBInitPopup {
    pub tick:         u64,
    pub phase:        Phase,
    throbber_visible: bool,
    item:             usize,

    input:          TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init:              Option<Receiver<anyhow::Result<UserTokens>>>,
    rx_authorization_url: Option<Receiver<String>>,
    rx_session_id:        Option<Receiver<anyhow::Result<String>>>,

    pub tokens: Option<UserTokens>,

    home_dir: PathBuf,
}

impl TMDBInitPopup {
    pub fn new(home_dir: &PathBuf) -> Self {
        let (tx_init, rx_init) = channel();
        let home_dir_cloned = home_dir.clone();

        thread::spawn(move || {
            _ = tx_init.send(TMDBTokens::init(&home_dir_cloned));
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
        self.throbber_visible || matches!(self.phase, Phase::Authorize(_))
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Initializing => Phase::GetAccessToken,
            Phase::GetAccessToken => {
                let access_token = self.input.lines()[0].clone();

                self.tokens = Some(UserTokens {
                    access_token: access_token.clone(),
                    session_id:   String::default(),
                });

                let (tx_authorization_url, rx_authorization_url) = channel();
                let (tx_session_id, rx_session_id) = channel();
                thread::spawn(move || {
                    _ = tx_session_id
                        .send(tmdb::get_session_id(&access_token, tx_authorization_url));
                });

                self.rx_authorization_url = Some(rx_authorization_url);
                self.rx_session_id = Some(rx_session_id);

                Phase::GettingAuthorizationUrl
            }
            Phase::Authorize(_) => Phase::Finalize,
            Phase::Finalize => Phase::Done,
            _ => Phase::Initializing,
        };
    }

    pub fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        match self.phase {
            Phase::Initializing =>
                if let Some(rx_init_response) = self.rx_init.as_ref() {
                    if let Ok(result) = rx_init_response.try_recv() {
                        if let Ok(tokens) = result {
                            if !tokens.has_access_token() {
                                self.advance_phase();
                            } else if !tokens.has_session_id() {
                                self.advance_phase();
                                self.input = TextArea::new(vec![tokens.access_token.clone()]);
                                self.advance_phase();
                            } else {
                                self.tokens = Some(tokens);
                                self.phase = Phase::Done;
                            }
                        } else {
                            self.advance_phase();
                        }
                    }
                },
            Phase::GettingAuthorizationUrl => {
                if let Some(rx_authorization_url) = self.rx_authorization_url.as_ref() {
                    if let Ok(authorization_url) = rx_authorization_url.try_recv() {
                        self.phase = Phase::Authorize(authorization_url);
                    }
                }
                if let Some(rx_session_id) = self.rx_session_id.as_ref() {
                    if let Ok(result) = rx_session_id.try_recv() {
                        if let Err(error) = result {
                            self.phase = Phase::Error(format!("{:#}", error));
                        }
                    }
                }
            }
            Phase::Authorize(_) => {
                if let Some(rx_authorization_url) = self.rx_authorization_url.as_ref() {
                    if let Err(std::sync::mpsc::TryRecvError::Disconnected) =
                        rx_authorization_url.try_recv()
                    {
                        self.advance_phase();
                    }
                }
                if let Some(rx_session_id) = self.rx_session_id.as_ref() {
                    if let Ok(result) = rx_session_id.try_recv() {
                        if let Err(error) = result {
                            self.phase = Phase::Error(format!("{:#}", error));
                        }
                    }
                }
            }
            Phase::Finalize =>
                if let Some(rx_session_id) = self.rx_session_id.as_ref() {
                    if let Ok(result) = rx_session_id.try_recv() {
                        match result {
                            Ok(session_id) => {
                                if let Some(tokens) = self.tokens.as_mut() {
                                    tokens.session_id = session_id;
                                }

                                self.advance_phase();
                            }
                            Err(error) => {
                                self.phase = Phase::Error(format!("{:#}", error));
                            }
                        }
                    }
                },
            _ => (),
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

        self.throbber_visible = false;
        match &self.phase {
            Phase::Initializing
            | Phase::GettingAuthorizationUrl
            | Phase::Finalize
            | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = dynamic_popup(
                    frame,
                    Some(5),
                    4.0,
                    tailwind::BLUE.c950,
                    "  TMDB Authentication  ",
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
                    throbber_area.centered(constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::GetAccessToken => {
                let input_valid = !self.input.is_empty();

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                tmdb_init_popup.item += 1;
                                if tmdb_init_popup.item > 1 {
                                    tmdb_init_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                tmdb_init_popup.item =
                                    tmdb_init_popup.item.checked_sub(1).unwrap_or(1);
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_esc((None, Some(0)), "".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 1;
                    }
                });
                if input_valid {
                    key_event_handler.bind_enter((None, None), "Confirm".into(), |app, _| {
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.advance_phase();
                        }
                    });
                }
                key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Key(key_event) => {
                                tmdb_init_popup.input.input(key_event);
                            }
                            _ => (),
                        }
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(9),
                    4.0,
                    tailwind::BLUE.c950,
                    "  TMDB Authentication  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let [input_area, _, actions_area] = vertical![==5, >=1, ==1]
                    .areas(add_padding(popup_area, Padding::proportional(1)));

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
                            if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                tmdb_init_popup.advance_phase();
                            }
                        },
                    );
                }

                let input_selected = self.item == 0;
                widgets::input_field(
                    input_selected,
                    input_valid,
                    &mut self.input,
                    WrapMode::Glyph,
                    frame,
                    input_area,
                    (0, 0),
                    " Access Token ",
                    "Enter the Access Token",
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    input_area,
                    |app, _| {
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.item = 0;
                        }
                    },
                );
            }
            Phase::Authorize(authorization_url) => {
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 0;
                        tmdb_init_popup.input.clear();
                        tmdb_init_popup.rx_session_id = None;
                        tmdb_init_popup.rx_authorization_url = None;
                        tmdb_init_popup.phase = Phase::GetAccessToken;
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(8),
                    4.0,
                    tailwind::BLUE.c950,
                    "  TMDB Authentication  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let back_mouse_area = widgets::action(
                    Action::new(" Back ", ActionTypes::Default, false, true),
                    HorizontalAlignment::Left,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    back_mouse_area,
                    |app, _| {
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.item = 0;
                            tmdb_init_popup.input.clear();
                            tmdb_init_popup.rx_session_id = None;
                            tmdb_init_popup.rx_authorization_url = None;
                            tmdb_init_popup.phase = Phase::GetAccessToken;
                        }
                    },
                );

                let [_, hyperlink_area, _] = vertical![>=1, ==3, >=1]
                    .areas(add_padding(popup_area, Padding::proportional(1)));

                let hyperlink_text = "  Click to Authorize  ";
                let [hyperlink_area] = horizontal![==(hyperlink_text.len() as u16)]
                    .flex(Flex::Center)
                    .areas(hyperlink_area);
                widgets::hyperlink(
                    text![
                        " ".repeat(hyperlink_text.len()),
                        hyperlink_text,
                        " ".repeat(hyperlink_text.len())
                    ]
                    .fg(material::GREEN.c100)
                    .bg(material::BLUE.c800),
                    authorization_url,
                    hyperlink_area,
                    frame,
                );
            }
            Phase::Error(error) => {
                key_event_handler.bind_enter((None, None), "Back".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 0;
                        tmdb_init_popup.input.clear();
                        tmdb_init_popup.rx_session_id = None;
                        tmdb_init_popup.rx_authorization_url = None;
                        tmdb_init_popup.phase = Phase::GetAccessToken;
                    }
                });
                key_event_handler.bind_esc((None, None), "Back".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 0;
                        tmdb_init_popup.input.clear();
                        tmdb_init_popup.rx_session_id = None;
                        tmdb_init_popup.rx_authorization_url = None;
                        tmdb_init_popup.phase = Phase::GetAccessToken;
                    }
                });

                let popup_area = dynamic_popup(
                    frame,
                    Some(9),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Error  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, _, actions_area] = vertical![>=1, ==1, ==1]
                    .areas(add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(
                    Paragraph::new(error.as_str())
                        .wrap(Wrap { trim: true })
                        .centered(),
                    message_area,
                );

                let mouse_area = widgets::action(
                    Action::new(" Back ", ActionTypes::Default, true, true),
                    HorizontalAlignment::Center,
                    actions_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.item = 0;
                            tmdb_init_popup.input.clear();
                            tmdb_init_popup.rx_session_id = None;
                            tmdb_init_popup.rx_authorization_url = None;
                            tmdb_init_popup.phase = Phase::GetAccessToken;
                        }
                    },
                );
            }
        }
    }
}
