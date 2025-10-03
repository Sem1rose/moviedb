use crate::{
    custom::{
        helpers::{center_rect, v_center},
        hyperlink::Hyperlink,
    },
    draw::Drawer,
    types::*,
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
    GetInput,
    GotInput,
    GetAuthorization(String),
    Done,
}

#[derive(Default)]
pub struct TMDBInitPopup {
    pub phase: Phase,

    pub access_token_input: Input,
}

impl TMDBInitPopup {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Initializing => {
                self.access_token_input.reset();
                Phase::GetInput
            }
            Phase::GetInput => Phase::GotInput,
            Phase::GetAuthorization(_) => Phase::Done,
            _ => Phase::Initializing,
        };
    }

    pub fn get_authorization(&mut self, authorization_url: String) {
        self.phase = Phase::GetAuthorization(authorization_url);
    }

    pub fn handle_key_events(&mut self, event: KeyEvent) -> Result<()> {
        let kind = event.kind;
        let code = event.code;

        if kind != KeyEventKind::Press {
            return Ok(());
        }

        match code {
            KeyCode::Enter => {
                if let Phase::GetInput = self.phase {
                    if !self.access_token_input.value().is_empty() {
                        self.advance_phase();
                    }
                }
            }
            _ => {
                if let Phase::GetInput = self.phase {
                    self.access_token_input.handle_event(&Event::Key(event));
                }
            }
        }

        Ok(())
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
            Phase::GetInput => {
                let [_, left, right, _] = horizontal![==2, >=1, >=1, ==2].areas(horiz);

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
                frame.render_widget(Paragraph::new("Enter access token: "), v_center(left));
                frame.render_widget(Block::new().bg(tailwind::RED.c700), input_center);

                let width = input_area.width as usize - 1;
                let start = self.tmdb_init_popup.access_token_input.visual_scroll(width);
                let cursor_pos = self.tmdb_init_popup.access_token_input.cursor() - start;
                let mut chars = self
                    .tmdb_init_popup
                    .access_token_input
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
            Phase::GotInput => {
                let [_, throbber_area, text_area, _] = horizontal![==2, >=1, >=1, ==2].areas(horiz);

                let throbber = throbber_widgets_tui::Throbber::default()
                    .throbber_set(throbber_widgets_tui::BRAILLE_SIX_DOUBLE)
                    .throbber_style(Style::new().bold().fg(tailwind::VIOLET.c400));

                frame.render_stateful_widget(
                    throbber,
                    v_center(throbber_area),
                    &mut self.throbber_state,
                );
                frame.render_widget(
                    Paragraph::new("Getting Authorization URL..."),
                    v_center(text_area),
                );
            }
            Phase::GetAuthorization(url) => {
                //'\e]8;;https://google.com\e\\ass\e]8;;\e\ '
                let [_, prompt_area, _] = vertical![>=1,==1,>=1].areas(horiz);

                frame.render_widget(
                    // line![
                    //     span!("Please follow "),
                    //     span!(r#"\x1B]8;;"#.to_owned() + url + r#"\x1B\\THIS\x1B]8;;\x1B\"#)
                    //         .bold()
                    //         .italic()
                    //         .underlined()
                    //         .blue(),
                    //     span!(" link to authorize the application.")
                    // ]
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
        }

        Ok(())
    }
}
