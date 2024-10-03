use crate::{
    config_tmdb::Conf,
    draw::Drawer,
    draw::{CurrentScreen, Popup},
    tmdb::DetailsResponse,
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use serde::Deserialize;
use std::{error::Error, fs};

pub struct App {
    pub single_shot: bool,
    pub should_quit: bool,
    pub movies: Vec<Movie>,
}

impl App {
    pub fn new(_single_shot: bool) -> Self {
        Self {
            single_shot: _single_shot,
            should_quit: false,
            movies: vec![],
        }
    }

    pub fn set_movies(&mut self, _movies: Vec<Movie>) {
        self.movies = _movies;
    }

    pub fn handle(&mut self, drawer: &mut Drawer) -> Result<(), Box<dyn Error>> {
        if event::poll(std::time::Duration::from_millis(0))? {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent { code, kind, .. }) => {
                    if kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    match code {
                        KeyCode::Char('q') => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Char('a') => {
                            if drawer.popup.is_none() {
                                drawer.open_add_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.popup.is_some() && drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Char('e') => {
                            if drawer.popup.is_none() {
                                drawer.open_edit_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.popup.is_some() && drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Char('d') => {
                            if drawer.popup.is_none() {
                                drawer.open_remove_movie_popup();
                                drawer.clear_images = true;
                            } else if drawer.popup.is_some() && drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                        KeyCode::Delete => {
                            if drawer.popup.is_none() {
                                drawer.open_remove_movie_popup();
                                drawer.clear_images = true;
                            }
                        }
                        KeyCode::Esc => {
                            if drawer.popup.is_some() {
                                drawer.close_add_movie_popup();
                                drawer.clear_images(false);
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Up => {
                            drawer.dec_selection(self);
                        }
                        KeyCode::Down => {
                            drawer.inc_selection(self);
                        }
                        KeyCode::Right => {
                            drawer.inc_selection_horiz(self);
                        }
                        KeyCode::Left => {
                            drawer.dec_selection_horiz(self);
                        }
                        KeyCode::Enter => match drawer.popup {
                            Some(Popup::AddMovie) => {
                                if *drawer.add_movie_popup_options.failed.lock().unwrap() {
                                    drawer.close_add_movie_popup();
                                    drawer.clear_images(false);
                                } else if drawer.add_movie_popup_options.phase == 0
                                    && drawer.add_movie_popup_options.search_input.value() != ""
                                {
                                    drawer.add_movie_popup_options.finished_search_input = true;
                                    drawer.update = true;
                                } else if drawer.add_movie_popup_options.phase == 2 {
                                    drawer.add_movie_popup_options.movie_selected = true;
                                    drawer.update = true;
                                } else if drawer.add_movie_popup_options.phase == 3
                                    && drawer.add_movie_popup_options.search_input.value() != ""
                                    && drawer.add_movie_popup_options.user_rating_valid
                                {
                                    drawer.add_movie_popup_options.got_user_rating = true;
                                    drawer.update = true;
                                }
                            }
                            Some(Popup::EditMovie) => {
                                if drawer.edit_movie_popup_options.errored {
                                    drawer.close_edit_movie_popup();
                                    drawer.clear_images(false);
                                    drawer.update = true;
                                } else if !drawer.edit_movie_popup_options.got_user_rating
                                    && drawer.edit_movie_popup_options.user_rating_input.value()
                                        != ""
                                    && drawer.edit_movie_popup_options.user_rating_valid
                                {
                                    drawer.edit_movie_popup_options.got_user_rating = true;
                                    drawer.update = true;
                                }
                            }
                            Some(Popup::RemoveMovie) => {
                                if drawer.remove_movie_popup_options.errored {
                                    drawer.close_remove_movie_popup();
                                    drawer.clear_images(false);
                                } else if drawer.remove_movie_popup_options.selected == 1 {
                                    drawer.remove_movie_popup_options.confirmed = true;
                                    drawer.update = true;
                                } else if drawer.remove_movie_popup_options.selected == 0 {
                                    drawer.close_remove_movie_popup();
                                    drawer.clear_images(false);
                                }
                            }
                            _ => {}
                        },
                        _ => {
                            if drawer.accepting_input {
                                drawer.handle_input(&event);
                            }
                        }
                    }
                }
                Event::Resize(_, _) => {
                    drawer.clear_images(true);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn fetch_movies(&mut self, config: &Conf) {
        let file_path = config.home.join("ratings.json");

        let file_contents = fs::read_to_string(&file_path).unwrap_or_else(|_| {
            panic!("Couldn't read database contents at {}", file_path.display())
        });
        // let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
        //     panic!(
        //         "Couldn't parse database contents at {}",
        //         file_path.display()
        //     )
        // });

        // let movies = json_contents
        //     .members()
        //     .map(|x| {
        //         Movie::new(
        //             x["name"].to_string(),
        //             x["rating"]
        //                 .to_string()
        //                 .parse()
        //                 .expect("Rating was not a number!"),
        //             x["year"].to_string(),
        //             x["id"]
        //                 .to_string()
        //                 .parse()
        //                 .expect("Couldn't parse movie id"),
        //         )
        //     })
        //     .collect::<Vec<Movie>>();

        let movies = serde_json::from_str(&file_contents).expect("couldn't deserialize json!");

        self.set_movies(movies);
    }

    pub fn save_movies(&self, config: &Conf) -> Result<(), Box<dyn Error>> {
        let string = serde_json::to_string_pretty(self.movies.as_slice()).unwrap();

        fs::rename(
            config.home.join("ratings.json"),
            config.home.join("ratings.json.bak"),
        )?;
        fs::write(config.home.join("ratings.json"), string)?;

        Ok(())
    }
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub name: String,
    pub id: u32,
    pub year: String,
    pub user_rating: f64,
    pub vote_average: f64,
    pub genres: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<u32>,
    pub overview: String,
    pub runtime: u32,
    pub released: bool,
    pub tagline: String,
    pub vote_count: u32,
}

impl Movie {
    pub fn new(
        name: String,
        user_rating: f64,
        vote_average: f64,
        year: String,
        id: u32,
        genres: Vec<String>,
        overview: String,
        collection: Option<String>,
        collection_id: Option<u32>,
        runtime: u32,
        released: bool,
        tagline: String,
        vote_count: u32,
    ) -> Self {
        Movie {
            name,
            user_rating,
            vote_average,
            year,
            id,
            genres,
            collection,
            collection_id,
            overview,
            runtime,
            released,
            tagline,
            vote_count,
        }
    }

    pub fn from(movie_details: DetailsResponse, user_rating: f64) -> Self {
        let mut collection: Option<String> = None;
        let mut collection_id: Option<u32> = None;
        if movie_details.belongs_to_collection.is_some() {
            collection = Some(movie_details.belongs_to_collection.clone().unwrap().name);
            collection_id = Some(movie_details.belongs_to_collection.clone().unwrap().id);
        }
        Self {
            name: movie_details.title,
            user_rating,
            vote_average: movie_details.vote_average,
            year: movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
            id: movie_details.id,
            genres: movie_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview: movie_details.overview,
            collection,
            collection_id,
            runtime: movie_details.runtime,
            released: movie_details.status == "Released",
            tagline: movie_details.tagline,
            vote_count: movie_details.vote_count,
        }
    }
}
