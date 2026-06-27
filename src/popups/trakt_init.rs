use crate::{
    helpers::{add_padding, center_rect, dynamic_popup}, key_event_handler::{self, KeyEventHandler}, popups::Popups, tokens::trakt_tokens::{TraktTokens, UserTokens}, trakt::{self, TokenResponse}, widgets::{self, Action, ActionTypes}
};
use ratatui::{
    Frame, layout::*, macros::{constraint, vertical, horizontal, text}, prelude::*, style::palette::{material, tailwind}, widgets::*
};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};
use ratatui_textarea::{TextArea, WrapMode};
use throbber_widgets_tui::{Throbber, ThrobberState};

#[derive(Default, Debug)]
pub enum Phase {
    #[default]
    Initializing,
    GetSecrets,
    GettingAuthorizationUrl,
    Authorize(String),
    Finalize,
    Error(String),
    RefreshingTokens,
    Done,
}

#[derive(Default)]
pub struct TraktInitPopup {
    item: usize,
    pub tick: u64,
    pub phase: Phase,
    throbber_visible: bool,
    one_shot: bool,

    input0: TextArea<'static>,
    input1: TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init: Option<Receiver<anyhow::Result<UserTokens>>>,
    tx_auth_code: Option<Sender<String>>,
    rx_auth_url: Option<Receiver<String>>,
    rx_tokens: Option<Receiver<anyhow::Result<TokenResponse>>>,

    pub tokens: Option<UserTokens>
}

impl TraktInitPopup {
    pub fn new(home_dir: &PathBuf, one_shot: bool) -> Self {
        let (tx_init, rx_init) = channel();
        let home_dir_cloned = home_dir.clone();

        thread::spawn(move || {
            _ = tx_init.send(TraktTokens::init(&home_dir_cloned));
        });

        Self {
            one_shot,
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
        self.item = 0;

        self.phase = match self.phase {
            Phase::Initializing => Phase::GetSecrets,
            Phase::GetSecrets => {
                let client_id = self.input0.lines()[0].clone();
                let client_secret = self.input1.lines()[0].clone();

                self.tokens = Some(UserTokens {
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),

                    access_token: String::default(),
                    refresh_token: String::default(),
                    expires_on: i64::MAX,
                });

                let (tx_auth_url, rx_auth_url) = channel();
                let (tx_auth_code, rx_auth_code) = channel();
                let (tx_tokens, rx_tokens) = channel();
                thread::spawn(move || {
                    _ = tx_tokens
                        .send(trakt::get_tokens(&client_id, &client_secret, tx_auth_url, rx_auth_code));
                });

                self.tx_auth_code = Some(tx_auth_code);
                self.rx_auth_url = Some(rx_auth_url);
                self.rx_tokens = Some(rx_tokens);

                Phase::GettingAuthorizationUrl
            }
            Phase::Authorize(_) => {
                if let Some(tx_auth_code) = self.tx_auth_code.take() {
                    let auth_code = self.input0.lines()[0].clone();

                    _ = tx_auth_code.send(auth_code);
                }

                Phase::Finalize
            },
            Phase::Finalize | Phase::RefreshingTokens => Phase::Done,
            _ => Phase::Initializing,
        };
    }

    pub fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        match self.phase {
            Phase::Initializing => {
                if let Some(rx_init_response) = self.rx_init.as_ref() {
                    if let Ok(result) = rx_init_response.try_recv() {
                        if let Ok(tokens) = result {
                            if !tokens.has_secrets() {
                                self.advance_phase();
                            } else if !tokens.has_tokens() {
                                self.advance_phase();
                                self.input0 = TextArea::new(vec![tokens.client_id.clone()]);
                                self.input1 = TextArea::new(vec![tokens.client_secret.clone()]);
                                self.advance_phase();
                            } else {
                                self.tokens = Some(tokens);

                                if let Some(tokens) = self.tokens.as_ref() {
                                    if trakt::should_refresh_tokens(tokens) {
                                        let client_id = tokens.client_id.clone();
                                        let client_secret = tokens.client_secret.clone();
                                        let refresh_token = tokens.refresh_token.clone();
                                        let (tx_tokens, rx_tokens) = channel();

                                        thread::spawn(move || {
                                            _ = tx_tokens
                                                .send(trakt::refresh_tokens(&client_id, &client_secret, &refresh_token));
                                        });

                                        self.rx_tokens = Some(rx_tokens);

                                        self.phase = Phase::RefreshingTokens;
                                    } else {
                                        self.phase = Phase::Done;
                                    }
                                }
                            }
                        } else {
                            self.advance_phase();
                        }
                    }
                }
            }
            Phase::GettingAuthorizationUrl => {
                if let Some(rx_auth_url) = self.rx_auth_url.as_ref() {
                    if let Ok(url) = rx_auth_url.try_recv() {
                            self.input0.clear();
                            self.phase = Phase::Authorize(url);
                    }
                }
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(result) = rx_tokens.try_recv() {
                        if let Err(error) = result {
                            self.phase = Phase::Error(format!("{:#}", error));
                        }
                    }
                }
            }
            Phase::Authorize(_) => {
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(result) = rx_tokens.try_recv() {
                        if let Err(error) = result {
                            self.phase = Phase::Error(format!("{:#}", error));
                        }
                    }
                }
            }
            Phase::Finalize => {
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(result) = rx_tokens.try_recv() {
                        match result {
                            Ok(token_response) => {
                                if let Some(user_tokens) = self.tokens.as_mut() {
                                    user_tokens.access_token = token_response.access_token;
                                    user_tokens.refresh_token = token_response.refresh_token;
                                    user_tokens.expires_on = token_response.created_at + token_response.expires_in;
                                }

                                self.advance_phase();
                            }
                            Err(error) => {
                                self.phase = Phase::Error(format!("{:#}", error));
                            }
                        }
                    }
                }
            }
            Phase::RefreshingTokens => {
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(result) = rx_tokens.try_recv() {
                        match result {
                            Ok(token_response) => {
                                if let Some(user_tokens) = self.tokens.as_mut() {
                                    user_tokens.access_token = token_response.access_token;
                                    user_tokens.refresh_token = token_response.refresh_token;
                                    user_tokens.expires_on = token_response.created_at + token_response.expires_in;
                                }

                                self.advance_phase();
                            }
                            Err(error) => self.phase = Phase::Error(format!("{:#}", error))
                        }
                    }
                }
            }
            _ => (),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        if self.one_shot {
            key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
                app.drawer.close_popups();
            });
            key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
                app.drawer.close_popups();
            });
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    app.drawer.close_popups();
                },
            );
        } else {
            key_event_handler.bind_key((None, None), 'q', "Quit".into(), |app, _| {
                app.quit = true;
            });
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    app.quit = true;
                },
            );
        }

        self.throbber_visible = false;
        match &self.phase {
            Phase::Initializing
            | Phase::GettingAuthorizationUrl
            | Phase::Finalize
            | Phase::RefreshingTokens
            | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = dynamic_popup(
                    frame,
                    Some(5),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Trakt Authentication  ",
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
                frame.render_widget(Paragraph::new(if matches!(self.phase, Phase::RefreshingTokens) { "Refreshing tokens" } else { "Processing" }).centered(), message_area);

                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                        .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                    center_rect(throbber_area, constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::GetSecrets => {
                let input_valid = !(self.input0.lines()[0].is_empty() || self.input1.lines()[0].is_empty());

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                trakt_init_popup.item += 1;
                                if trakt_init_popup.item > 2 {
                                    trakt_init_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                trakt_init_popup.item =
                                    trakt_init_popup.item.checked_sub(1).unwrap_or(2);
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_enter((None, None), "".into(), |app, _| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item += 1;
                    }
                });
                key_event_handler.bind_enter(
                    (None, Some(2)),
                    "Confirm".into(),
                move |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if input_valid {
                                trakt_init_popup.advance_phase();
                            }
                        }
                    },
                );
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 2;
                    }
                });
                key_event_handler.bind_input_field(
                    (None, Some(0)),
                    "".into(),
                    |app, data| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Key(key_event) => {
                                    trakt_init_popup.input0.input(key_event);
                                }
                                _ => (),
                            }
                        }
                    },
                );
                key_event_handler.bind_input_field(
                    (None, Some(1)),
                    "".into(),
                    |app, data| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Key(key_event) => {
                                    trakt_init_popup.input1.input(key_event);
                                }
                                _ => (),
                            }
                        }
                    },
                );

                let popup_area = dynamic_popup(
                    frame,
                    Some(10),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Trakt Authentication  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let [ci_input_area, cs_input_area, _, actions_area] =
                    vertical![==3, ==3, >=1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));

                let ci_input_selected = self.item == 0;
                widgets::input_field(ci_input_selected, !self.input0.is_empty(), &mut self.input0, WrapMode::None, frame, ci_input_area, (0, 0), " Client ID ", "Enter the Client ID");
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    ci_input_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                        }
                    },
                );

                let cs_input_selected = self.item == 1;
                widgets::input_field(cs_input_selected, !self.input1.is_empty(), &mut self.input1, WrapMode::None, frame, cs_input_area, (0, 0), " Client Secret ", "Enter the Client Secret");
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    cs_input_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 1;
                        }
                    },
                );

                let mouse_area = widgets::action(Action::new(" Confirm ", ActionTypes::Normal, self.item == 2, input_valid), HorizontalAlignment::Right, actions_area, frame);
                if input_valid {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        |app, _| {
                            if let Some(Popups::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                    trakt_init_popup.advance_phase();
                            }
                        },
                    );
                }
            }
            Phase::Authorize(authorization_url) => {
                let input_valid = !self.input0.is_empty();

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                trakt_init_popup.item += 1;
                                if trakt_init_popup.item > 3 {
                                    trakt_init_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                trakt_init_popup.item =
                                    trakt_init_popup.item.checked_sub(1).unwrap_or(3);
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 3;
                    }
                });
                key_event_handler.bind_esc((None, Some(3)), "".into(), |app, _| {
                    if let Some(Popups::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_auth_url = None;
                        trakt_init_popup.tx_auth_code = None;
                        trakt_init_popup.input0.clear();
                        trakt_init_popup.input1.clear();
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                });
                key_event_handler.bind_enter(
                    (None, Some(0)),
                    "".into(),
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 1;
                        }
                    },
                );
                if input_valid {
                    key_event_handler.bind_enter(
                        (None, Some(1)),
                        "Confirm".into(),
                        |app, _| {
                            if let Some(Popups::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                    trakt_init_popup.advance_phase();
                            }
                        },
                    );
                }
                key_event_handler.bind_enter(
                    (None, Some(2)),
                    "Skip".into(),
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    },
                );
                key_event_handler.bind_enter(
                    (None, Some(3)),
                    "Back".into(),
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_auth_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );
                key_event_handler.bind_input_field(
                    (None, Some(0)),
                    "".into(),
                    |app, data| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                key_event_handler::Data::Key(key_event) => {
                                    trakt_init_popup.input0.input(key_event);
                                }
                                _ => (),
                            }
                        }
                    },
                );

                let popup_area = dynamic_popup(
                    frame,
                    Some(12),
                    4.0,
                    tailwind::BLUE.c950,
                    "  Trakt Authentication  ",
                    Style::new().fg(material::YELLOW.c800),
                    Alignment::Center,
                    Style::new().fg(tailwind::VIOLET.c950),
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let skip_mouse_area = widgets::action(Action::new(" Skip ", ActionTypes::Normal, self.item == 2, true), HorizontalAlignment::Right, popup_area, frame);
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    skip_mouse_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    },
                );

                let back_mouse_area = widgets::action(Action::new(" Back ", ActionTypes::Normal, self.item == 3, true), HorizontalAlignment::Left, popup_area, frame);
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    back_mouse_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_auth_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );

                let [_, message_area, _, input_area, _, actions_area] =
                    vertical![==1, ==3, >=1, ==3, ==1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));

                let hyperlink_text = "  Click to Authorize  ";
                let [message_area] = horizontal![==(hyperlink_text.len() as u16)].flex(Flex::Center).areas(message_area);
                widgets::hyperlink(text![" ".repeat(hyperlink_text.len()), hyperlink_text, " ".repeat(hyperlink_text.len())].fg(material::GREEN.c100).bg(material::BLUE.c800), authorization_url, message_area, frame);

                widgets::input_field(self.item == 0, input_valid, &mut self.input0, WrapMode::None, frame, input_area, (8, 8), " Authorization Code ", "Enter the authorization code");
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    input_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                        }
                    },
                );

                let confirm_mouse_area = widgets::action(Action::new(" Confirm ", ActionTypes::Normal, self.item == 1, input_valid), HorizontalAlignment::Right, actions_area, frame);
                if input_valid {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        confirm_mouse_area,
                        |app, _| {
                            if let Some(Popups::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                    trakt_init_popup.advance_phase();
                            }
                        },
                    );
                }
            }
            Phase::Error(error) => {
                key_event_handler.bind_enter(
                    (None, None),
                    "Back".into(),
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_auth_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );
                key_event_handler.bind_esc(
                    (None, None),
                    "Back".into(),
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_auth_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );

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
                let [message_area, _, actions_area] =
                    vertical![>=1, ==1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(
                    Paragraph::new(error.as_str())
                        .wrap(Wrap { trim: true })
                        .centered(),
                    message_area,
                );

                let mouse_area = widgets::action(Action::new(" Back ", ActionTypes::Default, true, true), HorizontalAlignment::Center, actions_area, frame);
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_auth_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );
            }
        }
    }
}
