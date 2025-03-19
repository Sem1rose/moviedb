use crate::{
    app::{App, Result},
    draw::Drawer,
};
use ratatui::{layout::*, prelude::*, widgets::*, Frame};
use style::palette::tailwind;
use tui_input::Input;

#[derive(Default)]
pub struct EditMoviePopup {
    pub init_ed: bool,
    pub errored: bool,
    pub user_rating_input: Input,
    pub user_rating_valid: bool,
    pub got_user_rating: bool,
    pub user_rating: f64,
}

impl EditMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }
}

impl Drawer {
    pub(crate) fn draw_edit_movie_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = self.center(frame_area, Constraint::Percentage(40), Constraint::Max(8));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Edit Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        // frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);
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

        if !self.edit_movie_popup_options.init_ed {
            self.edit_movie_popup_options.init_ed = true;
            self.edit_movie_popup_options.user_rating_input = app.movies
                [self.main_screen_options.scroll_pos + self.main_screen_options.selected]
                .user_rating
                .to_string()
                .into();
        }
        if !self.edit_movie_popup_options.got_user_rating {
            let [_, right, left, _] = Layout::horizontal([
                Constraint::Length(2),
                Constraint::Length(12),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .areas(horiz);
            let prompt_area = Layout::vertical([Constraint::Length(1); 6]).split(right)[2];
            let [_, input_top, input_center, input_bottom, _, _] =
                Layout::vertical([Constraint::Length(1); 6]).areas(left);
            let [_, input_area, _] = Layout::horizontal([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(input_center);

            // ▄▀█ ▂🮂▗▖▘▝
            frame.render_widget(
                Paragraph::new("🮂".repeat(input_bottom.width as usize)).fg(tailwind::RED.c700),
                input_bottom,
            );
            frame.render_widget(
                Paragraph::new("▂".repeat(input_top.width as usize)).fg(tailwind::RED.c700),
                input_top,
            );
            frame.render_widget(Paragraph::new("New rating: "), prompt_area);
            frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

            let areas = Layout::vertical([Constraint::Length(1); 6]).split(horiz);
            let [_, button_area] =
                Layout::horizontal([Constraint::Min(1), Constraint::Length(4)]).areas(areas[5]);
            frame.render_widget(Paragraph::new(" Ok ").on_red().right_aligned(), button_area);

            let width = input_area.width as usize - 1;
            let start = self
                .edit_movie_popup_options
                .user_rating_input
                .visual_scroll(width);
            let cursor_pos = self.edit_movie_popup_options.user_rating_input.cursor() - start;
            let mut chars = self
                .edit_movie_popup_options
                .user_rating_input
                .value()
                .chars()
                .skip(start);

            let mut input_string: Vec<Span> = vec![];
            for i in 0..=(start + width) {
                let c = chars.next().unwrap_or(' ');
                if i == cursor_pos {
                    input_string.push(c.to_string().reversed());
                } else {
                    input_string.push(c.to_string().into());
                }
            }
            frame.render_widget(Line::from_iter(input_string), input_area);

            let input_parsed = self
                .edit_movie_popup_options
                .user_rating_input
                .value()
                .parse::<f64>();
            self.edit_movie_popup_options.user_rating_valid =
                input_parsed.is_ok() && input_parsed.unwrap() <= 10.0;

            if !self.edit_movie_popup_options.user_rating_valid {
                frame.render_widget(
                    Paragraph::new("Please enter a valid rating!")
                        .red()
                        .centered(),
                    areas[4],
                );
            }
        } else {
            self.edit_movie_popup_options.user_rating = format!(
                "{:.1}",
                self.edit_movie_popup_options
                    .user_rating_input
                    .value()
                    .parse::<f32>()
                    .unwrap()
            )
            .parse()
            .unwrap();
            app.movies[self.main_screen_options.scroll_pos + self.main_screen_options.selected]
                .user_rating = self.edit_movie_popup_options.user_rating;

            if app.save_movies().is_err() {
                self.edit_movie_popup_options.errored = true;
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                frame.render_widget(
                    Paragraph::new("Couldn't save new rating!").red().centered(),
                    areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").right_aligned().on_red(), areas[4]);
            } else {
                self.close_popups();
                // self.clear_images(false);
            }
        }

        Ok(())
    }
}
