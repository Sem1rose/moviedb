use crate::{
    app::App,
    custom::{
        helpers::{center_rect, v_center},
        hyperlink::Hyperlink,
    },
    draw::Drawer,
};
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::*,
    prelude::*,
    widgets::*,
    Frame,
};
use ratatui_macros::{horizontal, span, vertical};
use style::palette::tailwind;
use tui_input::{backend::crossterm::EventHandler, Input};

#[derive(Default)]
pub enum Phase {
    #[default]
    Initializing,
    GetAuthorization(String),
    GotAuthorization,
    RefreshingTokens,
    Error(anyhow::Error),
}

#[derive(Default)]
pub struct TraktInitPopup {
    pub phase: Phase,

    pub auth_code_input: Input,
}

impl TraktInitPopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::GetAuthorization(_) => Phase::GotAuthorization,
            _ => Phase::Initializing,
        };
    }

    pub fn get_authorization(&mut self, authorization_url: String) {
        self.auth_code_input.reset();
        self.phase = Phase::GetAuthorization(authorization_url);
    }

    pub fn handle_key_events(&mut self, event: KeyEvent, app: &mut App) -> anyhow::Result<bool> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(false);
        }

        match code {
            KeyCode::Enter => {
                if let Phase::GetAuthorization(_) = self.phase {
                    if !self.auth_code_input.value().is_empty() {
                        self.advance_phase();
                    }
                } else if let Phase::Error(_) = self.phase {
                    app.trakt_config.init(&app.config);
                    self.advance_phase();
                }
            }
            KeyCode::Esc => {
                if let Phase::GetAuthorization(_) = self.phase {
                    return Ok(true);
                }
            }
            _ => {
                if let Phase::GetAuthorization(_) = self.phase {
                    self.auth_code_input.handle_event(&Event::Key(event));
                }
            }
        }

        Ok(false)
    }
}

impl Drawer {
    pub(crate) fn draw_trakt_init_popup(&mut self, frame: &mut Frame) -> anyhow::Result<()> {
        let frame_area = frame.area();
        let popup_area = center_rect(frame_area, Constraint::Percentage(40), Constraint::Max(10));

        let popup = Block::new()
            .bg(tailwind::INDIGO.c950)
            .fg(tailwind::INDIGO.c300)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(tailwind::EMERALD.c400))
            .title_top("Trakt Authorization")
            .title_alignment(Alignment::Center)
            .title_style(Style::new().fg(tailwind::AMBER.c300));

        frame.render_widget(Clear, popup_area);
        frame.render_widget(&popup, popup_area);

        let [_, vert, _] = vertical![==1, >=1, ==1].areas(popup_area);
        let [_, horiz, _] = horizontal![==2, >=1, ==2].areas(vert);

        match &self.trakt_init_popup.phase {
            Phase::Initializing => {
                let [_, throbber_area, _, text_area, _] =
                    horizontal![==2, ==1, ==2, >=1, ==2].areas(horiz);

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
            Phase::GetAuthorization(url) => {
                let [_, center, _] = horizontal![==2, >=1, ==2].areas(horiz);
                let [_, top, bottom, _] = vertical![>=1, ==1, ==3, >=1].areas(center);

                frame.render_widget(
                    &Hyperlink::new(
                        span!("Click here to get the authentication code.")
                            .bold()
                            .underlined()
                            .blue(),
                        url,
                    ),
                    top,
                );

                let [left, right] = horizontal![>=1, >=1].areas(bottom);

                let [_, label, _] = vertical![>=0, ==1, >=0].areas(left);

                let [_, input_top, input_center, input_bottom, _] =
                    vertical![>=0, ==1, ==1, ==1,>=0].areas(right);

                let [_, input_area, _] = horizontal![==1, >=1, ==1].areas(input_center);

                // ▄▀█ ▂🮂▗▖▘▝
                frame.render_widget(
                    Paragraph::new("🮃".repeat(input_bottom.width as usize)).fg(tailwind::RED.c700),
                    input_bottom,
                );
                frame.render_widget(
                    Paragraph::new("▂".repeat(input_top.width as usize)).fg(tailwind::RED.c700),
                    input_top,
                );
                frame.render_widget(Paragraph::new("Enter authentication code: "), label);
                frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

                let width = input_area.width as usize - 1;
                let start = self.trakt_init_popup.auth_code_input.visual_scroll(width);
                let cursor_pos = self.trakt_init_popup.auth_code_input.cursor() - start;
                let mut chars = self
                    .trakt_init_popup
                    .auth_code_input
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
            }
            Phase::GotAuthorization => {
                let [_, throbber_area, _, text_area, _] =
                    horizontal![==2, ==1, ==2, >=1, ==2].areas(horiz);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(
                    throbber,
                    v_center(throbber_area),
                    &mut self.throbber_state,
                );
                frame.render_widget(Paragraph::new("Processing..."), v_center(text_area));
            }
            Phase::RefreshingTokens => {
                let [_, throbber_area, _, text_area, _] =
                    horizontal![==2, ==1, ==2, >=1, ==2].areas(horiz);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(
                    throbber,
                    v_center(throbber_area),
                    &mut self.throbber_state,
                );
                frame.render_widget(
                    Paragraph::new("Refreshing Trakt tokens..."),
                    v_center(text_area),
                );
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
