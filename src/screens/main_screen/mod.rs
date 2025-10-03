mod movie_description;
mod movies_list;
use crate::{
    app::App,
    draw::Drawer,
    screens::{
        main_screen::{movie_description::MovieDescription, movies_list::MoviesList},
        Screens,
    },
    types::*,
};
use crossterm::event::KeyModifiers;
use log::error;
use ratatui::style::Stylize;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, vertical};
use style::palette::tailwind;
// use threadpool::ThreadPool;

//                    id     backdrop/poster
// pub type MovieID = (usize, bool);

#[derive(Default)]
pub struct MainScreen {
    pub movies_list: MoviesList,
    pub movie_description: MovieDescription,
}

// impl Default for MainScreen {
//     fn default() -> Self {
//         Self {
//             movies_list: MoviesList::default(),
//             movie_description: MovieDescription::default(),
//         }
//     }
// }

impl Drawer {
    pub fn main_screen_handle_key_events(&mut self, app: &mut App, event: KeyEvent) {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return;
        }

        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('a') => {
                self.open_add_movie_popup();
            }
            KeyCode::Char('e') => {
                self.open_edit_movie_popup(app);
            }
            KeyCode::Char('d') => {
                self.open_remove_movie_popup();
            }
            KeyCode::Delete => {
                self.open_remove_movie_popup();
            }
            KeyCode::Char('G') => {
                self.main_screen
                    .movies_list
                    .goto_index(app.movies.len(), app.movies.len() - 1);
            }
            KeyCode::Char('g') => {
                self.main_screen.movies_list.goto_index(app.movies.len(), 0);
            }
            KeyCode::Up => {
                self.main_screen.movies_list.dec_movie_selection();
            }
            KeyCode::Down => {
                self.main_screen
                    .movies_list
                    .inc_movie_selection(app.movies.len());
            }
            KeyCode::Esc => {
                self.close_popups();
            }
            KeyCode::Char('r') => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    self.image_backend.reload_images(
                        app,
                        self.main_screen.movies_list.scroll_pos,
                        Some(self.main_screen.movies_list.num_visible_movies),
                    );
                }
            }
            _ => (),
        }
    }

    pub fn open_main_screen(&mut self) {
        self.close_popups();

        self.current_screen = Screens::MainScreen;
    }

    pub fn render_main_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();

        let num_movies = ((frame_area.height - 4) as f32 / 8.0).floor() as usize;
        let footer_height = (((frame_area.height - 4) % 8) % num_movies as u16) + 1;

        let vert_lay = vertical![==3, >=1, ==footer_height].split(frame_area);
        let horiz_lay = horizontal![>=30, ==2/3].split(vert_lay[1]);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

        self.render_movies_list(frame, app, horiz_lay[1], num_movies)?;

        if !app.movies.is_empty() {
            self.draw_movie_description(app, frame, horiz_lay[0]);
        }

        Ok(())
    }
}
