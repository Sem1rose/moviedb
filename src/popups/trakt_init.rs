use std::{
    path::Path,
    sync::mpsc::{Receiver, channel},
    thread,
};

use ratatui::{
    Frame,
    layout::{Flex, HorizontalAlignment, Margin},
    macros::{constraint, horizontal, line, span, text, vertical},
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

use crate::{
    app::App,
    event_handler::{self, EventHandler},
    helpers,
    image_backend::RatatuiImage,
    popups::{Popup, PopupTrait},
    sources::trakt::{self, smo::TokenResponse},
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
    Authorize((String, String)),
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

    input:          TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init:                   Option<Receiver<anyhow::Result<UserTokens>>>,
    rx_authorization_url_code: Option<Receiver<(String, String)>>,
    rx_tokens:                 Option<Receiver<anyhow::Result<TokenResponse>>>,

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
                let client_id = self.input.lines()[0].clone();

                self.user_tokens = Some(UserTokens {
                    client_id: client_id.clone(),

                    access_token:  String::default(),
                    refresh_token: String::default(),
                    expires_on:    i64::MAX,
                });

                let (tx_auth_url_code, rx_auth_url_cide) = channel();
                let (tx_tokens, rx_tokens) = channel();
                thread::spawn(move || {
                    _ = tx_tokens.send(trakt::tokens::get_tokens(&client_id, tx_auth_url_code));
                });

                self.rx_authorization_url_code = Some(rx_auth_url_cide);
                self.rx_tokens = Some(rx_tokens);

                Phase::GettingAuthorizationUrl
            }
            Phase::Authorize(_) | Phase::RefreshingTokens => Phase::Done,
            _ => Phase::Initializing,
        };
    }
}

impl PopupTrait for TraktInitPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.throbber_visible
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
                                self.input = TextArea::new(vec![user_tokens.client_id.clone()]);
                                self.advance_phase();
                            } else {
                                self.status = if user_tokens.should_refresh_tokens() {
                                    let client_id = user_tokens.client_id.clone();
                                    let refresh_token = user_tokens.refresh_token.clone();
                                    let (tx_tokens, rx_tokens) = channel();

                                    thread::spawn(move || {
                                        _ = tx_tokens.send(trakt::tokens::refresh_tokens(
                                            &client_id,
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
                if let Some(rx_authorization_url) = self.rx_authorization_url_code.as_ref() {
                    if let Ok(authorization_url) = rx_authorization_url.try_recv() {
                        self.input.clear();
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
            Phase::Authorize(_) | Phase::RefreshingTokens => {
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
        event_handler: &mut EventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        event_handler.clear();
        if self.can_close {
            event_handler.bind_esc((None, None), "Close".into(), |app, _| {
                app.drawer.close_popup();
            });
            event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
                app.drawer.close_popup();
            });
            event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    app.drawer.close_popup();
                },
            );
        } else {
            // event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            //     app.quit = true;
            // });
            event_handler.bind_key((None, None), 'q', "Quit".into(), |app, _| {
                app.quit = true;
            });
        }

        self.throbber_visible = false;
        match &self.phase {
            Phase::Initializing
            | Phase::GettingAuthorizationUrl
            | Phase::RefreshingTokens
            | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(7, 36, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                event_handler.bind_mouse_button_down(
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
                let input_valid = !self.input.lines()[0].is_empty();

                event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::event_handler::Data::Direction(_, _) => {
                                trakt_init_popup.item = (trakt_init_popup.item == 0) as usize;
                            }
                            _ => {}
                        }
                    }
                });
                event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 1;
                    }
                });
                if input_valid {
                    event_handler.bind_enter((None, None), "Confirm".into(), move |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.advance_phase();
                        }
                    });
                }

                event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let event_handler::Data::Key(key_event) = data {
                            trakt_init_popup.input.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(8, 40, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let [input_area, _] = vertical![==3, >=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                let input_selected = self.item == 0;
                widgets::input_field(
                    true,
                    input_selected,
                    !self.input.is_empty(),
                    &mut self.input,
                    WrapMode::None,
                    frame,
                    input_area,
                    " Client ID ",
                    "Enter the Client ID",
                    None,
                );
                event_handler.bind_mouse_button_down(
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

                let mouse_area = widgets::action(
                    Action::new(" Confirm ", ActionType::Normal, self.item == 1, input_valid),
                    HorizontalAlignment::Right,
                    true,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                if input_valid {
                    event_handler.bind_mouse_button_down(
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
            Phase::Authorize((authorization_url, authorization_code)) => {
                event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::event_handler::Data::Direction(true, _) => {
                                trakt_init_popup.item += 1;
                                if trakt_init_popup.item > 2 {
                                    trakt_init_popup.item = 0;
                                }
                            }
                            crate::event_handler::Data::Direction(false, _) => {
                                trakt_init_popup.item =
                                    trakt_init_popup.item.checked_sub(1).unwrap_or(2);
                            }
                            _ => {}
                        }
                    }
                });
                event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if trakt_init_popup.item == 2 {
                            trakt_init_popup.item = 0;
                        } else {
                            trakt_init_popup.item = 2;
                        }
                    }
                });
                event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.phase = Phase::Done;
                    }
                });
                event_handler.bind_enter((None, Some(2)), "Back".into(), |app, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_authorization_url_code = None;
                        trakt_init_popup.input.clear();
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(10, 40, frame.area()),
                    " Trakt Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let skip_mouse_area = widgets::action(
                    Action::new(" Skip ", ActionType::Normal, self.item == 1, true),
                    HorizontalAlignment::Right,
                    false,
                    popup_area,
                    frame,
                );
                event_handler.bind_mouse_button_down(
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
                    Action::new(" Back ", ActionType::Normal, self.item == 2, true),
                    HorizontalAlignment::Left,
                    false,
                    popup_area,
                    frame,
                );
                event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    back_mouse_area,
                    |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 0;
                            trakt_init_popup.rx_tokens = None;
                            trakt_init_popup.rx_authorization_url_code = None;
                            trakt_init_popup.input.clear();
                            trakt_init_popup.phase = Phase::GetSecrets;
                        }
                    },
                );

                let [_, hyperlink_area, _, code_area] = vertical![==1, ==3, ==1, ==1]
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

                frame.render_widget(
                    line![
                        "User Code: ",
                        authorization_code.as_str().bold().fg(tailwind::VIOLET.c300)
                    ]
                    .centered(),
                    code_area,
                );
            }
            Phase::Error(error) => {
                let back = |app: &mut App, _| {
                    if let Some(Popup::TraktInit(trakt_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        trakt_init_popup.item = 0;
                        trakt_init_popup.rx_tokens = None;
                        trakt_init_popup.rx_authorization_url_code = None;
                        trakt_init_popup.input.clear();
                        trakt_init_popup.user_tokens = None;
                        trakt_init_popup.status = None;
                        trakt_init_popup.phase = Phase::GetSecrets;
                    }
                };
                event_handler.bind_esc((None, None), "Back".into(), back);
                event_handler.bind_enter((None, Some(0)), "Back".into(), back);
                if self.status.is_some() {
                    event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    });
                    event_handler.bind_tab((None, None), "".into(), |app, data| {
                        if let Some(Popup::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                crate::event_handler::Data::Direction(true, _) => {
                                    trakt_init_popup.item += 1;
                                    if trakt_init_popup.item > 1 {
                                        trakt_init_popup.item = 0;
                                    }
                                }
                                crate::event_handler::Data::Direction(false, _) => {
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
                event_handler.bind_mouse_button_down(
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
                    event_handler.bind_mouse_button_down(
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
                event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    back,
                );
            }
        }
    }
}
