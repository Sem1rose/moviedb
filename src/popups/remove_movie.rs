use crate::{
    app::{App, Result},
    draw::Drawer,
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use style::palette::tailwind;

#[derive(Default)]
pub struct RemoveMoviePopup {
    pub errored: bool,
    pub confirmed: bool,
    pub selected: i32,
    pub finished: bool,
}

impl RemoveMoviePopup {
    pub const NUMBUTTONS: i32 = 2;
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    fn dec_selection_horiz(&mut self) {
        self.selected -= 1;
        if self.selected < 0 {
            self.selected = RemoveMoviePopup::NUMBUTTONS - 1;
        }
    }

    fn inc_selection_horiz(&mut self) {
        self.selected += 1;
        if self.selected >= RemoveMoviePopup::NUMBUTTONS {
            self.selected = 0;
        }
    }
}

impl Drawer {
    pub fn remove_movie_popup_handle_key_events(&mut self, event: KeyEvent) -> Result<()> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(());
        }

        match code {
            KeyCode::Right => {
                self.remove_movie_popup_options.inc_selection_horiz();
            }
            KeyCode::Left => {
                self.remove_movie_popup_options.dec_selection_horiz();
            }
            KeyCode::Enter => {
                if self.remove_movie_popup_options.errored
                    || self.remove_movie_popup_options.selected == 0
                {
                    self.close_popups();
                } else if self.remove_movie_popup_options.selected == 1 {
                    self.remove_movie_popup_options.confirmed = true;
                }
            }
            KeyCode::Esc => {
                self.close_popups();
            }
            _ => (),
        }

        Ok(())
    }

    pub(crate) fn draw_remove_movie_popup(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
    ) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = self.center(frame_area, Constraint::Percentage(40), Constraint::Max(7));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Remove Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(popup_area);
        let [_, horiz, _] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .areas(vert);

        if !self.remove_movie_popup_options.confirmed {
            let areas = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(horiz);

            frame.render_widget(
                Paragraph::new(format!(
                    "Do you really want to remove {}?",
                    app.movies[self.main_screen_options.current_movie_index()].name
                ))
                .wrap(Wrap { trim: false }),
                areas[1],
            );

            let button_areas =
                Layout::horizontal([Constraint::Min(1), Constraint::Min(1), Constraint::Min(1)])
                    .split(areas[2]);
            frame.render_widget(
                Paragraph::new(format!(
                    "{}Cancel{}",
                    if self.remove_movie_popup_options.selected == 0 {
                        ">"
                    } else {
                        " "
                    },
                    if self.remove_movie_popup_options.selected == 0 {
                        "<"
                    } else {
                        " "
                    },
                ))
                .centered()
                .on_green(),
                button_areas[2],
            );
            frame.render_widget(
                Paragraph::new(format!(
                    "{}Yes{}",
                    if self.remove_movie_popup_options.selected == 1 {
                        ">"
                    } else {
                        " "
                    },
                    if self.remove_movie_popup_options.selected == 1 {
                        "<"
                    } else {
                        " "
                    },
                ))
                .centered()
                .on_red(),
                button_areas[0],
            );
        } else if !self.remove_movie_popup_options.finished {
            self.remove_movie_popup_options.finished = true;
            app.movies
                .remove(self.main_screen_options.current_movie_index());

            if app.save_movies().is_err() {
                self.remove_movie_popup_options.errored = true;
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Couldn't remove movie!").red().centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            }

            if self.main_screen_options.current_movie_index() >= app.movies.len() {
                if self.main_screen_options.scroll_pos > 0 {
                    self.main_screen_options.scroll_pos -= 1;
                } else if self.main_screen_options.selected > 0 {
                    self.main_screen_options.selected -= 1;
                }
            } else {
                self.main_screen_options
                    .rehash_images(app, self.main_screen_options.current_movie_index());
            }

            self.close_popups();
        }
        Ok(())
    }
}
