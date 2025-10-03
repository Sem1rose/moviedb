use crate::{custom::helpers::center_rect, draw::Drawer, types::Result};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, vertical};
use style::palette::tailwind;

impl Drawer {
    pub fn error_popup_handle_key_events(&mut self, event: KeyEvent) -> bool {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return false;
        }

        match code {
            KeyCode::Enter => {
                return true;
            }
            KeyCode::Esc => {
                return true;
            }
            _ => (),
        }

        false
    }

    pub(crate) fn draw_error_popup(&mut self, frame: &mut Frame) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(30), Constraint::Max(8));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Error encountered")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        let v_areas = Layout::vertical([Constraint::Length(1); 5]).split(horiz);
        let h_areas = horizontal![>=1, ==6].split(v_areas[4]);

        frame.render_widget(
            Paragraph::new(self.error_popup_error.clone())
                .red()
                .centered(),
            v_areas[2],
        );
        frame.render_widget(Paragraph::new(" Ok ").centered().on_red(), h_areas[1]);

        Ok(())
    }
}
