use crate::{
    drawer::Drawer,
    popups::Popups,
    screens::Screens,
    tokens::*,
    types::{initialize_terminal, Movie, OldMovie, Term},
    KeyEventHandler,
};
use log::{error, warn};
use ratatui::crossterm::event::{self, Event};
use std::{
    fs::{read_to_string, rename, write},
    path::PathBuf,
    time::Duration,
};

pub struct App {
    home: PathBuf,
    cache: PathBuf,

    pub movies: Vec<Movie>,
    pub quit: bool,

    terminal: Term,
    pub key_event_handler: KeyEventHandler,
    pub drawer: Drawer,

    pub trakt_tokens: TraktTokens,
    pub tmdb_tokens: TMDBTokens,
    pub omdb_tokens: OMDBTokens,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let home = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join("moviedb");
        let cache = dirs::cache_dir()
            .expect("Couldn't get user's cache dir")
            .join("moviedb");

        let file_contents =
            read_to_string(home.join(".credentials")).expect("Couldn't read credentials");
        let creds = serde_json::from_str(&file_contents).expect("Couldn't deserialize credentials");

        Self {
            quit: false,

            movies: vec![],
            terminal: initialize_terminal()?,
            key_event_handler: KeyEventHandler::default(),
            drawer: Drawer::new(&cache),

            tmdb_tokens: TMDBTokens::new(&creds),
            omdb_tokens: OMDBTokens::new(&creds),
            trakt_tokens: TraktTokens::new(&creds),

            home,
            cache,
        }
        .fetch_movies()
    }

    pub fn fetch_movies(mut self) -> anyhow::Result<Self> {
        let file_path = &self.home.join("ratings.json");

        let read_result = read_to_string(file_path);
        if let Err(error) = read_result {
            error!("Error reading ratings file: {}.\nRenaming corrupted file and creating a new database.", error);

            let mut renamed = self.home.join("corrupted_ratings.json");
            let mut i = 1;
            while renamed.exists() {
                renamed = self.home.join(format!("corrupted_ratings_{i}.json"));
                i += 1;
            }

            rename(file_path, renamed)?;

            write(&self.home.join("ratings.json"), "[]")?;
            return Ok(self);
        }
        let contents = read_to_string(file_path)?;

        let result = serde_json::from_str(&contents);
        if let Err(error) = result {
            warn!(
                "Error deserializing ratings file: {}.\nRetrying with the old format...",
                error
            );

            let result = serde_json::from_str::<Vec<OldMovie>>(&contents);
            if let Err(error) = result {
                error!("Error deserializing ratings file: {}.\nRenaming corrupted file and creating a new database.", error);

                let mut renamed = self.home.join("corrupted_ratings.json");
                let mut i = 1;
                while renamed.exists() {
                    renamed = self.home.join(format!("corrupted_ratings_{i}.json"));
                    i += 1;
                }

                rename(file_path, renamed)?;

                write(&self.home.join("ratings.json"), "[]")?;
            } else {
                let movies: Vec<Movie> = result.unwrap().into_iter().map(|x| x.into()).collect();
                self.set_movies(Self::remove_duplicates(movies));
            }
        } else {
            let movies: Vec<Movie> = result.unwrap();
            self.set_movies(Self::remove_duplicates(movies));
        }

        Ok(self)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            main_screen.set_movies(&self.movies);
        }

        loop {
            self.key_event_handler.clear();

            self.terminal
                .draw(|frame| {
                    self.drawer.render_app(frame, &mut self.key_event_handler);
                })
                .map(|_| ())?;

            for callback in self.key_event_handler.execute_immediates() {
                callback(self, crate::key_event_handler::Data::None);
            }

            if !self.drawer.check_refresh_immediate() {
                if self.drawer.check_refresh_delayed() {
                    if event::poll(Duration::from_millis(10))? {
                        if let Ok(event) = event::read() {
                            self.handle_event(event);
                        }
                    }
                } else {
                    if let Ok(event) = event::read() {
                        self.handle_event(event);
                    }
                }
            }

            if self.quit {
                break;
            }
        }

        Ok(())
    }

    pub fn set_movies(&mut self, _movies: Vec<Movie>) {
        self.movies = _movies;
    }

    pub fn add_play(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            if let Some(Popups::EditMovie(edit_movie_popup)) = self.drawer.active_popup.as_mut() {
                let mut movie = self.movies.remove(
                    self.movies
                        .iter()
                        .position(|x| x == main_screen.current_movie().unwrap())
                        .unwrap(),
                );
                movie.add_play(
                    chrono::Local::now(),
                    edit_movie_popup.user_rating_input.lines()[0]
                        .parse()
                        .unwrap(),
                );
                self.movies.push(movie);
            }
            main_screen.set_movies(&self.movies);
            main_screen.goto_index(-1);
        }
        self.save_movies().unwrap();
    }
    pub fn add_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            if let Some(Popups::AddMovie(add_movie_popup)) = self.drawer.active_popup.as_mut() {
                let tmdb_movie_details = add_movie_popup.tmdb_movie_details_result.take().unwrap();
                let trakt_movie_details = add_movie_popup.trakt_movie_details_result.take();
                let omdb_movie_details = add_movie_popup.omdb_movie_details_result.take();

                let mut movie = Movie::from(tmdb_movie_details, add_movie_popup.user_rating);
                let x = self.movies.iter().position(|x| movie == x);
                if x.is_some() {
                    let mut movie = self.movies.remove(x.unwrap());
                    movie.add_play(chrono::Local::now(), add_movie_popup.user_rating);
                    self.movies.push(movie);
                } else {
                    if let Some(trakt) = trakt_movie_details {
                        movie.add_trakt_details(trakt);
                    }
                    if let Some(omdb) = omdb_movie_details {
                        movie.add_omdb_details(omdb);
                    }
                    self.movies.push(movie);
                }
            }
            main_screen.set_movies(&self.movies);
            main_screen.goto_index(-1);
            self.drawer.close_popups();
        }
        self.save_movies().unwrap();
    }
    pub fn edit_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            if let Some(Popups::EditMovie(edit_movie_popup)) = self.drawer.active_popup.as_ref() {
                let index = self
                    .movies
                    .iter()
                    .position(|x| x == main_screen.current_movie().unwrap())
                    .unwrap();
                self.movies[index].edit_user_rating(
                    edit_movie_popup.user_rating_input.lines()[0]
                        .parse::<f64>()
                        .unwrap(),
                );
            }
            main_screen.set_movies(&self.movies);
        }
        self.save_movies().unwrap();
    }
    pub fn remove_movie(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.drawer.current_screen.as_mut() {
            let index = self
                .movies
                .iter()
                .position(|x| x == main_screen.current_movie().unwrap())
                .unwrap();
            self.movies.remove(index);
            main_screen.set_movies(&self.movies);
        }
        self.save_movies().unwrap();
    }

    fn remove_duplicates(mut movies: Vec<Movie>) -> Vec<Movie> {
        let mut new_movies = vec![];

        let mut i = movies.len() - 1;
        while i < movies.len() {
            let mut new_movie = movies[i].clone();
            let mut id = movies.iter().position(|x| movies[i] == &x).unwrap();
            while id != i {
                new_movie.add_play(movies[id].plays[0].0.clone(), movies[id].get_user_rating());

                movies.remove(id);
                i -= 1;
                id = movies.iter().position(|x| movies[i] == &x).unwrap();
            }

            movies.remove(i);
            new_movies.insert(0, new_movie);
            _ = i == 0 && break; // points for epic bash syntax

            i -= 1;
        }

        new_movies
    }

    fn save_movies(&self) -> anyhow::Result<()> {
        let string = serde_json::to_string_pretty(self.movies.as_slice()).unwrap();

        rename(
            &self.home.join("ratings.json"),
            self.home.join("ratings.json").with_extension("json.bak"),
        )?;
        write(&self.home.join("ratings.json"), string)?;

        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => {
                if let Some((callback, data)) =
                    self.key_event_handler.handle_key_event(event, &self.drawer)
                {
                    callback(self, data);
                }
            }
            Event::FocusGained => (),
            Event::FocusLost => (),
            Event::Mouse(_) => (),
            Event::Paste(_) => (),
            Event::Resize(_, _) => (),
        }
    }
}
