use crate::helpers::add_padding;
use itertools::Itertools;
use ratatui::{
    layout::*, macros::span, prelude::*, style::palette::material, widgets::*, Frame,
};
use ratatui_textarea::{TextArea, WrapMode};
use style::palette::tailwind;


pub fn input_field(selected: bool, valid: bool, input: &mut TextArea<'static>, wrap_mode: WrapMode, frame: &mut Frame, area: Rect, horiz_padding: (u16, u16), title: &'static str, placeholder_text: &'static str) {
    input
        .set_style(Style::new().fg(if selected {
            tailwind::SLATE.c200
        } else {
            tailwind::STONE.c400
        }));
    input.set_cursor_style(
        Style::new()
            .fg(if selected {
                tailwind::SLATE.c300
            } else {
                tailwind::STONE.c400
            })
            .add_modifier(if selected {
                Modifier::REVERSED
            } else {
                Modifier::default()
            }),
    );
    input.set_block(
        Block::bordered()
            .border_type(ratatui::widgets::BorderType::Thick)
            .style(Style::new().fg(if selected {
                if valid {
                    material::BLUE.c500
                } else {
                    material::RED.c600
                }
            } else {
                tailwind::STONE.c600
            }))
            .title(title)
            .title_style(Style::new().fg(if selected {
                material::BLUE.c400
            } else {
                if valid {
                    material::BLUE.c600
                } else {
                    material::RED.c600
                }
            }))
            .padding(Padding::symmetric(1, 0)),
    );
    input.set_placeholder_text(placeholder_text);
    input
        .set_placeholder_style(Style::new().fg(material::GRAY.c700));
    input.set_wrap_mode(wrap_mode);

    frame.render_widget(
        &*input,
        add_padding(area, Padding::new(horiz_padding.0, horiz_padding.1, 0, 0)),
    );
}

pub enum ActionTypes {
    Default,
    Normal,
    Critical,
}

pub fn action(action: &'static str, action_type: ActionTypes, selected: bool, valid: bool, alignment: HorizontalAlignment, area: Rect, frame: &mut Frame) -> Rect {
    let action = span!(action)
        .fg(if valid {
            if selected {
                tailwind::SLATE.c300
            } else {
                match action_type {
                    ActionTypes::Default => tailwind::SLATE.c300,
                    ActionTypes::Normal => material::BLUE.c500,
                    ActionTypes::Critical => tailwind::RED.c500,
                }
            }
        } else {
            tailwind::SLATE.c500
        })
        .bg(if valid {
            if selected {
                match action_type {
                    ActionTypes::Default => material::BLUE.c600,
                    ActionTypes::Normal => material::BLUE.c800,
                    ActionTypes::Critical => material::RED.c800,
                }
            } else {
                if matches!(action_type, ActionTypes::Default) {
                    material::BLUE.c900
                } else {
                    tailwind::SLATE.c950
                }
            }
        } else {
            if selected {
                tailwind::SLATE.c700
            } else {
                tailwind::SLATE.c800
            }
        });

    let mouse_area = match alignment {
        HorizontalAlignment::Left => {
            area
        },
        HorizontalAlignment::Center => {
            area.offset(Offset::new((area.width as i32 - action.width() as i32) / 2, 0))
        },
        HorizontalAlignment::Right => {
            area.offset(Offset::new(area.width as i32 - action.width() as i32, 0))
        },
    }.resize(Size::new(action.width() as u16, 1));
    let line = match alignment {
        HorizontalAlignment::Left => {
            Line::from(action)
        },
        HorizontalAlignment::Center => {
            Line::from(action).centered()
        },
        HorizontalAlignment::Right => {
            Line::from(action).right_aligned()
        },
    };

    frame.render_widget(line, area);

    mouse_area
}
pub fn actions<const N: usize>(actions: [&'static str; N], types: [ActionTypes; N], selected: [bool; N], valid: [bool; N], alignment: HorizontalAlignment, spacing: u16, area: Rect, frame: &mut Frame) -> [Rect; N] {
    let actions = actions.into_iter().enumerate().map(|(i, x)| {
        let valid = valid[i];
        let selected = selected[i];
        let action_type = &types[i];
        span!(x)
            .fg(if valid {
                if selected {
                    tailwind::SLATE.c300
                } else {
                    match action_type {
                        ActionTypes::Default => tailwind::SLATE.c300,
                        ActionTypes::Normal => material::BLUE.c500,
                        ActionTypes::Critical => tailwind::RED.c500,
                    }
                }
            } else {
                tailwind::SLATE.c500
            })
            .bg(if valid {
                if selected {
                    match action_type {
                        ActionTypes::Default => material::BLUE.c600,
                        ActionTypes::Normal => material::BLUE.c800,
                        ActionTypes::Critical => material::RED.c800,
                    }
                } else {
                    if matches!(action_type, ActionTypes::Default) {
                        material::BLUE.c900
                    } else {
                        tailwind::SLATE.c950
                    }
                }
            } else {
                if selected {
                    tailwind::SLATE.c700
                } else {
                    tailwind::SLATE.c800
                }
            })
    }).collect_vec();
    let actions_count = actions.len();
    let actions_width = actions.iter().fold(0, |a, x| a + x.width()) + spacing as usize * (actions_count - 1);

    let mut mouse_areas = [Rect::default(); N];
    let mut mouse_area = match alignment {
        HorizontalAlignment::Left => {
            area
        },
        HorizontalAlignment::Center => {
            area.offset(Offset::new((area.width as i32 - actions_width as i32) / 2, 0))
        },
        HorizontalAlignment::Right => {
            area.offset(Offset::new(area.width as i32 - actions_width as i32, 0))
        },
    };
    for (i, action) in actions.iter().enumerate() {
        mouse_area = mouse_area.resize(Size::new(action.width() as u16, 1));
        mouse_areas[i] = mouse_area;

        mouse_area = mouse_area.offset(Offset::new(action.width() as i32 + spacing as i32, 0));
    }

    let mut line = Line::from(actions.into_iter().flat_map(|x| [x, span!(" ".repeat(spacing as usize))]).take(actions_count * 2 - 1).collect_vec());
    line = match alignment {
        HorizontalAlignment::Left => line,
        HorizontalAlignment::Center => {
            line.centered()
        },
        HorizontalAlignment::Right => {
            line.right_aligned()
        },
    };

    frame.render_widget(line, area);

    mouse_areas
}

pub fn hyperlink<'content>(text: impl Into<Text<'content>>, url: &str, area: Rect, frame: &mut Frame) {
    frame.render_widget(&Hyperlink::new(text.into(), url), area);
}

struct Hyperlink<'content> {
    text: Text<'content>,
    url: String,
}

impl<'content> Hyperlink<'content> {
    fn new(text: impl Into<Text<'content>>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl Widget for &Hyperlink<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        (&self.text).render(area, buffer);

        // this is a hacky workaround for https://github.com/ratatui/ratatui/issues/902, a bug
        // in the terminal code that incorrectly calculates the width of ANSI escape sequences. It
        // works by rendering the hyperlink as a series of 2-character chunks, which is the
        // calculated width of the hyperlink text.
        for (j, line) in self.text.lines.clone().into_iter().enumerate() {
            for (i, two_chars) in line
                .to_string()
                .chars()
                .chunks(2)
                .into_iter()
                .enumerate()
            {
                let text = two_chars.collect::<String>();
                let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
                buffer[(area.x + i as u16 * 2, area.y + j as u16)].set_symbol(hyperlink.as_str());
            }
        }
    }
}
