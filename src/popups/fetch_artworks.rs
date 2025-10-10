use crate::{
    app::App, config::Config, custom::helpers::center_rect, draw::Drawer, tmdb, trakt, types::*,
};
// use log::error;
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use ratatui_macros::{horizontal, vertical};
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};
use style::palette::tailwind;

#[derive(Default)]
pub struct FetchArtworksPopup {
    pub done: bool,

    num_movies: usize,
    pub progress: usize,

    tx_fetch_request: Option<Sender<Option<MovieID>>>,
    rx_fetch_response: Option<Receiver<(MovieID, Result<()>)>>,

    errored: Option<u32>,
}

impl FetchArtworksPopup {
    pub fn begin(&mut self, app: &mut App) -> Result<()> {
        *self = Self::default();
        self.num_movies = app.movies.len();

        self.fetch_artworks(app)
    }

    pub fn start_thread(&mut self, app: &App) {
        let (tx_fetch_request, rx_fetch_request) = channel::<Option<MovieID>>();
        let (tx_fetch_response, rx_fetch_response) = channel::<(MovieID, Result<()>)>();

        let conf = app.config.clone();
        let tmdb_conf = app.tmdb_config.clone();
        let trakt_conf = app.trakt_config.clone();

        thread::spawn(move || {
            for fetch_request in rx_fetch_request.iter() {
                if fetch_request.is_none() {
                    break;
                }

                let request = fetch_request.unwrap();
                let tx_response = tx_fetch_response.clone();

                let conf_owned = conf.clone();
                let tmdb_conf_owned = tmdb_conf.clone();
                let trakt_conf_owned = trakt_conf.clone();
                thread::spawn(move || {
                    let result = tmdb::get_movie_poster_banner(
                        &conf_owned,
                        &tmdb_conf_owned,
                        request.tmdb,
                        true,
                    );

                    _ = if let Ok(true) = result {
                        tx_response.send((request, Ok(())))
                    } else {
                        let result = trakt::get_movie_poster_banner(
                            &conf_owned,
                            &trakt_conf_owned,
                            request.imdb.clone(),
                            true,
                        );

                        if let Err(error) = result {
                            tx_response.send((request, Err(error)))
                        } else {
                            tx_response.send((request, Ok(())))
                        }
                    };
                });
            }
        });

        self.tx_fetch_request = Some(tx_fetch_request);
        self.rx_fetch_response = Some(rx_fetch_response);
    }

    pub fn check_done(&mut self, app: &App) -> bool {
        if self.progress == app.movies.len() {
            self.done = true;
        }
        self.done
    }

    fn check_artwork_fetched(&self, config: &Config, id: u32) -> bool {
        config
            .dirs
            .poster_cache
            .join(format!("{}.jpg", id))
            .is_file()
            && config
                .dirs
                .backdrop_cache
                .join(format!("{}.jpg", id))
                .is_file()
    }

    pub fn fetch_artworks(&mut self, app: &mut App) -> Result<()> {
        self.start_thread(app);
        for movie in &app.movies {
            if !self.check_artwork_fetched(&app.config, movie.id.tmdb) {
                _ = self
                    .tx_fetch_request
                    .as_ref()
                    .unwrap()
                    .send(Some(movie.id.clone()));
            } else {
                self.progress += 1;
            }
        }

        Ok(())
    }

    pub fn read_threads_responses(&mut self) -> Result<()> {
        for (id, fetch_result) in self.rx_fetch_response.as_ref().unwrap().try_iter() {
            if fetch_result.is_err() {
                // if let Err(error) = fetch_result {
                // error!("error while downloading {}: {error}", id.tmdb);

                self.errored = Some(id.tmdb);
                _ = self.tx_fetch_request.as_ref().unwrap().send(Some(id));
            } else {
                if let Some(i) = self.errored {
                    if i == id.tmdb {
                        self.errored = None;
                    }
                }
                self.progress += 1;
            }
        }

        Ok(())
    }
}

impl Drawer {
    pub(crate) fn draw_fetch_artworks_popup(
        &mut self,
        frame: &mut Frame,
        // app: &mut App,
    ) -> Result<()> {
        let frame_area = frame.area();

        let progress = self.fetch_artwork_popup.progress;
        let num_movies = self.fetch_artwork_popup.num_movies;

        let popup_area = center_rect(
            frame_area,
            Constraint::Percentage(50),
            Constraint::Length(12),
        );

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Working...")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let layout = vertical![==1, ==1, ==3, ==3, ==1, ==1, ==1].split(popup.inner(popup_area));

        let info_text = "Getting movie posters...";
        let [text_lay, throbber_lay] = horizontal![==(info_text.len() as u16), ==1]
            .flex(Flex::Center)
            .areas(layout[1]);

        frame.render_widget(info_text, text_lay);

        let throbber = throbber_widgets_tui::Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));
        frame.render_stateful_widget(throbber, throbber_lay, &mut self.throbber_state);

        let [progress_lay] = horizontal![==(layout[3].width - 6)]
            .flex(Flex::Center)
            .areas(layout[3]);

        let progress_guage = Gauge::default()
            .ratio(progress as f64 / num_movies as f64)
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(format!("{}/{}", progress, num_movies).fg(tailwind::PINK.c500))
            .use_unicode(true);

        frame.render_widget(progress_guage, progress_lay);

        if let Some(id) = self.fetch_artwork_popup.errored {
            let errored_text = format!("movie {id} errored, retrying!!");

            let [text_lay] = horizontal![==(errored_text.len() as u16)]
                .flex(Flex::Center)
                .areas(layout[5]);

            frame.render_widget(errored_text, text_lay);
        }

        Ok(())
    }
}
