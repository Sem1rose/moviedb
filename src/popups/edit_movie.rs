use crate::{custom::helpers::center_rect, draw::Drawer, types::*};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, text, vertical};
use style::palette::tailwind;
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Default)]
pub enum Phase {
    #[default]
    GetNewRating,
    Done,
}

#[derive(Default)]
pub struct EditMoviePopup {
    pub user_rating_input: Input,

    pub user_rating: f64,

    pub phase: Phase,
}

impl EditMoviePopup {
    pub fn begin(&mut self, user_rating: f64) {
        *self = Self::default();

        self.user_rating_input = user_rating.to_string().into();
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::GetNewRating => Phase::Done,
            _ => Phase::GetNewRating,
        };
    }

    pub fn check_input_rating(&mut self) -> bool {
        if self.user_rating_input.value().is_empty() {
            return false;
        }

        if let Ok(x) = self.user_rating_input.value().parse::<f64>() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn handle_key_events(&mut self, event: KeyEvent) -> bool {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return false;
        }

        match code {
            KeyCode::Enter => {
                if let Phase::GetNewRating = self.phase {
                    if self.check_input_rating() {
                        self.advance_phase();
                    }
                }
            }
            KeyCode::Esc => {
                return true;
            }
            _ => {
                if let Phase::GetNewRating = self.phase {
                    self.user_rating_input.handle_event(&Event::Key(event));
                }
            }
        }

        false
    }
}

impl Drawer {
    pub(crate) fn draw_edit_movie_popup(&mut self, frame: &mut Frame) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(35), Constraint::Max(10));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Edit Movie")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match self.edit_movie_popup.phase {
            Phase::GetNewRating => {
                let [_, right, left, _] = horizontal![==2, ==12, >=1, ==2].areas(horiz);
                let prompt_area = Layout::vertical([Constraint::Length(1); 6]).split(right)[2];
                let [_, input_top, input_center, input_bottom, _, _] =
                    Layout::vertical([Constraint::Length(1); 6]).areas(left);
                let [_, input_area, _] = horizontal![==1, >=1, ==1].areas(input_center);

                // ▄▀█ ▂🮂▗▖▘▝
                frame.render_widget(
                    text!["🮂".repeat(input_bottom.width as usize)].fg(tailwind::RED.c700),
                    input_bottom,
                );
                frame.render_widget(
                    text!["▂".repeat(input_top.width as usize)].fg(tailwind::RED.c700),
                    input_top,
                );

                frame.render_widget(text!["New rating: "], prompt_area);

                frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

                let areas = Layout::vertical([Constraint::Length(1); 6]).split(horiz);
                let [_, button_area] = horizontal![>=1, ==4].areas(areas[5]);

                frame.render_widget(text![" Ok "].on_red().right_aligned(), button_area);

                let width = input_area.width as usize - 1;
                let start = self.edit_movie_popup.user_rating_input.visual_scroll(width);
                let cursor_pos = self.edit_movie_popup.user_rating_input.cursor() - start;
                let mut chars = self
                    .edit_movie_popup
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

                if !self.edit_movie_popup.check_input_rating() {
                    frame.render_widget(
                        Paragraph::new("Please enter a valid rating!")
                            .red()
                            .centered(),
                        areas[4],
                    );
                }
            }
            _ => {
                let areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
                let [_, throbber_area, text_area, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(areas[2]);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state);
                frame.render_widget(Paragraph::new(" Processing..."), text_area);
            }
        }

        Ok(())
    }
}
