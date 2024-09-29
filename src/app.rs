use crate::{config_tmdb::Conf, draw::Drawer};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
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
        if event::poll(std::time::Duration::from_millis(8))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Up => {
                            drawer.dec_selection(self);
                        }
                        KeyCode::Down => {
                            drawer.inc_selection(self);
                        }
                        _ => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    pub fn fetch_movies(&mut self, config: &Conf) {
        let file_path = config.home.join("ratings.json");

        let file_contents = fs::read_to_string(&file_path).unwrap_or_else(|_| {
            panic!("Couldn't read database contents at {}", file_path.display())
        });
        let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
            panic!(
                "Couldn't parse database contents at {}",
                file_path.display()
            )
        });

        let movies = json_contents
            .members()
            .map(|x| {
                Movie::new(
                    x["name"].to_string(),
                    x["rating"]
                        .to_string()
                        .parse()
                        .expect("Rating was not a number!"),
                    x["year"].to_string(),
                    x["id"]
                        .to_string()
                        .parse()
                        .expect("Couldn't parse movie id"),
                )
            })
            .collect::<Vec<Movie>>();

        self.set_movies(movies);
    }
}

#[derive(serde::Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub name: String,
    pub id: u32,
    pub year: String,
    pub rating: f32,
}

impl Movie {
    pub fn new(name: String, rating: f32, year: String, id: u32) -> Self {
        Movie {
            name,
            rating,
            year,
            id,
        }
    }
}
