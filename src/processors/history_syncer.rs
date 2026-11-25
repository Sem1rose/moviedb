use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use anyhow::anyhow;
use ratatui::{
    layout::{HorizontalAlignment, Margin},
    macros::vertical,
    text::Text,
    widgets::Padding,
};
use reqwest::blocking::Response;
use rustc_hash::FxHashSet;
use strum::IntoEnumIterator;
use toml::Value;

use crate::{
    event_handler::EventHandler,
    helpers,
    image_backend::RatatuiImage,
    processors::{Processor, ProcessorDiscriminants, ProcessorTrait},
    sources::{punch_play, simkl, tmdb, trakt},
    tokens::{PunchPlayTokens, SimklTokens, TraktTokens, tmdb_tokens::TMDBTokens},
    types::{SyncItem, SyncSource},
    widgets::{self, Action, ActionType},
};

#[derive(Default)]
pub struct HistorySyncerProcessor {
    item:        usize,
    initialized: bool,
    processing:  FxHashSet<(SyncItem, SyncSource)>,
    pub idle:    bool,

    tmdb_status:       Option<bool>,
    trakt_status:      Option<bool>,
    simkl_status:      Option<bool>,
    punch_play_status: Option<bool>,

    errors:           Vec<(SyncItem, SyncSource, String)>,
    tx_sync_request:  Option<Sender<(SyncItem, SyncSource)>>,
    rx_sync_response: Option<Receiver<(SyncItem, SyncSource, anyhow::Result<Response>)>>,
}

impl HistorySyncerProcessor {
    fn start_thread(
        mut self,
        tmdb_tokens: TMDBTokens,
        trakt_tokens: TraktTokens,
        simkl_tokens: SimklTokens,
        punch_play_tokens: PunchPlayTokens,
    ) -> Self {
        let (tx_sync_request, rx_sync_request) = channel::<(SyncItem, SyncSource)>();
        let (tx_sync_response, rx_sync_response) =
            channel::<(SyncItem, SyncSource, anyhow::Result<Response>)>();

        self.tmdb_status = tmdb_tokens.status;
        self.trakt_status = trakt_tokens.status;
        self.simkl_status = simkl_tokens.status;
        self.punch_play_status = punch_play_tokens.status;

        thread::spawn(move || {
            let mut punch_play_watchlist_id = None;

            for (item, source) in rx_sync_request.iter() {
                match source {
                    SyncSource::TMDB =>
                        if tmdb_tokens.status.unwrap_or_default() {
                            let tx_sync_response = tx_sync_response.clone();
                            let tmdb_tokens = tmdb_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id, rating, ..
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.trunc() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::AddToList { list, movie_id, .. } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
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
                                                item,
                                                source,
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
                                    movie_id, rating, ..
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.floor() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::EditPlay {
                                    movie_id, rating, ..
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            tmdb::movie::add_or_edit_rating(
                                                tmdb_tokens.access_token(),
                                                movie_id,
                                                rating.floor() as usize,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemovePlay { movie_id, date } => unimplemented!(),
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
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
                                                item,
                                                source,
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
                                                item,
                                                source,
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
                    SyncSource::Trakt => {
                        if trakt_tokens.status.unwrap_or_default() {
                            let tx_sync_response = tx_sync_response.clone();
                            let trakt_tokens = trakt_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            trakt::movie::log_watches(
                                                trakt_tokens.client_id(),
                                                trakt_tokens.access_token(),
                                                vec![(
                                                    movie_id,
                                                    Some(rating.trunc() as usize),
                                                    date,
                                                )],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::AddToList { list, movie_id, .. } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((item, source, {
                                                trakt::movie::add_movies_to_watchlist(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    vec![movie_id],
                                                )
                                            }))
                                        });
                                    }
                                    crate::types::ListID::Trakt(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
                                                trakt::list::add_movies_to_list(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    list_id,
                                                    vec![movie_id],
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
                                            item,
                                            source,
                                            trakt::movie::log_watches(
                                                trakt_tokens.client_id(),
                                                trakt_tokens.access_token(),
                                                vec![(
                                                    movie_id,
                                                    Some(rating.floor() as usize),
                                                    date,
                                                )],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::EditPlay {
                                    movie_id,
                                    from,
                                    to,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            // TODO: cleanup
                                            'label: {
                                                if let Err(error) = trakt::movie::modify_watch_date(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    movie_id,
                                                    from,
                                                    to,
                                                ) {
                                                    break 'label Err(error);
                                                }

                                                trakt::movie::modify_rating(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    movie_id,
                                                    rating.floor() as usize,
                                                    to,
                                                )
                                            },
                                        ))
                                    });
                                }
                                SyncItem::RemovePlay { movie_id, date } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            trakt::movie::delete_movie_watch(
                                                trakt_tokens.client_id(),
                                                trakt_tokens.access_token(),
                                                movie_id,
                                                date,
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            trakt::movie::remove_movies_from_watched(
                                                trakt_tokens.client_id(),
                                                trakt_tokens.access_token(),
                                                vec![movie_id],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemoveFromList { list, movie_id } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
                                                trakt::movie::remove_movies_from_watchlist(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    vec![movie_id],
                                                ),
                                            ))
                                        });
                                    }
                                    crate::types::ListID::Trakt(list_id) => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
                                                trakt::list::remove_movies_from_list(
                                                    trakt_tokens.client_id(),
                                                    trakt_tokens.access_token(),
                                                    list_id,
                                                    vec![movie_id],
                                                ),
                                            ))
                                        });
                                    }
                                    _ => (),
                                },
                            }
                        }
                    }
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
                                            item,
                                            source,
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
                                                item,
                                                source,
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
                                            item,
                                            source,
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
                                SyncItem::EditPlay {
                                    movie_id,
                                    from: _from,
                                    to,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            simkl::movie::edit_watched(
                                                simkl_tokens.access_token(),
                                                simkl_tokens.client_id(),
                                                simkl_tokens.app_name(),
                                                simkl_tokens.app_version(),
                                                &[(movie_id, rating.trunc() as usize, to)],
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemovePlay { movie_id, date } => unimplemented!(),
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
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
                                                item,
                                                source,
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
                    SyncSource::PunchPlay => {
                        // if punch_play_tokens.status.is_some() {
                        if punch_play_tokens.status.unwrap_or_default() {
                            let tx_sync_response = tx_sync_response.clone();
                            let punch_play_tokens = punch_play_tokens.clone();

                            match item {
                                SyncItem::AddToWatched {
                                    movie_id,
                                    date,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((item, source, 'label: {
                                            let response = punch_play::movie::log_watch(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(date),
                                            );
                                            if response.is_err()
                                                || (!response
                                                    .as_ref()
                                                    .unwrap()
                                                    .status()
                                                    .is_success()
                                                    && response.as_ref().unwrap().status() != 409)
                                            {
                                                break 'label response;
                                            }

                                            let response = punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(rating.trunc() as usize),
                                                Some(date),
                                            );
                                            if response.is_err()
                                                || !response.as_ref().unwrap().status().is_success()
                                            {
                                                break 'label response;
                                            }

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
                                                _ = punch_play::list::remove_id_from_list(
                                                    punch_play_tokens.access_token(),
                                                    watchlist_id,
                                                    movie_id,
                                                );
                                            }

                                            response
                                        }))
                                    });
                                }
                                SyncItem::AddToList { list, movie_id, .. } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
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
                                                item,
                                                source,
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
                                        tx_sync_response.send((item, source, {
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
                                                        let response =
                                                            punch_play::movie::add_or_edit_rating(
                                                                punch_play_tokens.access_token(),
                                                                movie_id,
                                                                Some(rating.floor() as usize),
                                                                Some(date),
                                                            );

                                                        response
                                                    },
                                                err @ _ => err,
                                            }
                                        }))
                                    });
                                }
                                SyncItem::EditPlay {
                                    movie_id,
                                    from: _,
                                    to,
                                    rating,
                                } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((
                                            item,
                                            source,
                                            punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                Some(rating.floor() as usize),
                                                Some(to),
                                            ),
                                        ))
                                    });
                                }
                                SyncItem::RemovePlay { movie_id, date } => unimplemented!(),
                                SyncItem::RemoveFromWatched { movie_id } => {
                                    thread::spawn(move || {
                                        tx_sync_response.send((item, source, {
                                            let response = punch_play::movie::add_or_edit_rating(
                                                punch_play_tokens.access_token(),
                                                movie_id,
                                                None,
                                                None,
                                            );
                                            match response {
                                                Ok(response) =>
                                                    if !response.status().is_success() {
                                                        Ok(response)
                                                    } else {
                                                        let response =
                                                            punch_play::movie::clear_history(
                                                                punch_play_tokens.access_token(),
                                                                movie_id,
                                                            );

                                                        response
                                                    },
                                                err @ _ => err,
                                            }
                                        }))
                                    });
                                }
                                SyncItem::RemoveFromList { list, movie_id } => match list {
                                    crate::types::ListID::Watchlist => {
                                        thread::spawn(move || {
                                            tx_sync_response.send((
                                                item,
                                                source,
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
                                                item,
                                                source,
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
                        }
                    }
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
        trakt_tokens: TraktTokens,
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
        .start_thread(tmdb_tokens, trakt_tokens, simkl_tokens, punch_play_tokens);
    }

    fn retry_sync(&mut self, sync_item: SyncItem, sync_source: SyncSource) {
        if let Some(tx_sync_request) = self.tx_sync_request.as_ref() {
            _ = tx_sync_request.send((sync_item, sync_source));
        }
    }

    pub fn add_sync_item(&mut self, sync_item: SyncItem) {
        if let Some(tx_sync_request) = self.tx_sync_request.as_ref() {
            for source in SyncSource::iter() {
                if !match source {
                    SyncSource::TMDB => self.tmdb_status,
                    SyncSource::Trakt => self.trakt_status,
                    SyncSource::Simkl => self.simkl_status,
                    SyncSource::PunchPlay => self.punch_play_status,
                }
                .unwrap_or_default()
                {
                    continue;
                }
                if !self.processing.contains(&(sync_item, source)) {
                    if tx_sync_request.send((sync_item, source)).is_ok() {
                        self.processing.insert((sync_item, source));
                        self.idle = false;
                    }
                }
            }
        }
    }
}

impl ProcessorTrait for HistorySyncerProcessor {
    fn needs_render(&self) -> bool {
        !self.errors.is_empty()
    }

    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn active_operation(&self) -> Option<[String; 2]> {
        self.processing.iter().next().map(|(i, s)| {
            [
                s.as_ref().to_string(),
                match i {
                    SyncItem::AddToWatched { movie_id, .. } => {
                        format!("Adding {movie_id} to watched")
                    }
                    SyncItem::AddToList {
                        list:
                            crate::types::ListID::PunchPlay(list_id)
                            | crate::types::ListID::TMDB(list_id),
                        movie_id,
                        ..
                    } => format!("Adding {movie_id} to list {list_id}"),
                    SyncItem::AddToList {
                        list: crate::types::ListID::Watchlist,
                        movie_id,
                        ..
                    } => format!("Adding {movie_id} to watchlist"),
                    SyncItem::AddPlay { movie_id, .. } => {
                        format!("Adding a new play for {movie_id}")
                    }
                    SyncItem::EditPlay { movie_id, .. } => format!("Editing {movie_id}"),
                    SyncItem::RemoveFromWatched { movie_id } => {
                        format!("Removing {movie_id} from watched")
                    }
                    SyncItem::RemoveFromList {
                        list:
                            crate::types::ListID::PunchPlay(list_id)
                            | crate::types::ListID::TMDB(list_id),
                        movie_id,
                    } => format!("Removing {movie_id} from list {list_id}"),
                    SyncItem::RemoveFromList {
                        list: crate::types::ListID::Watchlist,
                        movie_id,
                    } => format!("Removing {movie_id} from watchlist"),
                    _ => String::default(),
                },
            ]
        })
    }

    fn update(&mut self, _event_handler: &mut EventHandler) {
        if !self.initialized || self.idle {
            return;
        }

        for (item, source, result) in self.rx_sync_response.as_ref().unwrap().try_iter() {
            match result {
                Ok(response) =>
                    if !response.status().is_success() {
                        self.errors.push((
                            item,
                            source,
                            format!(
                                "{} {}",
                                response.status(),
                                response
                                    .json::<Value>()
                                    .map(|x| x.to_string())
                                    .unwrap_or_default()
                            ),
                        ));
                    } else {
                        self.processing.remove(&(item, source));
                    },
                Err(error) => self.errors.push((item, source, format!("{:?}", error))),
            }
        }

        if self.errors.is_empty() {
            self.item = 0;
        }

        if self.processing.is_empty() {
            self.idle = true;
        }
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        image_renderer: &mut RatatuiImage,
        event_handler: &mut EventHandler,
    ) {
        if let Some((item, source, error)) = self.errors.first() {
            event_handler.clear();
            event_handler.bind_tab((None, None), "Navigate".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    history_syncer_processor.item = (history_syncer_processor.item == 0) as usize;
                }
            });
            event_handler.bind_horizontal((None, None), "Navigate".into(), None, |app, data| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    if let crate::event_handler::Data::Direction(dir, _) = data {
                        history_syncer_processor.item = dir as usize;
                    }
                }
            });

            event_handler.bind_enter((None, Some(0)), "Retry".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    let (source, item, _) = history_syncer_processor.errors.remove(0);
                    history_syncer_processor.retry_sync(source, item);
                }
            });
            event_handler.bind_enter((None, Some(1)), "Skip".into(), |app, _| {
                if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                    app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                {
                    let (item, source, _) = history_syncer_processor.errors.remove(0);
                    history_syncer_processor.processing.remove(&(item, source));
                }
            });

            let popup_area = widgets::window(
                frame,
                helpers::centered_area(11, 44, frame.area()),
                " History syncer error ",
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
                event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Processor::HistorySyncer(history_syncer_processor)) =
                            app.get_processor_mut(ProcessorDiscriminants::HistorySyncer)
                        {
                            let (item, source, _) = history_syncer_processor.errors.remove(0);

                            if i == 0 {
                                history_syncer_processor.retry_sync(item, source);
                            } else {
                                history_syncer_processor.processing.remove(&(item, source));
                            }
                        }
                    },
                );
            }
        }
    }
}
