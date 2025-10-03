use crate::{app::App, custom::helpers::center_rect, draw::Drawer, types::*};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, vertical};
use style::palette::tailwind;

#[derive(Default)]
enum Phase {
    #[default]
    Confirm,
    Done,
}

#[derive(Default)]
pub struct RemoveMoviePopup {
    pub selected: usize,

    phase: Phase,
}

impl RemoveMoviePopup {
    pub const NUMBUTTONS: usize = 2;
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    fn dec_selection_horiz(&mut self) {
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(RemoveMoviePopup::NUMBUTTONS - 1);
    }

    fn inc_selection_horiz(&mut self) {
        self.selected += 1;
        if self.selected >= RemoveMoviePopup::NUMBUTTONS {
            self.selected = 0;
        }
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Confirm => Phase::Done,
            _ => Phase::Confirm,
        };
    }

    pub fn handle_key_events(&mut self, event: KeyEvent) -> bool {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return false;
        }

        match code {
            KeyCode::Right => {
                self.inc_selection_horiz();
            }
            KeyCode::Left => {
                self.dec_selection_horiz();
            }
            KeyCode::Enter => {
                if self.selected == 0 {
                    return true;
                } else if self.selected == 1 {
                    self.advance_phase();
                }
            }
            KeyCode::Esc => {
                return true;
            }
            _ => (),
        }

        false
    }
}

impl Drawer {
    pub(crate) fn draw_remove_movie_popup(
        &mut self,
        frame: &mut Frame,
        app: &mut App,
    ) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(40), Constraint::Max(7));

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

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match self.remove_movie_popup.phase {
            Phase::Confirm => {
                let areas = vertical![==1, >=1, ==1].split(horiz);

                frame.render_widget(
                    Paragraph::new(format!(
                        "Do you really want to remove {}?",
                        app.movies[self.main_screen.movies_list.current_movie_index()].name
                    ))
                    .wrap(Wrap { trim: false }),
                    areas[1],
                );

                let button_areas = horizontal![>=1, >=1, >=1].split(areas[2]);
                frame.render_widget(
                    Paragraph::new(format!(
                        "{}Cancel{}",
                        if self.remove_movie_popup.selected == 0 {
                            ">"
                        } else {
                            " "
                        },
                        if self.remove_movie_popup.selected == 0 {
                            "<"
                        } else {
                            " "
                        },
                    ))
                    .centered()
                    .black()
                    .on_green(),
                    button_areas[2],
                );
                frame.render_widget(
                    Paragraph::new(format!(
                        "{}Yes{}",
                        if self.remove_movie_popup.selected == 1 {
                            ">"
                        } else {
                            " "
                        },
                        if self.remove_movie_popup.selected == 1 {
                            "<"
                        } else {
                            " "
                        },
                    ))
                    .centered()
                    .on_red(),
                    button_areas[0],
                );
            }
            Phase::Done => {
                self.image_backend
                    .remove_cached_image(self.main_screen.movies_list.current_movie_index());

                app.movies
                    .remove(self.main_screen.movies_list.current_movie_index());

                if app.save_movies().is_err() {
                    self.open_error_popup("Couldn't remove movie!".into());

                    return Ok(());
                }

                if self.main_screen.movies_list.current_movie_index() >= app.movies.len() {
                    if self.main_screen.movies_list.scroll_pos > 0 {
                        self.main_screen.movies_list.scroll_pos -= 1;
                    } else if self.main_screen.movies_list.selected > 0 {
                        self.main_screen.movies_list.selected -= 1;
                    }
                    // } else {
                    //     self.image_backend.reload_images(
                    //         app,
                    //         self.main_screen.movies_list.current_movie_index(),
                    //         Some(self.main_screen.movies_list.num_visible_movies),
                    //     );
                }

                self.close_popups();
            }
        }
        Ok(())
    }
}
