use crate::{custom::helpers::center_rect, draw::Drawer};
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
pub enum Phase {
    #[default]
    Confirm,
    Done,
}

#[derive(Default)]
pub struct RemoveMoviePopup {
    pub selected: usize,

    pub phase: Phase,
}

const NUMBUTTONS: usize = 2;
impl RemoveMoviePopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    // damn, past me was so smart lol
    fn dec_selection(&mut self) {
        self.selected = self.selected.checked_sub(1).unwrap_or(NUMBUTTONS - 1);
    }

    fn inc_selection(&mut self) {
        self.selected += 1;
        if self.selected >= NUMBUTTONS {
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
                self.inc_selection();
            }
            KeyCode::Left => {
                self.dec_selection();
            }
            KeyCode::Enter => {
                if self.selected == 0 {
                    return true;
                } else {
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
    pub(crate) fn draw_remove_movie_popup(&mut self, frame: &mut Frame) -> anyhow::Result<()> {
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
                        self.main_screen.filtered_movies
                            [self.main_screen.movies_list.current_movie_index()]
                        .name
                    ))
                    .wrap(Wrap { trim: false }),
                    areas[1],
                );

                let button_areas = horizontal![>=1, >=1, >=1].split(areas[2]);
                frame.render_widget(
                    Paragraph::new(if self.remove_movie_popup.selected == 0 {
                        ">Cancel<"
                    } else {
                        " Cancel "
                    })
                    .centered()
                    .black()
                    .on_green(),
                    button_areas[2],
                );
                frame.render_widget(
                    Paragraph::new(if self.remove_movie_popup.selected == 1 {
                        ">Yes<"
                    } else {
                        " Yes "
                    })
                    .centered()
                    .on_red(),
                    button_areas[0],
                );
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
