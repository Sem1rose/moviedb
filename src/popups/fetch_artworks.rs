use crate::{
    app::{App, Errors},
    draw::Drawer,
    tmdb, trakt,
};
use log::{debug, error};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use ratatui_macros::{constraints, horizontal, line, span, text, vertical};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use style::palette::tailwind;

// #[derive(Default)]
pub struct FetchArtworksPopup {
    pub started: bool,
    pub progress: u32,

    // tx_fetch_request: Sender<u32>,
    tx_fetch_request: Sender<(u32, String)>,
    rx_fetch_response: Receiver<((u32, String), Result<(), Errors>)>,

    errored: Option<u32>,
}

impl FetchArtworksPopup {
    pub fn new(app: &App) -> Self {
        let (tx_fetch_request, rx_fetch_request) = mpsc::channel::<(u32, String)>();
        let (tx_fetch_response, rx_fetch_response) =
            mpsc::channel::<((u32, String), Result<(), Errors>)>();

        let conf = app.config.clone();
        let tmdb_conf = app.tmdb_config.clone();
        let trakt_conf = app.trakt_config.clone();

        thread::spawn(move || loop {
            for fetch_request in rx_fetch_request.try_iter() {
                let tx_response = tx_fetch_response.clone();

                let conf_owned = conf.clone();
                let tmdb_conf_owned = tmdb_conf.clone();
                let trakt_conf_owned = trakt_conf.clone();
                thread::spawn(move || {
                    let result = trakt::get_movie_poster_banner(
                        &conf_owned,
                        &trakt_conf_owned,
                        fetch_request.1.clone(),
                        false,
                    );

                    if let Ok(false) = result {
                        let result = tmdb::get_movie_poster_banner(
                            &conf_owned,
                            &tmdb_conf_owned,
                            fetch_request.0,
                            true,
                        );

                        if let Err(error) = result {
                            let _ = tx_response.send((fetch_request, Err(error)));
                        } else {
                            let _ = tx_response.send((fetch_request, Ok(())));
                        }
                    } else if result.is_err() {
                        let result = tmdb::get_movie_poster_banner(
                            &conf_owned,
                            &tmdb_conf_owned,
                            fetch_request.0,
                            true,
                        );

                        if let Err(error) = result {
                            let _ = tx_response.send((fetch_request, Err(error)));
                        } else {
                            let _ = tx_response.send((fetch_request, Ok(())));
                        }
                    } else {
                        let _ = tx_response.send((fetch_request, Ok(())));
                    }
                });
            }
        });

        Self {
            started: false,
            progress: 0,
            tx_fetch_request,
            rx_fetch_response,
            errored: None,
        }
    }
}

impl FetchArtworksPopup {
    pub fn begin(&mut self) {
        self.started = false;
        self.progress = 0;
    }
}

impl Drawer {
    fn start_fetch_artworks_thread(&mut self, app: &mut App) -> Result<(), Errors> {
        let contents = std::fs::read_to_string(&app.config.dirs.cached_movies_file)?;
        let movies_cached: Vec<_> = contents
            .split_ascii_whitespace()
            .map(|x| x.to_string())
            .collect();

        for movie in &app.movies {
            // let movie_id = movie.tmdb_id;
            let movie_id = (movie.tmdb_id, movie.imdb_id.clone());
            if !movies_cached.contains(&movie_id.0.to_string()) {
                let _ = self
                    .fetch_artwork_popup_options
                    .tx_fetch_request
                    .send(movie_id);
            } else {
                self.fetch_artwork_popup_options.progress += 1;
            }
        }

        Ok(())
    }

    fn read_fetch_threads_responses(&mut self, app: &mut App) -> Result<(), Errors> {
        let contents = std::fs::read_to_string(&app.config.dirs.cached_movies_file)?;

        let mut movies_cached: Vec<_> = contents
            .split_ascii_whitespace()
            .map(|x| x.to_string())
            .collect();

        for (id, fetch_result) in self
            .fetch_artwork_popup_options
            .rx_fetch_response
            .try_iter()
        {
            if let Err(error) = fetch_result {
                error!("error while downloading {}: {error}", id.0);

                self.fetch_artwork_popup_options.errored = Some(id.0);

                // let _ = self.fetch_artwork_popup_options.tx_fetch_request.send(id);
            } else {
                self.fetch_artwork_popup_options.progress += 1;

                movies_cached.push(id.0.to_string());
            }
        }

        std::fs::write(
            &app.config.dirs.cached_movies_file,
            movies_cached.join("\n"),
        )?;

        Ok(())
    }

    pub(crate) fn draw_fetch_artworks_popup(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
    ) -> Result<bool, Errors> {
        let frame_area = frame.area();

        if !self.fetch_artwork_popup_options.started {
            self.fetch_artwork_popup_options.started = true;

            self.start_fetch_artworks_thread(app)?;
        }

        self.read_fetch_threads_responses(app)?;

        let progress = self.fetch_artwork_popup_options.progress;
        let num_movies = app.movies.len();

        if progress == num_movies as u32 {
            return Ok(true);
        }

        let popup_area = self.center(
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
        // let layout = Layout::vertical([
        //     Constraint::Length(1),
        //     Constraint::Length(1),
        //     Constraint::Length(3),
        //     Constraint::Length(3),
        //     Constraint::Length(1),
        //     Constraint::Length(1),
        //     Constraint::Length(1),
        // ])
        // .split(popup.inner(popup_area));

        let info_text = "Getting movie posters...";
        let [text_lay, throbber_lay] = horizontal![==(info_text.len() as u16), ==1]
            .flex(Flex::Center)
            .areas(layout[1]);
        // let [text_lay, throbber_lay] = Layout::horizontal(vec![
        //     Constraint::Length(info_text.len() as u16),
        //     Constraint::Length(1),
        // ])
        // .flex(Flex::Center)
        frame.render_widget(info_text, text_lay);

        let throbber = throbber_widgets_tui::Throbber::default()
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
            .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));
        frame.render_stateful_widget(throbber, throbber_lay, &mut self.throbber_state);

        let [progress_lay] = horizontal![==(layout[3].width - 6)]
            .flex(Flex::Center)
            .areas(layout[3]);
        // let [progress_lay] = Layout::horizontal(vec![Constraint::Length(layout[3].width - 6)])
        //     .flex(Flex::Center)
        //     .areas(layout[3]);
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

        if let Some(id) = self.fetch_artwork_popup_options.errored {
            let errored_text = format!("movie {id} errored, retrying!!");
            // let [text_lay] =
            //     Layout::horizontal(vec![Constraint::Length(errored_text.len() as u16)])
            //         .flex(Flex::Center)
            //         .areas(layout[1]);

            let [text_lay] = horizontal![==(errored_text.len() as u16)]
                .flex(Flex::Center)
                .areas(layout[1]);

            frame.render_widget(errored_text, text_lay);
        }

        Ok(false)
    }
}
