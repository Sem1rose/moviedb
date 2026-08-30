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
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::{Popup, PopupTrait},
    tokens::simkl_tokens::{SimklTokens, UserTokens},
    widgets::{self, Action, ActionType, Hyperlink},
};

#[derive(Default, Debug, AsRefStr)]
#[strum(serialize_all = "title_case")]
pub enum Phase {
    #[default]
    Initializing,
    GetClientInfo,
    GettingUserCode,
    Authorize(String),
    Finalizing,
    Error(String),
    Done,
}

#[derive(Default)]
pub struct SimklInitPopup {
    pub tick:         u64,
    pub phase:        Phase,
    throbber_visible: bool,
    item:             usize,
    status:           Option<bool>,
    can_close:        bool,

    client_id_input:     TextArea<'static>,
    client_secret_input: TextArea<'static>,
    app_name_input:      TextArea<'static>,
    app_version_input:   TextArea<'static>,
    throbber_state:      ThrobberState,

    rx_init:         Option<Receiver<anyhow::Result<UserTokens>>>,
    rx_user_code:    Option<Receiver<String>>,
    rx_access_token: Option<Receiver<anyhow::Result<String>>>,

    pub user_tokens: Option<UserTokens>,
}

impl SimklInitPopup {
    pub fn new(home_dir: &Path, can_close: bool) -> Self {
        let (tx_init, rx_init) = channel();

        let home_dir_cloned = home_dir.to_path_buf();
        thread::spawn(move || {
            _ = tx_init.send(SimklTokens::init(&home_dir_cloned));
        });

        Self {
            can_close,
            rx_init: Some(rx_init),
            ..Default::default()
        }
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Initializing => Phase::GetClientInfo,
            Phase::GetClientInfo => {
                let client_id = self.client_id_input.lines()[0].clone();
                let client_secret = self.client_secret_input.lines()[0].clone();
                let app_name = self.app_name_input.lines()[0].clone();
                let app_version = self.app_version_input.lines()[0].clone();

                self.user_tokens = Some(UserTokens {
                    client_id: client_id.clone(),
                    client_secret,
                    app_name: if app_name.is_empty() {
                        "moviedb".into()
                    } else {
                        app_name
                    },
                    app_version: if app_version.is_empty() {
                        "1.0".into()
                    } else {
                        app_version
                    },

                    access_token: Default::default(),
                });

                let (tx_user_code, rx_user_code) = channel();
                let (tx_access_token, rx_access_token) = channel();
                thread::spawn(move || {
                    _ = tx_access_token.send(simkl::tokens::get_tokens(&client_id, tx_user_code));
                });

                self.rx_user_code = Some(rx_user_code);
                self.rx_access_token = Some(rx_access_token);

                Phase::GettingUserCode
            }
            Phase::Authorize(_) => Phase::Finalizing,
            Phase::Finalizing => Phase::Done,
            _ => Phase::Initializing,
        };
    }
}

impl PopupTrait for SimklInitPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.throbber_visible || matches!(self.phase, Phase::Authorize(_))
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
                                self.client_id_input =
                                    TextArea::new(vec![user_tokens.client_id.clone()]);
                                self.client_secret_input =
                                    TextArea::new(vec![user_tokens.client_secret.clone()]);
                                self.app_name_input =
                                    TextArea::new(vec![user_tokens.app_name.clone()]);
                                self.app_version_input =
                                    TextArea::new(vec![user_tokens.app_version.clone()]);
                                self.advance_phase();
                            } else {
                                self.phase = Phase::Done;
                                self.status = Some(true);
                                self.user_tokens = Some(user_tokens);
                            }
                        } else {
                            self.advance_phase();
                        }
                    }
                },
            Phase::GettingUserCode => {
                if let Some(rx_user_code) = self.rx_user_code.as_ref() {
                    if let Ok(user_code) = rx_user_code.try_recv() {
                        self.status = Some(false);
                        self.phase = Phase::Authorize(user_code);
                    }
                }
                if let Some(rx_access_token) = self.rx_access_token.as_ref() {
                    if let Ok(Err(error)) = rx_access_token.try_recv() {
                        self.item = 0;
                        self.phase = Phase::Error(format!("{:#}", error));
                    }
                }
            }
            Phase::Authorize(_) => 'label: {
                if let Some(rx_user_code) = self.rx_user_code.as_ref() {
                    if let Err(std::sync::mpsc::TryRecvError::Disconnected) =
                        rx_user_code.try_recv()
                    {
                        self.advance_phase();
                        break 'label;
                    }
                }
                if let Some(rx_access_token) = self.rx_access_token.as_ref() {
                    if let Ok(Err(error)) = rx_access_token.try_recv() {
                        self.item = 0;
                        self.phase = Phase::Error(format!("{:#}", error));
                    }
                }
            }
            Phase::Finalizing =>
                if let Some(rx_access_token) = self.rx_access_token.as_ref() {
                    if let Ok(result) = rx_access_token.try_recv() {
                        match result {
                            Ok(access_token) => {
                                if let Some(tokens) = self.user_tokens.as_mut() {
                                    tokens.access_token = access_token;
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
                },
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
            Phase::Initializing | Phase::GettingUserCode | Phase::Finalizing | Phase::Done => {
                self.throbber_visible = true;

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(7, 40, frame.area()),
                    " Simkl Authentication ",
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
            Phase::GetClientInfo => {
                let inputs_valid =
                    !(self.client_id_input.is_empty() || self.client_secret_input.is_empty());

                key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                simkl_init_popup.item += 1;
                                if simkl_init_popup.item > 4 {
                                    simkl_init_popup.item = 0;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                simkl_init_popup.item =
                                    simkl_init_popup.item.checked_sub(1).unwrap_or(1);
                            }
                            _ => {}
                        }
                    }
                });
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        simkl_init_popup.item = 4;
                    }
                });
                key_event_handler.bind_enter((None, None), "Next".into(), |app, _| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        simkl_init_popup.item += 1;
                    }
                });
                if inputs_valid {
                    key_event_handler.bind_enter((None, Some(3)), "Confirm".into(), |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.advance_phase();
                        }
                    });
                    key_event_handler.bind_enter((None, Some(4)), "Confirm".into(), |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.advance_phase();
                        }
                    });
                } else {
                    key_event_handler.bind_enter((None, Some(4)), "".into(), |_, _| {});
                }

                key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            simkl_init_popup.client_id_input.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(1)), "".into(), |app, data| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            simkl_init_popup.client_secret_input.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(2)), "".into(), |app, data| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            simkl_init_popup.app_name_input.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(3)), "".into(), |app, data| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let key_event_handler::Data::Key(key_event) = data {
                            simkl_init_popup.app_version_input.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(14, 50, frame.area()),
                    " Simkl Authentication ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let [client_id_area, client_secret_area, app_info_area, _] =
                    vertical![==3, ==3, ==3, ==1]
                        .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                widgets::input_field(
                    true,
                    self.item == 0,
                    !self.client_id_input.is_empty(),
                    &mut self.client_id_input,
                    WrapMode::None,
                    frame,
                    client_id_area,
                    " Client ID ",
                    "Enter the Client ID",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    client_id_area,
                    |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.item = 0;
                        }
                    },
                );

                widgets::input_field(
                    true,
                    self.item == 1,
                    !self.client_secret_input.is_empty(),
                    &mut self.client_secret_input,
                    WrapMode::None,
                    frame,
                    client_secret_area,
                    " Client Secret ",
                    "Enter the Client Secret",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    client_secret_area,
                    |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.item = 1;
                        }
                    },
                );

                let [app_name_area, app_version_area] = horizontal![>=1, >=1].areas(app_info_area);

                widgets::input_field(
                    true,
                    self.item == 2,
                    true,
                    &mut self.app_name_input,
                    WrapMode::None,
                    frame,
                    app_name_area,
                    " App name ",
                    "moviedb",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    app_name_area,
                    |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.item = 2;
                        }
                    },
                );

                widgets::input_field(
                    true,
                    self.item == 3,
                    true,
                    &mut self.app_version_input,
                    WrapMode::None,
                    frame,
                    app_version_area,
                    " App version ",
                    "1.0",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    app_version_area,
                    |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.item = 3;
                        }
                    },
                );

                let confirm_mouse_area = widgets::action(
                    Action::new(
                        " Confirm ",
                        ActionType::Default,
                        self.item == 4,
                        inputs_valid,
                    ),
                    HorizontalAlignment::Right,
                    true,
                    helpers::add_padding(popup_area, Padding::right(1)),
                    frame,
                );
                if inputs_valid {
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        confirm_mouse_area,
                        |app, _| {
                            if let Some(Popup::SimklInit(simkl_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                simkl_init_popup.advance_phase();
                            }
                        },
                    );
                }
            }
            Phase::Authorize(user_code) => {
                key_event_handler.bind_esc((None, None), "".into(), |app, _| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        simkl_init_popup.item = 0;
                        simkl_init_popup.rx_access_token = None;
                        simkl_init_popup.rx_user_code = None;
                        simkl_init_popup.phase = Phase::GetClientInfo;
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(10, 40, frame.area()),
                    " Simkl Authentication ",
                    false,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );

                let back_mouse_area = widgets::action(
                    Action::new(" Back ", ActionType::Default, false, true),
                    HorizontalAlignment::Left,
                    false,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    back_mouse_area,
                    |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.item = 0;
                            simkl_init_popup.rx_access_token = None;
                            simkl_init_popup.rx_user_code = None;
                            simkl_init_popup.phase = Phase::GetClientInfo;
                        }
                    },
                );

                let [_, user_code_area, hyperlink_area, _] = vertical![*=1, ==1, ==3, *=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                frame.render_widget(
                    line![
                        "User Code: ",
                        user_code.as_str().bold().fg(tailwind::VIOLET.c300)
                    ]
                    .centered(),
                    user_code_area,
                );

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
                        url:  "https://simkl.com/pin".to_string(),
                    },
                    hyperlink_area,
                );
            }
            Phase::Error(error) => {
                let back = |app: &mut App, _| {
                    if let Some(Popup::SimklInit(simkl_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        simkl_init_popup.item = 0;
                        simkl_init_popup.client_id_input.clear();
                        simkl_init_popup.rx_access_token = None;
                        simkl_init_popup.rx_user_code = None;
                        simkl_init_popup.phase = Phase::GetClientInfo;
                        if let Some(false) = simkl_init_popup.status {
                            if let Some(user_tokens) = simkl_init_popup.user_tokens.as_ref() {
                                simkl_init_popup.client_id_input =
                                    TextArea::new(vec![user_tokens.access_token.clone()]);
                                simkl_init_popup
                                    .client_id_input
                                    .move_cursor(ratatui_textarea::CursorMove::End);
                            }
                        }
                    }
                };
                key_event_handler.bind_enter((None, None), "Back".into(), back);
                key_event_handler.bind_esc((None, Some(0)), "Back".into(), back);
                if self.status.is_some() {
                    key_event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            simkl_init_popup.phase = Phase::Done;
                        }
                    });
                    key_event_handler.bind_tab((None, None), "".into(), |app, data| {
                        if let Some(Popup::SimklInit(simkl_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                crate::key_event_handler::Data::Direction(true, _) => {
                                    simkl_init_popup.item += 1;
                                    if simkl_init_popup.item > 1 {
                                        simkl_init_popup.item = 0;
                                    }
                                }
                                crate::key_event_handler::Data::Direction(false, _) => {
                                    simkl_init_popup.item =
                                        simkl_init_popup.item.checked_sub(1).unwrap_or(1);
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
                            if let Some(Popup::SimklInit(simkl_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                simkl_init_popup.phase = Phase::Done;
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
