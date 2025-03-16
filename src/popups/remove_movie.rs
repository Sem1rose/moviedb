use crate::{
    app::{App, Errors, Result},
    draw::Drawer,
};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use style::palette::tailwind;

#[derive(Default)]
pub struct RemoveMoviePopup {
    pub errored: bool,
    pub confirmed: bool,
    pub selected: i32,
    pub finished: bool,
}

impl RemoveMoviePopup {
    pub const BUTTONS: i32 = 2;
    pub fn begin(&mut self) {
        *self = Self::default();
    }
}

impl Drawer {
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
                    app.movies
                        [(self.main_screen_options.scroll_pos + self.main_screen_options.selected)]
                        .name
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
                .remove(self.main_screen_options.scroll_pos + self.main_screen_options.selected);

            if app.save_movies().is_err() {
                self.remove_movie_popup_options.errored = true;
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Couldn't remove movie!").red().centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            }

            self.main_screen_options.clear_all_image();
            if self.main_screen_options.current_movie_index() >= app.movies.len() {
                if self.main_screen_options.scroll_pos > 0 {
                    self.main_screen_options.scroll_pos -= 1;
                } else if self.main_screen_options.selected > 0 {
                    self.main_screen_options.selected -= 1;
                }
            }

            // if self.main_screen_options.selected == self.main_screen_options.num_visible_movies - 1
            // {
            //     self.main_screen_options.dec_movie_selection();
            // }
            // } else if
            // self.main_screen_options.clear_all_image();

            self.close_popups();
            // self.fetch_artwork_popup_options.start_thread(&app);
            // self.open_fetch_artworks_popup(app);

            // if self.draw_fetch_artworks_popup(frame, app)? {
            //     if app.save_movies().is_err() {
            //         self.remove_movie_popup_options.errored = true;
            //         let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
            //         frame.render_widget(
            //             Paragraph::new("Couldn't save new rating!").red().centered(),
            //             areas[2],
            //         );
            //         frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            //     } else {
            //         // self.clear_images(false);
            //     }
            // }
        }
        Ok(())
    }
}
