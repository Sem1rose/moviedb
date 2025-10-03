use crate::{
    app::App,
    draw::Drawer,
    tmdb,
    trakt::{self, TraktTokens},
    types::*,
};
use ratatui::{prelude::*, widgets::*, Frame};
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};
use style::palette::tailwind;

#[derive(Default)]
pub enum Phase {
    #[default]
    TMDBInit,
    TraktInit,
    FetchArtwork,
    Done,
}

#[derive(Default)]
pub struct InitScreen {
    pub phase: Phase,

    started_step: bool,

    tmdb_rx_authorization_url: Option<Receiver<String>>,
    tmdb_rx_session_id: Option<Receiver<Result<String>>>,

    trakt_rx_authorization_url: Option<Receiver<String>>,
    trakt_tx_auth_code: Option<Sender<String>>,
    trakt_rx_tokens: Option<Receiver<Result<TraktTokens>>>,
}

impl Drawer {
    pub fn render_init_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area: Rect = frame.area();
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);

        match self.init_screen.phase {
            Phase::TMDBInit => {
                if !self.init_screen.started_step {
                    self.open_tmdb_init_popup();
                    app.tmdb_config.init(&app.config);

                    self.init_screen.started_step = true;
                }

                self.handle_init_screen_tmdb_init(app)?;
            }
            Phase::TraktInit => {
                if !self.init_screen.started_step {
                    self.open_trakt_init_popup();
                    app.trakt_config.init(&app.config);

                    self.init_screen.started_step = true;
                }

                self.handle_init_screen_trakt_init(app)?;
            }
            Phase::FetchArtwork => {
                if !self.init_screen.started_step {
                    self.open_fetch_artworks_popup(app)?;

                    self.init_screen.started_step = true;
                }

                self.handle_init_screen_fetch_artworks(app);
            }
            Phase::Done => {
                self.open_main_screen();
            }
        }

        Ok(())
    }

    fn init_screen_advance_phase(&mut self) {
        self.init_screen.started_step = false;

        self.init_screen.phase = match self.init_screen.phase {
            Phase::TMDBInit => Phase::TraktInit,
            Phase::TraktInit => Phase::FetchArtwork,
            Phase::FetchArtwork => Phase::Done,
            _ => Phase::TMDBInit,
        };
    }

    fn handle_init_screen_fetch_artworks(&mut self, app: &App) {
        if self.fetch_artwork_popup.done {
            self.init_screen_advance_phase();
        }
    }

    fn handle_init_screen_tmdb_init(&mut self, app: &mut App) -> Result<()> {
        use crate::popups::tmdb_init::Phase as TMDBPhase;

        match self.tmdb_init_popup.phase {
            TMDBPhase::Initializing => {
                if let Ok(result) = app.rx_tmdb.try_recv() {
                    if let Ok(decrypted_creds) = result {
                        app.tmdb_config.set_creds(decrypted_creds)?;

                        self.init_screen_advance_phase();
                    } else if result.is_err() {
                        self.tmdb_init_popup.advance_phase();
                    }
                }
            }
            TMDBPhase::GotInput => {
                if self.init_screen.tmdb_rx_session_id.is_none() {
                    let (tx_authorization_url, rx_authorization_url) = channel();
                    let (tx_session_id, rx_session_id) = channel();

                    let access_token = self.tmdb_init_popup.access_token_input.value().to_string();

                    app.tmdb_config.set_access_token(access_token.clone());

                    thread::spawn(move || {
                        tx_session_id
                            .send(tmdb::get_session_id(&access_token, tx_authorization_url))
                    });

                    self.init_screen.tmdb_rx_authorization_url = Some(rx_authorization_url);
                    self.init_screen.tmdb_rx_session_id = Some(rx_session_id);
                }

                self.read_init_screen_channels(app)?;
            }
            TMDBPhase::GetAuthorization(_) => {
                self.read_init_screen_channels(app)?;
            }
            _ => (),
        }

        Ok(())
    }

    fn read_init_screen_channels(&mut self, app: &mut App) -> Result<()> {
        match self.init_screen.phase {
            Phase::TMDBInit => {
                if let Some(channel) = self.init_screen.tmdb_rx_authorization_url.as_ref() {
                    let result = channel.try_recv();

                    if let Ok(data) = result {
                        self.tmdb_init_popup.get_authorization(data);

                        let _ = self.init_screen.tmdb_rx_authorization_url.take();
                    }

                    // else if let Err(error) = result {
                    //     self.tmdb_init_popup_options.phase =
                    //         crate::popups::tmdb_init::Phase::GetInput;
                    //     self.tmdb_init_popup_options.access_token_input.reset();

                    //     let _ = self.init_screen_options.tmdb_rx_session_id.take();
                    // }
                }

                if let Some(channel) = self.init_screen.tmdb_rx_session_id.as_ref() {
                    if let Ok(result) = channel.try_recv() {
                        if let Ok(session_id) = result {
                            self.tmdb_init_popup.advance_phase();

                            app.tmdb_config.set_session_id(session_id);
                            app.tmdb_config.save_creds(&app.config)?;

                            self.init_screen_advance_phase();

                            let _ = self.init_screen.tmdb_rx_session_id.take();
                        } else if let Err(error) = result {
                            self.tmdb_init_popup.phase = crate::popups::tmdb_init::Phase::GetInput;
                            self.tmdb_init_popup.access_token_input.reset();

                            let _ = self.init_screen.tmdb_rx_session_id.take();
                            // panic!("Error while getting TMDB session_id: {error}");
                        }
                    }
                }
            }
            Phase::TraktInit => {
                if let Some(channel) = self.init_screen.trakt_rx_authorization_url.as_ref() {
                    let result = channel.try_recv();

                    if let Ok(data) = result {
                        self.trakt_init_popup.get_authorization(data);

                        let _ = self.init_screen.trakt_rx_authorization_url.take();
                    }
                }

                if let Some(channel) = self.init_screen.trakt_rx_tokens.as_ref() {
                    if let Ok(result) = channel.try_recv() {
                        if let Ok(trakt_tokens) = result {
                            self.trakt_init_popup.advance_phase();

                            app.trakt_config.set_tokens(trakt_tokens);
                            app.trakt_config.save_creds(&app.config)?;

                            self.init_screen_advance_phase();

                            let _ = self.init_screen.trakt_rx_tokens.take();
                        } else if let Err(error) = result {
                            self.trakt_init_popup.phase =
                                crate::popups::trakt_init::Phase::Initializing;
                            self.trakt_init_popup.advance_phase();

                            let _ = self.init_screen.trakt_rx_tokens.take();
                            // panic!("Error while getting TMDB session_id: {error}");
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn handle_init_screen_trakt_init(&mut self, app: &mut App) -> Result<()> {
        use crate::popups::trakt_init::Phase as TraktPhase;

        match self.trakt_init_popup.phase {
            TraktPhase::Initializing => {
                if let Ok(result) = app.rx_trakt.try_recv() {
                    if let Ok(decrypted_creds) = result {
                        app.trakt_config.set_creds(decrypted_creds)?;

                        if trakt::check_tokens(&app.trakt_config) {
                            self.init_screen_advance_phase();
                        } else {
                            self.trakt_init_popup.phase = TraktPhase::RefreshingTokens;

                            let (tx_tokens, rx_tokens) = channel();

                            let client_id = app.trakt_config.client_id_owned();
                            let client_secret = app.trakt_config.client_secret_owned();
                            let refresh_token = app.trakt_config.refresh_token_owned();
                            thread::spawn(move || {
                                tx_tokens.send(trakt::refresh_tokens(
                                    &client_id,
                                    &client_secret,
                                    &refresh_token,
                                ))
                            });

                            self.init_screen.trakt_rx_tokens = Some(rx_tokens);
                        }
                    } else if result.is_err() {
                        self.trakt_init_popup.advance_phase();
                    }
                }
            }
            TraktPhase::GotSecrets => {
                if self.init_screen.trakt_rx_authorization_url.is_none() {
                    let (tx_authorization_url, rx_authorization_url) = channel();
                    let (tx_auth_code, rx_auth_code) = channel();
                    let (tx_tokens, rx_tokens) = channel();

                    let client_id = self.trakt_init_popup.cliend_id_input.value().to_string();
                    let client_secret = self
                        .trakt_init_popup
                        .client_secret_input
                        .value()
                        .to_string();

                    app.trakt_config.set_secrets(&client_id, &client_secret);

                    thread::spawn(move || {
                        tx_tokens.send(trakt::get_tokens(
                            &client_id,
                            &client_secret,
                            tx_authorization_url,
                            rx_auth_code,
                        ))
                    });

                    self.init_screen.trakt_rx_authorization_url = Some(rx_authorization_url);
                    self.init_screen.trakt_tx_auth_code = Some(tx_auth_code);
                    self.init_screen.trakt_rx_tokens = Some(rx_tokens);
                }

                self.read_init_screen_channels(app)?;
            }
            TraktPhase::GotAuthorization => {
                if self.init_screen.trakt_tx_auth_code.is_some() {
                    let auth_code = self.trakt_init_popup.auth_code_input.value().to_string();

                    let _ = self
                        .init_screen
                        .trakt_tx_auth_code
                        .take()
                        .unwrap()
                        .send(auth_code);
                }

                self.read_init_screen_channels(app)?;
            }
            TraktPhase::RefreshingTokens => {
                self.read_init_screen_channels(app)?;
            }
            _ => (),
        }

        Ok(())
    }
}
