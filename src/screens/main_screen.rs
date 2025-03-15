use crate::draw::{CurrentScreen, Drawer};

#[derive(Default)]
pub struct MainScreen {
    pub movies_visible: u32,
    pub scroll_pos: u32,
    pub selected: u32,
    // pub search_str: String,
}

impl MainScreen {
    pub fn default() -> Self {
        Self {
            movies_visible: 0,
            scroll_pos: 0,
            selected: 0,
            // search_str: String::default(),
        }
    }

    pub fn inc_movie_selection(&mut self, num_movies: usize) -> bool {
        if num_movies == 0 {
            return false;
        }
        if self.scroll_pos + self.selected < num_movies as u32 - 1 {
            if self.selected < self.movies_visible - 1 {
                self.selected += 1;
            } else {
                self.scroll_pos += 1;
            }
            return true;
        }

        false
    }

    pub fn dec_movie_selection(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;
            return true;
        } else if self.scroll_pos > 0 {
            self.scroll_pos -= 1;
            return true;
        }
        false
    }
}

impl Drawer {
    pub fn open_main_screen(&mut self) {
        self.close_popups();
        self.current_screen = CurrentScreen::MainScreen;
    }

    pub fn set_num_movies_visible(&mut self, num_movies_visible: u32) {
        if self.main_screen_options.movies_visible == 0
            || self.main_screen_options.movies_visible == num_movies_visible
        {
            self.main_screen_options.movies_visible = num_movies_visible;
        } else {
            self.clear_images(true);

            // don't know why i did all of this
            let current_pos =
                self.main_screen_options.scroll_pos + self.main_screen_options.selected;
            self.main_screen_options.movies_visible = num_movies_visible;
            if self.main_screen_options.selected >= num_movies_visible {
                self.main_screen_options.selected = num_movies_visible - 1;
            }

            self.main_screen_options.scroll_pos = current_pos - self.main_screen_options.selected;
        }
    }
}
