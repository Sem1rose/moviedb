use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, channel},
    thread,
};

use log::error;
use ratatui::{
    Frame,
    layout::Margin,
    macros::{constraint, line, vertical},
    style::{Style, Stylize, palette::tailwind},
    widgets::{Gauge, Padding},
};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    event_handler::EventHandler,
    helpers,
    image_backend::RatatuiImage,
    popups::PopupTrait,
    types::{Collection, Entry, Movie, Person},
    widgets,
};

#[derive(Debug)]
enum Results {
    Movies(anyhow::Result<Vec<Movie>>),
    Watched(anyhow::Result<Vec<Entry>>),
    Persons(anyhow::Result<Vec<Person>>),
    Collections(anyhow::Result<Vec<Collection>>),
}

#[derive(Default)]
pub struct AppInitPopup {
    item:     usize,
    pub tick: u64,
    pub done: bool,
    home_dir: PathBuf,

    rx_results:     Option<Receiver<Results>>,
    throbber_state: ThrobberState,

    num_done:        usize,
    pub movies:      Vec<Movie>,
    pub watched:     Vec<Entry>,
    pub persons:     Vec<Person>,
    pub collections: Vec<Collection>,
}

impl AppInitPopup {
    pub fn new(home_dir: &Path) -> Self {
        let (tx_load, rx_results) = channel::<Results>();

        let home_dir = home_dir.to_path_buf();
        {
            let home_dir_cloned = home_dir.clone();
            let tx_load = tx_load.clone();
            thread::spawn(move || {
                let path = &home_dir_cloned.join(format!("movies.json",));
                _ = tx_load.send(Results::Movies(
                    fs::read_to_string(path)
                        .map(|read_result| {
                            serde_json::from_str::<Vec<_>>(&read_result).map_err(Into::into)
                        })
                        .map_err(Into::into)
                        .flatten(),
                ));
            });
        }
        {
            let home_dir_cloned = home_dir.clone();
            let tx_load = tx_load.clone();
            thread::spawn(move || {
                let path = &home_dir_cloned.join(format!("watched.json",));
                _ = tx_load.send(Results::Watched(
                    fs::read_to_string(path)
                        .map(|read_result| {
                            serde_json::from_str::<Vec<_>>(&read_result).map_err(Into::into)
                        })
                        .map_err(Into::into)
                        .flatten(),
                ));
            });
        }
        {
            let home_dir_cloned = home_dir.clone();
            let tx_load = tx_load.clone();
            thread::spawn(move || {
                let path = &home_dir_cloned.join(format!("persons.json",));
                _ = tx_load.send(Results::Persons(
                    fs::read_to_string(path)
                        .map(|read_result| {
                            serde_json::from_str::<Vec<_>>(&read_result).map_err(Into::into)
                        })
                        .map_err(Into::into)
                        .flatten(),
                ));
            });
        }
        {
            let home_dir_cloned = home_dir.clone();
            let tx_load = tx_load.clone();
            thread::spawn(move || {
                let path = &home_dir_cloned.join(format!("collections.json",));
                _ = tx_load.send(Results::Collections(
                    fs::read_to_string(path)
                        .map(|read_result| {
                            serde_json::from_str::<Vec<_>>(&read_result).map_err(Into::into)
                        })
                        .map_err(Into::into)
                        .flatten(),
                ));
            });
        }

        Self {
            rx_results: Some(rx_results),
            home_dir: home_dir,

            ..Default::default()
        }
    }
}

impl PopupTrait for AppInitPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        !self.done
    }

    fn update(&mut self) {
        self.tick += 1;
        if self.tick & 15 == 0 {
            self.throbber_state.calc_next();
        }
        if self.done {
            return;
        }

        macro_rules! response_error {
            ($error:expr,$name:expr, $home_dir:expr) => {
                error!("Error deserializing {} file: {}.\nRenaming corrupted file and creating a new database.", $error, $name);

                let path = $home_dir.join(format!("{}.json", $name));
                let mut renamed = $home_dir.join(format!("corrupted_{}.json", $name));
                let mut i = 1;
                while renamed.exists() {
                    renamed = $home_dir.join(format!("corrupted_{}_{i}.json", $name));
                    i += 1;
                }

                _ = fs::rename(&path, renamed);
                _ = fs::write(&path, "[]");
            };
        }

        if let Some(rx_results) = self.rx_results.as_ref() {
            for response in rx_results.try_iter() {
                self.num_done += 1;
                match response {
                    Results::Movies(result) => match result {
                        Ok(data) => self.movies = data,
                        Err(error) => {
                            response_error!(error, "movies", self.home_dir);
                        }
                    },
                    Results::Watched(result) => match result {
                        Ok(data) => self.watched = data,
                        Err(error) => {
                            response_error!(error, "watched", self.home_dir);
                        }
                    },
                    Results::Persons(result) => match result {
                        Ok(data) => self.persons = data,
                        Err(error) => {
                            response_error!(error, "persons", self.home_dir);
                        }
                    },
                    Results::Collections(result) => match result {
                        Ok(data) => self.collections = data,
                        Err(error) => {
                            response_error!(error, "collections", self.home_dir);
                        }
                    },
                }
            }
        }

        if self.num_done >= 4 {
            self.done = true;
        }
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut EventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
        key_event_handler.clear();
        key_event_handler.bind_key((None, None), 'q', "Quit".into(), |app, _| {
            app.quit = true;
        });
        let popup_area = widgets::window(
            frame,
            helpers::centered_area(8, 30, frame.area()),
            " Initializing ",
            true,
        );
        image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [_, message_area, _, throbber_area, progress_area] =
            vertical![>=1, ==2, ==1, ==1, ==3].areas(popup_area);

        frame.render_widget(line!("Loading").centered(), message_area);
        frame.render_stateful_widget(
            Throbber::default()
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400)),
            throbber_area.centered(constraint!(==1), constraint!(==1)),
            &mut self.throbber_state,
        );

        let progress_area = helpers::add_padding(progress_area, Padding::new(1, 1, 1, 0));
        let progress_gauge = Gauge::default()
            .ratio(self.num_done as f64 / 4.0)
            .gauge_style(
                Style::new()
                    .fg(tailwind::LIME.c500)
                    .bg(tailwind::GREEN.c900)
                    .italic(),
            )
            .label(
                format!("{}/{}", self.num_done, 4)
                    .fg(tailwind::PINK.c500)
                    .bold(),
            )
            .use_unicode(true);
        frame.render_widget(progress_gauge, progress_area);
    }
}
