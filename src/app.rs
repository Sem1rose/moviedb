use crate::draw::Drawer;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::Result;

pub struct App {
    // pub socket: String,
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

    pub fn handle(&mut self, drawer: &mut Drawer) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(8))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Up => {
                            drawer.dec_movie_selection();
                        }
                        KeyCode::Down => {
                            drawer.inc_movie_selection(self.movies.len());
                        }
                        _ => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Movie {
    pub name: String,
    pub url: String,
    pub year: String,
    pub rating: f32,
    pub trakt_id: u32,
}

impl Movie {
    pub fn new(_name: String, _rating: f32, _url: String, _year: String, _trakt_id: u32) -> Self {
        Movie {
            name: _name,
            rating: _rating,
            url: _url,
            year: _year,
            trakt_id: _trakt_id,
        }
    }
}
