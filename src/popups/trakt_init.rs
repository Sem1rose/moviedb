use crate::{
    app::{App, Result},
    draw::Drawer,
    helpers::{center_rect, v_center},
};
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
enum Phase {
    #[default]
    Initializing,
    GetSecrets,
    Get,
}

#[derive(Default)]
pub struct TraktInitPopup {
    phase: Phase,
}

impl TraktInitPopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }
}

impl Drawer {
    pub fn trakt_init_popup_handle_key_events(&mut self, event: KeyEvent) -> Result<()> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(());
        }

        match code {
            KeyCode::Enter => {}
            KeyCode::Esc => {
                self.close_popups();
            }
            _ => {}
        }

        Ok(())
    }

    pub(crate) fn draw_trakt_init_popup(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(40), Constraint::Max(10));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Enter Trakt Credentials")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match self.trakt_init_popup_options.phase {
            Phase::Initializing => {
                let [_, throbber_area, _, text_area, _] = Layout::horizontal([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .areas(horiz);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(
                    throbber,
                    v_center(throbber_area),
                    &mut self.throbber_state,
                );
                frame.render_widget(
                    Paragraph::new("Initializing Trakt Config..."),
                    v_center(text_area),
                );
            }
            _ => (),
        }

        Ok(())
    }
}
