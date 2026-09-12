use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use log::info;
use ratatui::{
    layout::{HorizontalAlignment, Margin},
    macros::vertical,
    text::Text,
    widgets::Padding,
};
use rustc_hash::FxHashSet;
use strum::AsRefStr;

use crate::{
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::KeyEventHandler,
    processors::{Processor, ProcessorDiscriminants, ProcessorTrait},
    tokens::{
        punch_play_tokens::{PunchPlayTokens, UserTokens as PunchPlayUserTokens},
        trakt_tokens::{TraktTokens, UserTokens as TraktUserTokens},
    },
    widgets::{self, Action, ActionType},
};

#[derive(AsRefStr, Hash, PartialEq, Eq, Clone, Copy, Debug)]
enum Tokens {
    PunchPlay,
    Trakt,
}

enum RefreshResponse {
    PunchPlay(anyhow::Result<PunchPlayUserTokens>),
    Trakt(anyhow::Result<TraktUserTokens>),
}

#[derive(Default)]
pub struct TokensRefresherProcessor {
    item:              usize,
    initialized:       bool,
    active_refreshing: usize,

    skipped:             FxHashSet<Tokens>,
    errored:             Vec<(Tokens, String)>,
    tx_refresh_response: Option<Sender<RefreshResponse>>,
    rx_refresh_response: Option<Receiver<RefreshResponse>>,

    punch_play_tokens: PunchPlayTokens,
    trakt_tokens:      TraktTokens,
}

impl TokensRefresherProcessor {
    pub fn initialize(&mut self, punch_play_tokens: PunchPlayTokens, trakt_tokens: TraktTokens) {
        if self.initialized {
            return;
        }

        let (tx_refresh_response, rx_refresh_response) = channel();

        *self = Self {
            skipped: {
                let mut skipped: FxHashSet<Tokens> = Default::default();

                if !punch_play_tokens.status.unwrap_or_default() {
                    skipped.insert(Tokens::PunchPlay);
                }
                if !trakt_tokens.status.unwrap_or_default() {
                    skipped.insert(Tokens::Trakt);
                }

                skipped
            },
            tx_refresh_response: Some(tx_refresh_response),
            rx_refresh_response: Some(rx_refresh_response),

            punch_play_tokens,
            trakt_tokens,
            initialized: true,

            ..Default::default()
        }
    }

    pub fn update_punch_play_tokens(&mut self, tokens: PunchPlayTokens) {
        self.punch_play_tokens = tokens;

        if self.punch_play_tokens.status.unwrap_or_default() {
            self.skipped.remove(&Tokens::PunchPlay);
        } else {
            self.skipped.insert(Tokens::PunchPlay);
        }
    }

    pub fn update_trakt_tokens(&mut self, tokens: TraktTokens) {
        self.trakt_tokens = tokens;

        if self.trakt_tokens.status.unwrap_or_default() {
            self.skipped.remove(&Tokens::Trakt);
        } else {
            self.skipped.insert(Tokens::Trakt);
        }
    }

    fn refresh_tokens(&mut self, tokens: Tokens) {
        let tx_refresh_response = self.tx_refresh_response.as_ref().unwrap().clone();
        match tokens {
            Tokens::PunchPlay => {
                info!("refreshing punch play");
                let client_id = self.punch_play_tokens.client_id_owned();
                let client_secret = self.punch_play_tokens.client_secret_owned();
                let refresh_token = self.punch_play_tokens.refresh_token_owned();
                thread::spawn(move || {
                    _ = tx_refresh_response.send(RefreshResponse::PunchPlay(
                        punch_play::tokens::refresh_tokens(
                            &client_id,
                            &client_secret,
                            &refresh_token,
                        )
                        .map(|x| PunchPlayUserTokens {
                            client_id,
                            client_secret,
                            access_token: x.access_token,
                            refresh_token: x.refresh_token,
                            expires_on: unix_ts::Timestamp::now().seconds() + x.expires_in,
                        }),
                    ));
                });
                self.skipped.insert(Tokens::PunchPlay);
            }
            Tokens::Trakt => {
                info!("refreshing trakt");
                let client_id = self.trakt_tokens.client_id_owned();
                let client_secret = self.trakt_tokens.client_secret_owned();
                let refresh_token = self.trakt_tokens.refresh_token_owned();
                thread::spawn(move || {
                    _ = tx_refresh_response.send(RefreshResponse::Trakt(
                        trakt::tokens::refresh_tokens(&client_id, &client_secret, &refresh_token)
                            .map(|x| TraktUserTokens {
                                client_id,
                                client_secret,
                                access_token: x.access_token,
                                refresh_token: x.refresh_token,
                                expires_on: x.created_at + x.expires_in,
                            }),
                    ));
                });
                self.skipped.insert(Tokens::Trakt);
            }
        }

        self.active_refreshing += 1;
    }
}

impl ProcessorTrait for TokensRefresherProcessor {
    fn update(&mut self, key_event_handler: &mut KeyEventHandler) {
        if !self.initialized {
            return;
        }

        if !self.skipped.contains(&Tokens::PunchPlay) {
            if self.punch_play_tokens.should_refresh_tokens() {
                self.refresh_tokens(Tokens::PunchPlay);
            }
        }
        if !self.skipped.contains(&Tokens::Trakt) {
            if self.trakt_tokens.should_refresh_tokens() {
                self.refresh_tokens(Tokens::Trakt);
            }
        }

        if self.errored.is_empty() {
            self.item = 0;
        }

        if self.active_refreshing == 0 {
            return;
        }

        for response in self.rx_refresh_response.as_ref().unwrap().try_iter() {
            match response {
                RefreshResponse::PunchPlay(result) => match result {
                    Ok(user_tokens) => {
                        info!("successfully refreshed punchplay");
                        self.skipped.insert(Tokens::PunchPlay);
                        self.active_refreshing -= 1;

                        key_event_handler.bind_immediate(move |app, _| {
                            app.set_punch_play_user_tokens(user_tokens.clone())
                        });
                    }
                    Err(error) => self
                        .errored
                        .push((Tokens::PunchPlay, format!("{:?}", error))),
                },
                RefreshResponse::Trakt(result) => match result {
                    Ok(user_tokens) => {
                        info!("successfully refreshed trakt");
                        self.skipped.insert(Tokens::Trakt);
                        self.active_refreshing -= 1;

                        key_event_handler.bind_immediate(move |app, _| {
                            app.set_trakt_user_tokens(user_tokens.clone())
                        });
                    }
                    Err(error) => self.errored.push((Tokens::Trakt, format!("{:?}", error))),
                },
            }
        }
    }

    fn needs_render(&self) -> bool {
        !self.errored.is_empty()
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        key_event_handler: &mut KeyEventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        if let Some((tokens, error)) = self.errored.first() {
            key_event_handler.clear();
            key_event_handler.bind_tab((None, None), "Navigate".into(), |app, _| {
                if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                {
                    if tokens_refresher_processor.item < 2 {
                        tokens_refresher_processor.item = 2;
                    } else {
                        tokens_refresher_processor.item = 0;
                    }
                }
            });
            key_event_handler.bind_horizontal((None, None), "Navigate".into(), |app, data| {
                if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                {
                    if let crate::key_event_handler::Data::Direction(dir, _) = data {
                        tokens_refresher_processor.item = dir as usize;
                    }
                }
            });

            key_event_handler.bind_enter((None, Some(0)), "Retry".into(), |app, _| {
                if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                {
                    tokens_refresher_processor.item = 0;
                    let (tokens, _) = tokens_refresher_processor.errored.remove(0);
                    tokens_refresher_processor.active_refreshing -= 1;
                    tokens_refresher_processor.refresh_tokens(tokens);
                }
            });
            key_event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                {
                    tokens_refresher_processor.item = 0;
                    let (tokens, _) = tokens_refresher_processor.errored.remove(0);
                    tokens_refresher_processor.active_refreshing -= 1;
                    tokens_refresher_processor.skipped.insert(tokens);
                }
            });
            let tokens_cloned = *tokens;
            key_event_handler.bind_enter((None, Some(2)), "Retry".into(), move |app, _| {
                if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                {
                    tokens_refresher_processor.item = 0;
                    let (tokens, _) = tokens_refresher_processor.errored.remove(0);
                    tokens_refresher_processor.active_refreshing -= 1;
                    tokens_refresher_processor.skipped.insert(tokens);
                }
                app.drawer.stash_current_popup();
                match tokens_cloned {
                    Tokens::PunchPlay => app.drawer.open_punch_play_init_popup(),
                    Tokens::Trakt => app.drawer.open_trakt_init_popup(),
                }
            });

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
                    &format!("{} error while refreshing: {}", tokens.as_ref(), error),
                    message_area.width as usize,
                ))
                .centered(),
                message_area,
            );

            let mouse_area = widgets::action(
                Action::new(" Retry ", ActionType::Normal, self.item == 2, true),
                HorizontalAlignment::Right,
                false,
                popup_area,
                frame,
            );
            let tokens_cloned = *tokens;
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                        app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                    {
                        tokens_refresher_processor.item = 0;
                        let (tokens, _) = tokens_refresher_processor.errored.remove(0);
                        tokens_refresher_processor.active_refreshing -= 1;
                        tokens_refresher_processor.skipped.insert(tokens);
                    }
                    app.drawer.stash_current_popup();
                    match tokens_cloned {
                        Tokens::PunchPlay => app.drawer.open_punch_play_init_popup(),
                        Tokens::Trakt => app.drawer.open_trakt_init_popup(),
                    }
                },
            );

            let actions_mouse_areas = widgets::actions(
                [
                    Action::new(" Retry ", ActionType::Default, self.item == 0, true),
                    Action::new(" Skip ", ActionType::Critical, self.item == 1, true),
                ],
                HorizontalAlignment::Center,
                true,
                1,
                helpers::add_padding(popup_area, Padding::right(1)),
                frame.buffer_mut(),
            );
            for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Processor::TokensRefresher(tokens_refresher_processor)) =
                            app.get_processor_mut(ProcessorDiscriminants::TokensRefresher)
                        {
                            tokens_refresher_processor.item = 0;
                            let (tokens, _) = tokens_refresher_processor.errored.remove(0);

                            tokens_refresher_processor.active_refreshing -= 1;
                            if i == 0 {
                                tokens_refresher_processor.refresh_tokens(tokens);
                            } else {
                                tokens_refresher_processor.skipped.insert(tokens);
                            }
                        }
                    },
                );
            }
        }
    }
}
