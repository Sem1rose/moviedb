use crate::{
    app::{App, Result},
    draw::Drawer,
    tmdb, trakt,
};
use ratatui::{prelude::*, widgets::*, Frame};
use std::{
    sync::mpsc::{channel, Receiver},
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
    tmdb_rx_session_id: Option<Receiver<Result<String>>>,
    rx_authorization_url: Option<Receiver<String>>,
}

impl Drawer {
    pub fn render_init_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);

        match self.init_screen_options.phase {
            Phase::TMDBInit => {
                if !self.init_screen_options.started_step {
                    self.open_tmdb_init_popup();
                    app.tmdb_config.init(&app.config);

                    self.init_screen_options.started_step = true;
                }

                self.handle_init_screen_tmdb_init(app)?;
            }
            Phase::TraktInit => {
                if !self.init_screen_options.started_step {
                    self.open_trakt_init_popup();
                    app.trakt_config.init(&app.config);

                    self.init_screen_options.started_step = true;
                }

                self.handle_init_screen_trakt_init(app)?;
            }
            Phase::FetchArtwork => {
                if !self.init_screen_options.started_step {
                    self.open_fetch_artworks_popup(app)?;

                    self.init_screen_options.started_step = true;
                }

                self.handle_init_screen_fetch_artworks();
            }
            Phase::Done => {
                self.open_main_screen();
            }
        }

        Ok(())
    }

    fn init_screen_advance_phase(&mut self) {
        self.init_screen_options.started_step = false;

        self.init_screen_options.phase = match self.init_screen_options.phase {
            Phase::TMDBInit => Phase::TraktInit,
            Phase::TraktInit => Phase::FetchArtwork,
            Phase::FetchArtwork => Phase::Done,
            _ => Phase::TMDBInit,
        };
    }

    fn handle_init_screen_fetch_artworks(&mut self) {
        if self.fetch_artwork_popup_options.done {
            self.init_screen_advance_phase();
        }
    }

    fn handle_init_screen_tmdb_init(&mut self, app: &mut App) -> Result<()> {
        use crate::popups::tmdb_init::Phase as TMDBPhase;

        match self.tmdb_init_popup_options.phase {
            TMDBPhase::Initializing => {
                if let Ok(result) = app.rx_tmdb.try_recv() {
                    if let Ok(decrypted_creds) = result {
                        app.tmdb_config.set_creds(decrypted_creds)?;

                        self.init_screen_advance_phase();
                    } else if result.is_err() {
                        self.tmdb_init_popup_options.advance_phase();
                    }
                }
            }
            TMDBPhase::GotInput => {
                if self.init_screen_options.tmdb_rx_session_id.is_none() {
                    let (tx_authorization_url, rx_authorization_url) = channel();
                    let (tx_session_id, rx_session_id) = channel();

                    let access_token = self
                        .tmdb_init_popup_options
                        .access_token_input
                        .value()
                        .to_string();

                    app.tmdb_config.set_access_token(access_token.clone());

                    thread::spawn(move || {
                        tx_session_id
                            .send(tmdb::get_session_id(&access_token, tx_authorization_url))
                    });

                    self.init_screen_options.rx_authorization_url = Some(rx_authorization_url);
                    self.init_screen_options.tmdb_rx_session_id = Some(rx_session_id);
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
        match self.init_screen_options.phase {
            Phase::TMDBInit => {
                if let Some(channel) = self.init_screen_options.rx_authorization_url.as_ref() {
                    let result = channel.try_recv();

                    if let Ok(data) = result {
                        self.tmdb_init_popup_options.get_authorization(data);

                        let _ = self.init_screen_options.rx_authorization_url.take();
                    }

                    // else if let Err(error) = result {
                    //     self.tmdb_init_popup_options.phase =
                    //         crate::popups::tmdb_init::Phase::GetInput;
                    //     self.tmdb_init_popup_options.access_token_input.reset();

                    //     let _ = self.init_screen_options.tmdb_rx_session_id.take();
                    // }
                }

                if let Some(channel) = self.init_screen_options.tmdb_rx_session_id.as_ref() {
                    if let Ok(result) = channel.try_recv() {
                        if let Ok(session_id) = result {
                            self.tmdb_init_popup_options.advance_phase();

                            app.tmdb_config.set_session_id(session_id);
                            app.tmdb_config.save_creds(&app.config)?;

                            self.init_screen_advance_phase();

                            let _ = self.init_screen_options.tmdb_rx_session_id.take();
                        } else if let Err(error) = result {
                            self.tmdb_init_popup_options.phase =
                                crate::popups::tmdb_init::Phase::GetInput;
                            self.tmdb_init_popup_options.access_token_input.reset();

                            let _ = self.init_screen_options.tmdb_rx_session_id.take();
                            // panic!("Error while getting TMDB session_id: {error}");
                        }
                    }
                }
            }
            Phase::TraktInit => {}
            _ => (),
        }

        Ok(())
    }

    fn handle_init_screen_trakt_init(&mut self, app: &mut App) -> Result<()> {
        if let Ok(result) = app.rx_trakt.try_recv() {
            if let Ok(decrypted_creds) = result {
                app.trakt_config.set_creds(decrypted_creds)?;

                self.init_screen_advance_phase();
            } else if result.is_err() {
                todo!();
            }
        }

        Ok(())
    }
}
