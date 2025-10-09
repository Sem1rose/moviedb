use crate::{
    app::App,
    custom::{
        helpers::{center_rect, v_center},
        hyperlink::Hyperlink,
    },
    draw::Drawer,
    types::*,
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, span, vertical};
use style::palette::tailwind;

#[derive(Default)]
pub enum Phase {
    #[default]
    Initializing,
    GetAuthorization(String),
    Done,
    Error(Errors),
}

#[derive(Default)]
pub struct TMDBInitPopup {
    pub phase: Phase,
}

impl TMDBInitPopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::GetAuthorization(_) => Phase::Done,
            _ => Phase::Initializing,
        };
    }

    pub fn get_authorization(&mut self, authorization_url: String) {
        self.phase = Phase::GetAuthorization(authorization_url);
    }

    pub fn handle_key_events(&mut self, event: KeyEvent, app: &mut App) -> Result<bool> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(false);
        }

        match code {
            KeyCode::Enter => {
                if let Phase::Error(_) = self.phase {
                    app.tmdb_config.init(&app.config);
                    self.advance_phase();
                }
            }
            KeyCode::Esc => {
                if let Phase::GetAuthorization(_) = self.phase {
                    return Ok(true);
                }
            }
            _ => {}
        }

        Ok(false)
    }
}

impl Drawer {
    pub(crate) fn draw_tmdb_init_popup(&mut self, frame: &mut Frame) -> Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(40), Constraint::Max(10));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Enter TMDB Credentias")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match &self.tmdb_init_popup.phase {
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
                    Paragraph::new("Initializing TMDB Config..."),
                    v_center(text_area),
                );
            }
            Phase::GetAuthorization(url) => {
                let [_, prompt_area, _] = vertical![>=1,==1,>=1].areas(horiz);

                frame.render_widget(
                    &Hyperlink::new(
                        span!("Click here to go to the authorization url.")
                            .bold()
                            .underlined()
                            .blue(),
                        url,
                    ),
                    v_center(prompt_area),
                );
            }
            Phase::Done => {
                let [_, prompt_area, _] = vertical![>=1,==1,>=1].areas(horiz);

                frame.render_widget(span!("TMDB Initialization done!"), v_center(prompt_area));
            }
            Phase::Error(errors) => {
                let frame_area = frame.area();
                let popup_area =
                    center_rect(frame_area, Constraint::Percentage(30), Constraint::Max(8));

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
                    Paragraph::new(errors.to_string())
                        .red()
                        .centered()
                        .wrap(Wrap { trim: true }),
                    v_areas[2],
                );
                frame.render_widget(Paragraph::new(" Ok ").centered().on_red(), h_areas[1]);
            }
        }

        Ok(())
    }
}
