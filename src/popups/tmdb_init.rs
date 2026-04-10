use crate::{
    app::App,
    helpers::{add_padding, center_rect, dynamic_popup},
    key_event_handler::{self, KeyEventHandler},
    popups::Popups,
    tmdb,
    tokens::tmdb_tokens::{TMDBTokens, UserTokens},
};
use itertools::Itertools;
use ratatui::{
    layout::*,
    macros::{constraint, horizontal, span, vertical},
    prelude::*,
    style::palette::{material, tailwind},
    symbols::{block, scrollbar::Set},
    text::ToSpan,
    widgets::*,
    Frame,
};
use std::{
    fmt::format,
    ops::Add,
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    thread,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

#[derive(Default)]
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
use ratatui_textarea::{TextArea, WrapMode};

#[derive(Default)]
pub struct TMDBInitPopup {
    pub tick: u64,
    pub phase: Phase,
    throbber_visible: bool,
    tab: usize,
    item: usize,

    input: TextArea<'static>,
    throbber_state: ThrobberState,

    rx_init: Option<Receiver<anyhow::Result<UserTokens>>>,
    rx_authorization_url: Option<Receiver<String>>,
    rx_session_id: Option<Receiver<anyhow::Result<String>>>,

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
        (Some(self.tab), Some(self.item))
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
                    session_id: String::default(),
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
            Phase::Finalize => Phase::Error(format!("{:#?}", self.tokens)),
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
                            if !tokens.has_access_token() {
                                self.advance_phase();
                            } else if !tokens.has_session_id() {
                                // self.phase = Phase::GetAccessToken;
                                self.advance_phase();
                                self.advance_phase();
                            }
                        } else {
                            self.advance_phase();
                        }
                    }
                }
            }
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
            Phase::Finalize => {
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
                }
            }
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
                    "  TMDB Authorization  ",
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
            Phase::GetAccessToken => {
                self.tab = 0;

                key_event_handler.bind_tab((Some(self.tab), None), "".into(), |app, data| {
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
                key_event_handler.bind_enter((Some(self.tab), Some(0)), "".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 1;
                    }
                });
                key_event_handler.bind_esc((Some(self.tab), Some(0)), "".into(), |app, _| {
                    if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        tmdb_init_popup.item = 1;
                    }
                });
                key_event_handler.bind_enter(
                    (Some(self.tab), Some(1)),
                    "Confirm".into(),
                    |app, _| {
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.advance_phase();
                        }
                    },
                );
                key_event_handler.bind_input_field(
                    (Some(self.tab), Some(0)),
                    "".into(),
                    |app, data| {
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
                    },
                );

                let popup_area = dynamic_popup(
                    frame,
                    Some(9),
                    4.0,
                    tailwind::BLUE.c950,
                    "  TMDB Authorization  ",
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
                    vertical![==1, ==5, >=1, ==1, ==1].areas(popup_area);
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
                            if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                tmdb_init_popup.advance_phase();
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
                        .title(" Access Token ")
                        .title_style(Style::new().fg(if input_selected {
                            material::BLUE.c400
                        } else {
                            material::BLUE.c600
                        }))
                        .padding(Padding::symmetric(1, 0)),
                );
                self.input.set_placeholder_text("Enter the Access Token");
                self.input
                    .set_placeholder_style(Style::new().fg(material::GRAY.c700));
                self.input.set_wrap_mode(WrapMode::Glyph);
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
                        if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            tmdb_init_popup.item = 1;
                        }
                    },
                );
            }
            Phase::Authorize(authorization_url) => {
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
                let [_, message_area, _, actions_area, _] =
                    vertical![>=1, >=1, ==1, ==1, ==1].areas(popup_area);

                let text = authorization_url.to_span().into_centered_line();
                let hyperlink = Hyperlink::new(text, authorization_url);

                frame.render_widget(&hyperlink, message_area);
            }
            Phase::Error(error) => {
                self.tab = 2;

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
                let [_, message_area, _, actions_area, _] =
                    vertical![>=1, >=1, ==1, ==1, ==1].areas(popup_area);
                frame.render_widget(
                    Paragraph::new(error.as_str())
                        .wrap(Wrap { trim: true })
                        .centered(),
                    message_area,
                );

                let actions = vec![
                    Span::from(" Ok ").style(
                        Style::new()
                            .fg(tailwind::SLATE.c200)
                            .bg(material::BLUE.c600),
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
                            if let Some(Popups::TMDBInit(tmdb_init_popup)) =
                                app.drawer.active_popup.as_mut()
                            {}
                        },
                    );
                }
                frame.render_widget(Line::from(actions).right_aligned(), actions_area);
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
        for (i, two_chars) in self
            .text
            .to_string()
            .chars()
            .chunks(2)
            .into_iter()
            .enumerate()
        {
            let text = two_chars.collect::<String>();
            let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
            buffer[(area.x + i as u16 * 2, area.y)].set_symbol(hyperlink.as_str());
        }
    }
}
