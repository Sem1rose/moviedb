use crate::{
    drawer::Drawer,
    popups::Popups,
    screens::Screens,
    tokens::*,
    types::{initialize_terminal, reset_terminal, Movie, OldMovie, Term},
    KeyEventHandler,
};
use log::{error, warn};
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventState, KeyModifiers};
use std::{
    fs,
    path::PathBuf,
    time::Duration,
};

pub struct App {
    home: PathBuf,
    cache: PathBuf,
    pub quit: bool,
    pub movies: Vec<Movie>,

    terminal: Term,
    pub drawer: Drawer,
    pub key_event_handler: KeyEventHandler,

    pub tmdb_tokens: TMDBTokens,
    pub omdb_tokens: OMDBTokens,
    pub trakt_tokens: TraktTokens,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let home_dir = dirs::config_dir()
            .expect("Couldn't get user's config dir")
            .join("moviedb");
        let cache_dir = dirs::cache_dir()
            .expect("Couldn't get user's cache dir")
            .join("moviedb");

        Self {
            movies: vec![],
            terminal: initialize_terminal()?,
            drawer: Drawer::new(&home_dir, &cache_dir),
            key_event_handler: KeyEventHandler::default(),

            tmdb_tokens: TMDBTokens::new(&home_dir),
            omdb_tokens: OMDBTokens::new(&home_dir),
            trakt_tokens: TraktTokens::new(&home_dir),

            quit: false,
            home: home_dir,
            cache: cache_dir,
        }
        .fetch_movies()
    }

    pub fn fetch_movies(mut self) -> anyhow::Result<Self> {
        let file_path = &self.home.join("ratings.json");

        let read_result = fs::read_to_string(file_path);
        if let Err(error) = read_result {
            error!("Error reading ratings file: {}.\nRenaming corrupted file and creating a new database.", error);

            let mut renamed = self.home.join("corrupted_ratings.json");
            let mut i = 1;
            while renamed.exists() {
                renamed = self.home.join(format!("corrupted_ratings_{i}.json"));
                i += 1;
            }

            fs::rename(file_path, renamed)?;

            fs::write(&self.home.join("ratings.json"), "[]")?;
            return Ok(self);
        }
        let contents = fs::read_to_string(file_path)?;

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

                fs::rename(file_path, renamed)?;

                fs::write(&self.home.join("ratings.json"), "[]")?;
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

                            // while event::poll(Duration::from_millis(0))? {
                            //     if let Ok(event) = event::read() {
                            //         self.handle_event(event);
                            //     }
                            // }
                        }
                    }
                } else {
                    if let Ok(event) = event::read() {
                        self.handle_event(event);

                        // while event::poll(Duration::from_millis(0))? {
                        //     if let Ok(event) = event::read() {
                        //         self.handle_event(event);
                        //     }
                        // }
                    }
                }
            }

            if self.quit {
                break;
            }
        }

        reset_terminal(&mut self.terminal)?;

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

    pub fn set_tmdb_user_tokens(&mut self) {
        if let Some(Popups::TMDBInit(tmdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = tmdb_init_popup.tokens.take() {
                self.tmdb_tokens.set_creds(tokens).unwrap();

                self.drawer.close_popups();
            }
        }
    }
    pub fn set_omdb_user_tokens(&mut self) {
        if let Some(Popups::OMDBInit(omdb_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = omdb_init_popup.tokens.take() {
                self.omdb_tokens.set_creds(tokens).unwrap();

                self.drawer.close_popups();
            }
        }
    }
    pub fn set_trakt_user_tokens(&mut self) {
        if let Some(Popups::TraktInit(trakt_init_popup)) = self.drawer.active_popup.as_mut() {
            if let Some(tokens) = trakt_init_popup.tokens.take() {
                self.trakt_tokens.set_creds(tokens).unwrap();

                self.drawer.close_popups();
            }
        }
    }

    fn save_movies(&self) -> anyhow::Result<()> {
        let string = serde_json::to_string_pretty(self.movies.as_slice()).unwrap();

        fs::rename(
            &self.home.join("ratings.json"),
            self.home.join("ratings.json").with_extension("json.bak"),
        )?;
        fs::write(&self.home.join("ratings.json"), string)?;

        Ok(())
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

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => {
                if let Some((callback, data)) =
                    self.key_event_handler.handle_key_event(event, &self.drawer)
                {
                    callback(self, data);
                }
            }
            Event::Mouse(event) => {
                if let Some((callback, data)) = self
                    .key_event_handler
                    .handle_mouse_event(event, &self.drawer)
                {
                    callback(self, data);
                }
            }
            Event::FocusGained => (),
            Event::FocusLost => (),
            Event::Paste(string) => {
                for c in string.chars() {
                    if let Some((callback, data)) = self.key_event_handler.handle_key_event(
                        KeyEvent {
                            code: event::KeyCode::Char(c),
                            modifiers: KeyModifiers::NONE,
                            kind: event::KeyEventKind::Press,
                            state: KeyEventState::NONE,
                        },
                        &self.drawer,
                    ) {
                        callback(self, data);
                    }
                }
            }
            Event::Resize(_, _) => (),
        }
    }
}
