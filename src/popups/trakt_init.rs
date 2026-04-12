use crate::{
    helpers::{add_padding, center_rect, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    trakt::{self, TokenResponse},
    tokens::trakt_tokens::{TraktTokens, UserTokens},
};
use itertools::Itertools;
use ratatui::{
    Frame, layout::*, macros::{constraint, vertical, horizontal, span, text}, prelude::*, style::palette::{material, tailwind}, widgets::*
};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};
use ratatui_textarea::TextArea;
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
    home_dir: PathBuf,
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
            home_dir: home_dir.clone(),
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
                self.input0.set_style(Style::new().fg(if ci_input_selected {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                }));
                self.input0.set_cursor_style(
                    Style::new()
                        .fg(if ci_input_selected {
                            tailwind::SLATE.c300
                        } else {
                            tailwind::STONE.c400
                        })
                        .add_modifier(if ci_input_selected {
                            Modifier::REVERSED
                        } else {
                            Modifier::default()
                        }),
                );
                self.input0.set_block(
                    Block::bordered()
                        .border_type(ratatui::widgets::BorderType::Thick)
                        .fg(if ci_input_selected {
                            material::BLUE.c500
                        } else {
                            tailwind::STONE.c500
                        })
                        .title(" Client ID ")
                        .title_style(Style::new().fg(if ci_input_selected {
                            material::BLUE.c400
                        } else {
                            material::BLUE.c600
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input0.set_placeholder_text("Enter the Client ID");
                self.input0
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
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
                frame.render_widget(
                    &self.input0,
                    ci_input_area,
                );

                let cs_input_selected = self.item == 1;
                self.input1.set_style(Style::new().fg(if cs_input_selected {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                }));
                self.input1.set_cursor_style(
                    Style::new()
                        .fg(if cs_input_selected {
                            tailwind::SLATE.c300
                        } else {
                            tailwind::STONE.c400
                        })
                        .add_modifier(if cs_input_selected {
                            Modifier::REVERSED
                        } else {
                            Modifier::default()
                        }),
                );
                self.input1.set_block(
                    Block::bordered()
                        .border_type(ratatui::widgets::BorderType::Thick)
                        .fg(if cs_input_selected {
                            material::BLUE.c500
                        } else {
                            tailwind::STONE.c500
                        })
                        .title(" Client Secret ")
                        .title_style(Style::new().fg(if cs_input_selected {
                            material::BLUE.c400
                        } else {
                            material::BLUE.c600
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input1.set_placeholder_text("Enter the Client Secret");
                self.input1
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
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
                frame.render_widget(
                    &self.input1,
                    cs_input_area,
                );

                let confirm = span!(" Confirm ")
                    .fg(if input_valid {
                        if self.item == 2 {
                            tailwind::SLATE.c200
                        } else {
                            tailwind::SLATE.c300
                        }
                    } else {
                        tailwind::SLATE.c500
                    })
                    .bg(if input_valid {
                        if self.item == 2 {
                            material::BLUE.c600
                        } else {
                            material::BLUE.c900
                        }
                    } else {
                        if self.item == 2 {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    });
                let mouse_area = actions_area
                    .offset(Offset::new(actions_area.width as i32 - (confirm.width() as i32), 0))
                    .resize(Size::new(confirm.width() as u16, 1));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
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
                frame.render_widget(Line::from(confirm).right_aligned(), actions_area);
            }
            Phase::Authorize(authorization_url) => {
                let input_valid = !self.input0.lines()[0].is_empty();

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
                    move |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.item = 1;
                        }
                    },
                );
                key_event_handler.bind_enter(
                    (None, Some(1)),
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

                let skip = span!(" Skip ")
                    .fg(if self.item == 2 {
                        tailwind::SLATE.c200
                    } else {
                        tailwind::SLATE.c300
                    })
                    .bg(if self.item == 2 {
                        material::BLUE.c700
                    } else {
                        material::BLUE.c900
                    });
                let mouse_area = popup_area
                    .offset(Offset::new(popup_area.width as i32 - (skip.width() as i32), 0))
                    .resize(Size::new(skip.width() as u16, 1));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    |app, _| {
                        if let Some(Popups::TraktInit(trakt_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            trakt_init_popup.phase = Phase::Done;
                        }
                    },
                );
                frame.render_widget(Line::from(skip).right_aligned(), popup_area);

                let back = span!(" Back ")
                    .fg(if self.item == 3 {
                        tailwind::SLATE.c200
                    } else {
                        tailwind::SLATE.c300
                    })
                    .bg(if self.item == 3 {
                        material::BLUE.c600
                    } else {
                        material::BLUE.c900
                    });
                let mouse_area = popup_area
                    .resize(Size::new(back.width() as u16, 1));
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
                frame.render_widget(Line::from(back), popup_area);

                let [_, message_area, _, input_area, _, actions_area] =
                    vertical![==1, ==3, >=1, ==3, ==1, ==1].areas(add_padding(popup_area, Padding::proportional(1)));

                let hyperlink = Hyperlink::new(text!["                      ", "  Click to Authorize  ", "                      "].fg(material::GREEN.c100).bg(material::BLUE.c800), authorization_url);
                let [message_area] = horizontal![==(hyperlink.text.width() as u16)].flex(Flex::Center).areas(message_area);

                frame.render_widget(&hyperlink, message_area);

                self.input0.set_style(Style::new().fg(if self.item == 0 {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                }));
                self.input0.set_cursor_style(
                    Style::new()
                        .fg(if self.item == 0 {
                            tailwind::SLATE.c300
                        } else {
                            tailwind::STONE.c400
                        })
                        .add_modifier(if self.item == 0 {
                            Modifier::REVERSED
                        } else {
                            Modifier::default()
                        }),
                );
                self.input0.set_block(
                    Block::bordered()
                        .border_type(ratatui::widgets::BorderType::Thick)
                        .fg(if self.item == 0 {
                            material::BLUE.c500
                        } else {
                            tailwind::STONE.c500
                        })
                        .title(" Client ID ")
                        .title_style(Style::new().fg(if self.item == 0 {
                            material::BLUE.c400
                        } else {
                            material::BLUE.c600
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input0.set_placeholder_text("Enter the Client ID");
                self.input0
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
                frame.render_widget(
                    &self.input0,
                    add_padding(
                        input_area,
                        Padding::horizontal(1),
                    ),
                );

                let confirm = span!(" Confirm ")
                    .fg(if input_valid {
                        if self.item == 1 {
                            tailwind::SLATE.c200
                        } else {
                            tailwind::SLATE.c300
                        }
                    } else {
                        tailwind::SLATE.c500
                    })
                    .bg(if input_valid {
                        if self.item == 1 {
                            material::BLUE.c600
                        } else {
                            material::BLUE.c900
                        }
                    } else {
                        if self.item == 1 {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    });
                let mouse_area = actions_area
                    .offset(Offset::new(actions_area.width as i32 - (confirm.width() as i32), 0))
                    .resize(Size::new(confirm.width() as u16, 1));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
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
                frame.render_widget(Line::from(confirm).right_aligned(), actions_area);
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

                let back = span!(" Back ")
                    .fg(tailwind::SLATE.c200)
                    .bg(material::BLUE.c600);
                let mouse_area = actions_area
                    .resize(Size::new(back.width() as u16, 1));
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

                frame.render_widget(Line::from(back), actions_area);
            }
        }
    }
}

struct Hyperlink<'content> {
    text: Text<'content>,
    url: String,
}

impl<'content> Hyperlink<'content> {
    fn new(text: impl Into<Text<'content>>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl Widget for &Hyperlink<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        (&self.text).render(area, buffer);

        // this is a hacky workaround for https://github.com/ratatui/ratatui/issues/902, a bug
        // in the terminal code that incorrectly calculates the width of ANSI escape sequences. It
        // works by rendering the hyperlink as a series of 2-character chunks, which is the
        // calculated width of the hyperlink text.
        for (j, line) in self.text.lines.clone().into_iter().enumerate() {
            for (i, two_chars) in line
                .to_string()
                .chars()
                .chunks(2)
                .into_iter()
                .enumerate()
            {
                let text = two_chars.collect::<String>();
                let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
                buffer[(area.x + i as u16 * 2, area.y + j as u16)].set_symbol(hyperlink.as_str());
            }
        }
    }
}
