use std::{
    path::Path,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::{Flex, HorizontalAlignment, Margin},
    macros::{constraint, horizontal, span, text, vertical},
    style::{
        Style, Stylize,
        palette::{material, tailwind},
    },
    text::Text,
    widgets::Padding,
};
use ratatui_textarea::{TextArea, WrapMode};
use strum::AsRefStr;
use throbber_widgets_tui::{Throbber, ThrobberState};
use trakt::{self, smo::TokenResponse};

use crate::{
    app::App,
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::{Popup, PopupTrait},
    tokens::trakt_tokens::{TraktTokens, UserTokens},
    widgets::{self, Action, ActionType, Hyperlink},
};

#[derive(Default, Debug, AsRefStr)]
#[strum(serialize_all = "title_case")]
pub enum Phase {
    #[default]
    Initializing,
    GetSecrets,
    GettingAuthorizationUrl,
    Authorize(String),
    Finalizing,
    Error(String),
    RefreshingTokens,
    Done,
}

#[derive(Default)]
pub struct TraktInitPopup {
    item:             usize,
    pub tick:         u64,
    pub phase:        Phase,
    throbber_visible: bool,
    can_close:        bool,
    status:           Option<bool>,

    input0:         TextArea<'static>,
    input1:         TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init:              Option<Receiver<anyhow::Result<UserTokens>>>,
    tx_auth_code:         Option<Sender<String>>,
    rx_authorization_url: Option<Receiver<String>>,
    rx_tokens:            Option<Receiver<anyhow::Result<TokenResponse>>>,

    pub user_tokens: Option<UserTokens>,
}

impl TraktInitPopup {
    pub fn new(home_dir: &Path, can_close: bool) -> Self {
        let (tx_init, rx_init) = channel();

        let home_dir_cloned = home_dir.to_path_buf();
        thread::spawn(move || {
            _ = tx_init.send(TraktTokens::init(&home_dir_cloned));
        });

        Self {
            can_close,
            rx_init: Some(rx_init),
            ..Default::default()
        }
    }

    pub fn advance_phase(&mut self) {
        self.item = 0;

        self.phase = match self.phase {
            Phase::Initializing => Phase::GetSecrets,
            Phase::GetSecrets => {
                let client_id = self.input0.lines()[0].clone();
                let client_secret = self.input1.lines()[0].clone();

                self.user_tokens = Some(UserTokens {
                    client_id:     client_id.clone(),
                    client_secret: client_secret.clone(),

                    access_token:  String::default(),
                    refresh_token: String::default(),
                    expires_on:    i64::MAX,
                });

                let (tx_auth_url, rx_auth_url) = channel();
                let (tx_auth_code, rx_auth_code) = channel();
                let (tx_tokens, rx_tokens) = channel();
                thread::spawn(move || {
                    _ = tx_tokens.send(trakt::tokens::get_tokens(
                        &client_id,
                        &client_secret,
                        tx_auth_url,
                        rx_auth_code,
                    ));
                });

                self.tx_auth_code = Some(tx_auth_code);
                self.rx_authorization_url = Some(rx_auth_url);
                self.rx_tokens = Some(rx_tokens);

                Phase::GettingAuthorizationUrl
            }
            Phase::Authorize(_) => {
                if let Some(tx_auth_code) = self.tx_auth_code.take() {
                    let auth_code = self.input0.lines()[0].clone();

                    _ = tx_auth_code.send(auth_code);
                }

                Phase::Finalizing
            }
            Phase::Finalizing | Phase::RefreshingTokens => Phase::Done,
            _ => Phase::Initializing,
        };
    }
}

impl PopupTrait for TraktInitPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.throbber_visible // || matches!(self.phase, Phase::Authorize(_))
    }

    fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        match self.phase {
            Phase::Initializing =>
                if let Some(rx_init_response) = self.rx_init.as_ref() {
                    if let Ok(result) = rx_init_response.try_recv() {
                        if let Ok(user_tokens) = result {
                            if !user_tokens.has_secrets() {
                                self.advance_phase();
                            } else if !user_tokens.has_tokens() {
                                self.advance_phase();
                                self.input0 = TextArea::new(vec![user_tokens.client_id.clone()]);
                                self.input1 =
                                    TextArea::new(vec![user_tokens.client_secret.clone()]);
                                self.advance_phase();
                            } else {
                                self.status = if user_tokens.should_refresh_tokens() {
                                    let client_id = user_tokens.client_id.clone();
                                    let client_secret = user_tokens.client_secret.clone();
                                    let refresh_token = user_tokens.refresh_token.clone();
                                    let (tx_tokens, rx_tokens) = channel();

                                    thread::spawn(move || {
                                        _ = tx_tokens.send(trakt::tokens::refresh_tokens(
                                            &client_id,
                                            &client_secret,
                                            &refresh_token,
                                        ));
                                    });

                                    self.rx_tokens = Some(rx_tokens);

                                    self.phase = Phase::RefreshingTokens;
                                    Some(false)
                                } else {
                                    self.phase = Phase::Done;
                                    Some(true)
                                };

                                self.user_tokens = Some(user_tokens);
                            }
                        } else {
                            self.advance_phase();
                        }
                    }
                },
            Phase::GettingAuthorizationUrl => {
                if let Some(rx_authorization_url) = self.rx_authorization_url.as_ref() {
                    if let Ok(authorization_url) = rx_authorization_url.try_recv() {
                        self.input0.clear();
                        self.status = Some(false);
                        self.phase = Phase::Authorize(authorization_url);
                    }
                }
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(Err(error)) = rx_tokens.try_recv() {
                        self.item = 0;
                        self.phase = Phase::Error(format!("{:#}", error));
                    }
                }
            }
            Phase::Authorize(_) =>
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(Err(error)) = rx_tokens.try_recv() {
                        self.item = 0;
                        self.phase = Phase::Error(format!("{:#}", error));
                    }
                },
            Phase::Finalizing | Phase::RefreshingTokens => {
                if let Some(rx_tokens) = self.rx_tokens.as_ref() {
                    if let Ok(result) = rx_tokens.try_recv() {
                        match result {
                            Ok(token_response) => {
                                if let Some(user_tokens) = self.user_tokens.as_mut() {
                                    user_tokens.access_token = token_response.access_token;
                                    user_tokens.refresh_token = token_response.refresh_token;
                                    user_tokens.expires_on =
                                        token_response.created_at + token_response.expires_in;
                                }
                                self.status = Some(true);

                                self.advance_phase();
                            }
                            Err(error) => {
                                self.item = 0;
                                self.phase = Phase::Error(format!("{:#}", error));
                            }
                        }
                    }
                }
            }
            _ => (),
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

        self.throbber_visible = false;
        match &self.phase {
            Phase::Initializing
            | Phase::GettingAuthorizationUrl
            | Phase::Finalizing
            | Phase::RefreshingTokens
            | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(6, 28, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [_, message_area, _, throbber_area] =
                    vertical![>=1, ==2, >=1, ==1].areas(popup_area);
                frame.render_widget(
                    span!(self.phase.as_ref()).into_centered_line(),
                    message_area,
                );

                frame.render_stateful_widget(
                    Throbber::default()
                        .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                        .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
                    throbber_area.centered(constraint!(==1), constraint!(==1)),
                    &mut self.throbber_state,
                );
            }
            Phase::GetSecrets => {
                let input_valid =
                    !(self.input0.lines()[0].is_empty() || self.input1.lines()[0].is_empty());

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
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
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 2;
                    }
                });
                key_event_handler.bind_enter((None, None), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item += 1;
                    }
                });
                if input_valid {
                    key_event_handler.bind_enter(
                        (None, Some(1)),
                        "Confirm".into(),
                        move |app, _| {
                            if let Some(Popup::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                trakt_init_popup.advance_phase();
                            }
                        },
                    );
                    key_event_handler.bind_enter(
                        (None, Some(2)),
                        "Confirm".into(),
                        move |app, _| {
                            if let Some(Popup::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                trakt_init_popup.advance_phase();
                            }
                        },
                    );
                } else {
                    key_event_handler.bind_enter((None, Some(2)), "".into(), |_, _| {});
                }

                key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            trakt_init_popup.input0.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(1)), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            trakt_init_popup.input1.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(11, 44, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let [ci_input_area, cs_input_area, _] = vertical![==3, ==3, >=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                let ci_input_selected = self.item == 0;
                widgets::input_field(
                    true,
                    ci_input_selected,
                    !self.input0.is_empty(),
                    &mut self.input0,
                    WrapMode::None,
                    frame,
                    ci_input_area,
                    " Client ID ",
                    "Enter the Client ID",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    ci_input_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                        }
                    },
                );

                let cs_input_selected = self.item == 1;
                widgets::input_field(
                    true,
                    cs_input_selected,
                    !self.input1.is_empty(),
                    &mut self.input1,
                    WrapMode::None,
                    frame,
                    cs_input_area,
                    " Client Secret ",
                    "Enter the Client Secret",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    cs_input_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 1;
                        }
                    },
                );

                let mouse_area = widgets::action(
                    Action::new(" Confirm ", ActionType::Normal, self.item == 2, input_valid),
                    HorizontalAlignment::Right,
                    true,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                if input_valid {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        mouse_area,
                        |app, _| {
                            if let Some(Popup::TraktInit(trakt_init_popup)) =
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
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
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
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 3;
                    }
                });
                key_event_handler.bind_esc((None, Some(3)), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_authorization_url = None;
                        trakt_init_popup.tx_auth_code = None;
                        trakt_init_popup.input0.clear();
                        trakt_init_popup.input1.clear();
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                });
                key_event_handler.bind_enter((None, Some(0)), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 1;
                    }
                });
                if input_valid {
                    key_event_handler.bind_enter((None, Some(1)), "Confirm".into(), |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.advance_phase();
                        }
                    });
                }
                key_event_handler.bind_enter((None, Some(2)), "Skip".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.phase = Phase::Done;
                    }
                });
                key_event_handler.bind_enter((None, Some(3)), "Back".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_authorization_url = None;
                        trakt_init_popup.tx_auth_code = None;
                        trakt_init_popup.input0.clear();
                        trakt_init_popup.input1.clear();
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                });
                key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            trakt_init_popup.input0.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(12, 48, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let skip_mouse_area = widgets::action(
                    Action::new(" Skip ", ActionType::Normal, self.item == 2, true),
                    HorizontalAlignment::Right,
                    false,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    skip_mouse_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    },
                );

                let back_mouse_area = widgets::action(
                    Action::new(" Back ", ActionType::Normal, self.item == 3, true),
                    HorizontalAlignment::Left,
                    false,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    back_mouse_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_authorization_url = None;
                            trakt_init_popup.tx_auth_code = None;
                            trakt_init_popup.input0.clear();
                            trakt_init_popup.input1.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );

                let [_, hyperlink_area, _, input_area, _] = vertical![==1, ==3, >=1, ==3, ==1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                let hyperlink_text = "  Click to Authorize  ";
                let [hyperlink_area] = horizontal![==(hyperlink_text.len() as u16)]
                    .flex(Flex::Center)
                    .areas(hyperlink_area);
                frame.render_widget(
                    Hyperlink {
                        text: text![
                            " ".repeat(hyperlink_text.len()),
                            hyperlink_text,
                            " ".repeat(hyperlink_text.len())
                        ]
                        .fg(material::GREEN.c100)
                        .bg(material::BLUE.c800),
                        url:  authorization_url.clone(),
                    },
                    hyperlink_area,
                );

                widgets::input_field(
                    true,
                    self.item == 0,
                    input_valid,
                    &mut self.input0,
                    WrapMode::None,
                    frame,
                    helpers::add_padding(input_area, Padding::horizontal(8)),
                    " Authorization Code ",
                    "Enter the authorization code",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    input_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                        }
                    },
                );

                let confirm_mouse_area = widgets::action(
                    Action::new(" Confirm ", ActionType::Normal, self.item == 1, input_valid),
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
                            if let Some(Popup::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                trakt_init_popup.advance_phase();
                            }
                        },
                    );
                }
            }
            Phase::Error(error) => {
                let back = |app: &mut App, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_authorization_url = None;
                        trakt_init_popup.tx_auth_code = None;
                        trakt_init_popup.input0.clear();
                        trakt_init_popup.input1.clear();
                        trakt_init_popup.user_tokens = None;
                        trakt_init_popup.status = None;
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                };
                key_event_handler.bind_esc((None, None), "Back".into(), back);
                key_event_handler.bind_enter((None, Some(0)), "Back".into(), back);
                if self.status.is_some() {
                    key_event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    });
                    key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                crate::key_event_handler::Data::Direction(true, _) => {
                                    trakt_init_popup.item += 1;
                                    if trakt_init_popup.item > 1 {
                                        trakt_init_popup.item = 0;
                                    }
                                }
                                crate::key_event_handler::Data::Direction(false, _) => {
                                    trakt_init_popup.item =
                                        trakt_init_popup.item.checked_sub(1).unwrap_or(1);
                                }
                                _ => {}
                            }
                        }
                    });
                }

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(11, 44, frame.area()),
                    " Error ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [message_area, _] = vertical![>=1, ==1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));
                frame.render_widget(
                    Text::from_iter(helpers::wrap_text(
                        error.as_str(),
                        message_area.width as usize,
                    ))
                    .centered(),
                    message_area,
                );

                if self.status.is_some() {
                    let skip_mouse_area = widgets::action(
                        Action::new(" Skip ", ActionType::Normal, self.item == 1, true),
                        HorizontalAlignment::Right,
                        false,
                        popup_area,
                        frame,
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        skip_mouse_area,
                        |app, _| {
                            if let Some(Popup::TraktInit(trakt_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                trakt_init_popup.phase = Phase::Done;
                            }
                        },
                    );
                }

                let mouse_area = widgets::action(
                    Action::new(" Back ", ActionType::Default, self.item == 0, true),
                    HorizontalAlignment::Center,
                    true,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    back,
                );
            }
        }
    }
}
