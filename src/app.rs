pub struct App {
    pub single_shot: bool,
    pub should_quit: bool,
    pub movies: Vec<Movie>,
    pub movies_list_screen_options: MainScreen,
}

impl App {
    pub fn new(_single_shot: bool) -> Self {
        Self {
            single_shot: _single_shot,
            should_quit: false,
            movies: vec![],
            movies_list_screen_options: MainScreen::default(),
        }
    }

    pub fn set_movies(&mut self, _movies: Vec<Movie>) {
        self.movies = _movies;
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: u32) {
        if self.movies_list_screen_options.movies_visible == 0
            || num_movies_visible == self.movies_list_screen_options.movies_visible
        {
            self.movies_list_screen_options.movies_visible = num_movies_visible;
        } else {
            // don't know why i did all of this
            let current_pos = self.movies_list_screen_options.scroll_pos
                + self.movies_list_screen_options.selected;
            self.movies_list_screen_options.movies_visible = num_movies_visible;
            if self.movies_list_screen_options.selected >= num_movies_visible {
                self.movies_list_screen_options.selected = num_movies_visible - 1;
            }

            self.movies_list_screen_options.scroll_pos =
                current_pos - self.movies_list_screen_options.selected;
        }
    }

    pub fn inc_movie_selection(&mut self) {
        if self.movies_list_screen_options.scroll_pos + self.movies_list_screen_options.selected
            < self.movies.len() as u32 - 1
        {
            if self.movies_list_screen_options.selected
                < self.movies_list_screen_options.movies_visible - 1
            {
                self.movies_list_screen_options.selected += 1;
            } else {
                self.movies_list_screen_options.scroll_pos += 1;
            }
        }
    }

    pub fn dec_movie_selection(&mut self) {
        if self.movies_list_screen_options.selected > 0 {
            self.movies_list_screen_options.selected -= 1;
        } else if self.movies_list_screen_options.scroll_pos > 0 {
            self.movies_list_screen_options.scroll_pos -= 1;
        }
    }
}
pub struct MainScreen {
    pub movies_visible: u32,
    pub scroll_pos: u32,
    pub selected: u32,
    pub search_str: String,
}

impl MainScreen {
    pub fn default() -> Self {
        Self {
            movies_visible: 0,
            scroll_pos: 0,
            selected: 0,
            search_str: String::default(),
        }
    }
}

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
