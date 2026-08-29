use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use anyhow::anyhow;
use log::info;
use ratatui::{
    layout::{HorizontalAlignment, Margin},
    macros::vertical,
    text::Text,
    widgets::Padding,
};
use reqwest::blocking::Response;
use strum::IntoEnumIterator;
use toml::Value;

use crate::{
    helpers,
    key_event_handler::KeyEventHandler,
    processors::{Processor, ProcessorDiscriminants, ProcessorTrait},
    tokens::{PunchPlayTokens, SimklTokens, tmdb_tokens::TMDBTokens},
    types::{SyncItem, SyncSource},
    widgets::{self, Action, ActionType},
};

#[derive(Default)]
pub struct HistorySyncerProcessor {
    item:        usize,
    initialized: bool,
    progress:    u32,
    count:       u32,
    pub idle:    bool,

    errors:           Vec<(SyncSource, SyncItem, String)>,
    tx_sync_request:  Option<Sender<(SyncSource, SyncItem)>>,
    rx_sync_response: Option<Receiver<(SyncSource, SyncItem, anyhow::Result<Response>)>>,
}

impl HistorySyncerProcessor {
    fn start_threads(
        mut self,
        tmdb_tokens: TMDBTokens,
        simkl_tokens: SimklTokens,
        punch_play_tokens: PunchPlayTokens,
    ) -> Self {
        let (tx_sync_request, rx_sync_request) = channel::<(SyncSource, SyncItem)>();
        let (tx_sync_response, rx_sync_response) =
            channel::<(SyncSource, SyncItem, anyhow::Result<Response>)>();

        thread::spawn(move || {
            let mut punch_play_watchlist_id = None;

            for (source, item) in rx_sync_request.iter() {
                match source {
                    SyncSource::TMDB =>
                        if tmdb_tokens.status.unwrap_or_default() {
                            let tx_sync_response = tx_sync_response.clone();
                            let tmdb_tokens = tmdb_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id,
                                    date: _,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.trunc() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::AddToList {
                                    list,
                                    date: _,
                                    movie_id,
                                } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                tmdb::movie::add_or_remove_watchlist(
                                                    tmdb_tokens.access_token(),
                                                    tmdb_tokens.account_id(),
                                                    movie_id,
                                                    true,
                                                ),
                                            ))
                                        });
                                    }
                                    crate::types::ListID::TMDB(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                tmdb::list::add_item_to_list(
                                                    tmdb_tokens.access_token(),
                                                    list_id,
                                                    movie_id,
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                                SyncItem::AddPlay {
                                    movie_id,
                                    date: _,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.floor() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::Edit {
                                    movie_id,
                                    date: _,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.floor() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            tmdb::movie::delete_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromList { list, movie_id } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                tmdb::movie::add_or_remove_watchlist(
                                                    tmdb_tokens.access_token(),
                                                    tmdb_tokens.account_id(),
                                                    movie_id,
                                                    false,
                                                ),
                                            ))
                                        });
                                    }
                                    crate::types::ListID::TMDB(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                tmdb::list::remove_item_from_list(
                                                    tmdb_tokens.access_token(),
                                                    list_id,
                                                    movie_id,
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                            }
                        },
                    SyncSource::Simkl =>
                        if simkl_tokens.status.unwrap_or_default() {
                            let tx_sync_response = tx_sync_response.clone();
                            let simkl_tokens = simkl_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            simkl::movie::log_watched(
                                                simkl_tokens.access_token(),
                                                simkl_tokens.client_id(),
                                                simkl_tokens.app_name(),
                                                simkl_tokens.app_version(),
                                                &[(movie_id, rating.trunc() as usize, date)],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::AddToList {
                                    list,
                                    date,
                                    movie_id,
                                } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                simkl::movie::add_movies_to_watchlist(
                                                    simkl_tokens.access_token(),
                                                    simkl_tokens.client_id(),
                                                    simkl_tokens.app_name(),
                                                    simkl_tokens.app_version(),
                                                    &[(movie_id, date)],
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                                SyncItem::AddPlay {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            simkl::movie::edit_watched(
                                                simkl_tokens.access_token(),
                                                simkl_tokens.client_id(),
                                                simkl_tokens.app_name(),
                                                simkl_tokens.app_version(),
                                                &[(movie_id, rating.trunc() as usize, date)],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::Edit {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            simkl::movie::edit_watched(
                                                simkl_tokens.access_token(),
                                                simkl_tokens.client_id(),
                                                simkl_tokens.app_name(),
                                                simkl_tokens.app_version(),
                                                &[(movie_id, rating.trunc() as usize, date)],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            simkl::movie::remove_movies_history_or_from_watchlist(
                                                simkl_tokens.access_token(),
                                                simkl_tokens.client_id(),
                                                simkl_tokens.app_name(),
                                                simkl_tokens.app_version(),
                                                &[movie_id],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromList { list, movie_id } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                simkl::movie::remove_movies_history_or_from_watchlist(
                                                    simkl_tokens.access_token(),
                                                    simkl_tokens.client_id(),
                                                    simkl_tokens.app_name(),
                                                    simkl_tokens.app_version(),
                                                    &[movie_id]
                                                )
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                            }
                        },
                    SyncSource::PunchPlay =>
                        if punch_play_tokens.status.is_some() {
                            let tx_sync_response = tx_sync_response.clone();
                            let punch_play_tokens = punch_play_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((source, item, {
                                            let response = punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(rating.trunc() as usize),
                                                Some(date),
                                            );

                                            if punch_play_watchlist_id.is_none() {
                                                if let Ok(Some(watchlist_id)) =
                                                    punch_play::list::get_user_lists(
                                                        punch_play_tokens.access_token(),
                                                    )
                                                    .map(|y| {
                                                        y.into_iter()
                                                            .find(|x| x.is_watchlist)
                                                            .map(|x| x.id)
                                                    })
                                                {
                                                    punch_play_watchlist_id = Some(watchlist_id);
                                                }
                                            }

                                            if let Some(&watchlist_id) =
                                                punch_play_watchlist_id.as_ref()
                                            {
                                                _ = punch_play::list::remove_item_from_list(
                                                    punch_play_tokens.access_token(),
                                                    watchlist_id,
                                                    movie_id,
                                                );
                                            }
                                            response
                                        }))
                                    });
                                }
                                SyncItem::AddToList {
                                    list,
                                    date: _,
                                    movie_id,
                                } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                'label: {
                                                    if punch_play_watchlist_id.is_none() {
                                                        match
                                                            punch_play::list::get_user_lists(
                                                                punch_play_tokens.access_token(),
                                                            )
                                                            .map(|y| {
                                                                y.into_iter()
                                                                    .find(|x| x.is_watchlist)
                                                                    .map(|x| x.id)
                                                            })
                                                        {
                                                            Ok(watchlist_id) => {
                                                                punch_play_watchlist_id = watchlist_id;
                                                            }
                                                            Err(error) => {
                                                                break 'label Err(error);
                                                            }
                                                        }
                                                    }

                                                    if let Some(&watchlist_id) =
                                                        punch_play_watchlist_id.as_ref()
                                                    {
                                                        punch_play::list::add_item_to_list(
                                                            punch_play_tokens.access_token(),
                                                            watchlist_id,
                                                            "movie",
                                                            movie_id
                                                        )
                                                    } else {
                                                        Err(anyhow!("Punchplay: unable to find the watchlist's list_id"))
                                                    }
                                                },
                                            ))
                                        });
                                    }
                                    crate::types::ListID::PunchPlay(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                punch_play::list::add_item_to_list(
                                                    punch_play_tokens.access_token(),
                                                    list_id,
                                                    "movie",
                                                    movie_id,
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                                SyncItem::AddPlay {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((source, item, {
                                            let response = punch_play::movie::log_watch(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(date),
                                            );
                                            match response {
                                                Ok(response) =>
                                                    if !response.status().is_success() {
                                                        Ok(response)
                                                    } else {
                                                        let response2 =
                                                            punch_play::movie::add_or_edit_rating(
                                                                punch_play_tokens.access_token(),
                                                                movie_id,
                                                                Some(rating.floor() as usize),
                                                                Some(date),
                                                            );

                                                        response2
                                                    },
                                                res @ _ => res,
                                            }
                                        }))
                                    });
                                }
                                SyncItem::Edit {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(rating.floor() as usize),
                                                Some(date),
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            source,
                                            item,
                                            punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                None,
                                                None,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromList { list, movie_id } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                'label: {
                                                    if punch_play_watchlist_id.is_none() {
                                                        match
                                                            punch_play::list::get_user_lists(
                                                                punch_play_tokens.access_token(),
                                                            )
                                                            .map(|y| {
                                                                y.into_iter()
                                                                    .find(|x| x.is_watchlist)
                                                                    .map(|x| x.id)
                                                            })
                                                        {
                                                            Ok(watchlist_id) => {
                                                                punch_play_watchlist_id = watchlist_id;
                                                            }
                                                            Err(error) => {
                                                                break 'label Err(error);
                                                            }
                                                        }
                                                    }

                                                    if let Some(&watchlist_id) =
                                                        punch_play_watchlist_id.as_ref()
                                                    {
                                                        punch_play::list::remove_item_from_list(
                                                            punch_play_tokens.access_token(),
                                                            watchlist_id,
                                                            movie_id,
                                                        )
                                                    } else {
                                                        Err(anyhow!("Punchplay: unable to find the watchlist's list_id"))
                                                    }
                                                },
                                            ))
                                        });
                                    }
                                    crate::types::ListID::PunchPlay(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                source,
                                                item,
                                                punch_play::list::remove_item_from_list(
                                                    punch_play_tokens.access_token(),
                                                    list_id,
                                                    movie_id,
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                            }
                        },
                }
            }
        });

        self.tx_sync_request = Some(tx_sync_request);
        self.rx_sync_response = Some(rx_sync_response);

        self
    }

    pub fn initialize(
        &mut self,
        tmdb_tokens: TMDBTokens,
        simkl_tokens: SimklTokens,
        punch_play_tokens: PunchPlayTokens,
    ) {
        if self.initialized {
            return;
        }

        *self = Self {
            initialized: true,
            idle: true,

            ..Default::default()
        }
        .start_threads(tmdb_tokens, simkl_tokens, punch_play_tokens);
    }

    fn retry_sync(&mut self, sync_source: SyncSource, sync_item: SyncItem) {
        if let Some(tx_sync_request) = self.tx_sync_request.as_ref() {
            _ = tx_sync_request.send((sync_source, sync_item));
        }
    }

    pub fn add_sync_item(&mut self, sync_item: SyncItem) {
        if let Some(tx_sync_request) = self.tx_sync_request.as_ref() {
            for source in SyncSource::iter() {
                if tx_sync_request.send((source, sync_item)).is_ok() {
                    self.count += 1;
                    self.idle = false;
                }
            }
        }
    }
}

impl ProcessorTrait for HistorySyncerProcessor {
    fn update(&mut self, _key_event_handler: &mut KeyEventHandler) {
        if !self.initialized || self.idle {
            return;
        }

        for (source, item, result) in self
            .rx_sync_response
            .as_ref()
            .unwrap()
            .try_iter()
            .inspect(|x| info!("{x:#?}"))
        {
            match result {
                Ok(response) =>
                    if !response.status().is_success() {
                        self.errors.push((
                            source,
                            item,
                            match response.json::<Value>() {
                                Ok(err) => err.to_string(),
                                Err(_) => Default::default(),
                            },
                        ));
                    } else {
                        self.progress += 1;
                    },
                Err(error) => self.errors.push((source, item, format!("{:?}", error))),
            }
        }

        if self.errors.is_empty() {
            self.item = 0;
        }

        if self.progress == self.count {
            self.progress = 0;
            self.count = 0;
            self.idle = true;
        }
    }

    fn needs_render(&self) -> bool {
        !self.errors.is_empty()
    }

    fn render(&self, frame: &mut ratatui::Frame, key_event_handler: &mut KeyEventHandler) {
        if let Some((source, item, error)) = self.errors.first() {
            key_event_handler.clear();

            let popup_area = widgets::window(
                frame,
                helpers::centered_area(11, 44, frame.area()),
                " Error ",
                true,
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                popup_area.outer(Margin::new(1, 1)),
                |_, _| {},
            );
            let [message_area, _] = vertical![>=1, ==1]
                .areas(helpers::add_padding(popup_area, Padding::proportional(1)));
            frame.render_widget(
                Text::from_iter(helpers::wrap_text(
                    &format!(
                        "{} error while syncing {}: {}",
                        source.as_ref(),
                        item.movie_id(),
                        error
                    ),
                    message_area.width as usize,
                ))
                .centered(),
                message_area,
            );

            key_event_handler.bind_tab((None, None), "Navigate".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    history_syncer_processor.item = (history_syncer_processor.item == 0) as usize;
                }
            });
            key_event_handler.bind_horizontal((None, None), "Navigate".into(), |app, data| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    if let crate::key_event_handler::Data::Direction(dir, _) = data {
                        history_syncer_processor.item = dir as usize;
                    }
                }
            });

            key_event_handler.bind_enter((None, Some(0)), "Retry".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    let (source, item, _) = history_syncer_processor.errors.remove(0);
                    history_syncer_processor.retry_sync(source, item);
                }
            });
            key_event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    let _ = history_syncer_processor.errors.remove(0);
                    history_syncer_processor.progress += 1;
                }
            });

            let actions_mouse_areas = widgets::actions(
                [
                    Action::new(" Retry ", ActionType::Default, self.item == 0, true),
                    Action::new(" Skip ", ActionType::Critical, self.item == 1, true),
                ],
                HorizontalAlignment::Center,
                true,
                1,
                helpers::add_padding(popup_area, Padding::right(1)),
                frame,
            );
            for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                            app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                        {
                            let (source, item, _) = history_syncer_processor.errors.remove(0);

                            if i == 0 {
                                history_syncer_processor.retry_sync(source, item);
                            } else {
                                history_syncer_processor.progress += 1;
                            }
                        }
                    },
                );
            }
        }
    }
}
