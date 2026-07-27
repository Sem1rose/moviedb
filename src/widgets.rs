use itertools::Itertools;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{HorizontalAlignment, Offset, Rect, Size},
    macros::{line, span, vertical},
    style::{
        Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Clear, Padding, Widget},
};
use ratatui_textarea::{TextArea, WrapMode};

use crate::helpers::add_padding;

pub fn input_field(
    tab_selected: bool,
    selected: bool,
    valid: bool,
    input: &mut TextArea<'static>,
    wrap_mode: WrapMode,
    frame: &mut Frame,
    area: Rect,
    title: &'static str,
    placeholder_text: &str,
) {
    input.set_style(Style::new().fg(if tab_selected {
        if selected {
            tailwind::SLATE.c200
        } else {
            tailwind::STONE.c400
        }
    } else {
        tailwind::STONE.c400
    }));
    input.set_cursor_style(
        Style::new()
            .fg(if tab_selected && selected {
                tailwind::SLATE.c300
            } else {
                tailwind::STONE.c400
            })
            .add_modifier(if tab_selected && selected {
                Modifier::REVERSED
            } else {
                Modifier::default()
            }),
    );
    input.set_block(
        Block::bordered()
            .border_type(ratatui::widgets::BorderType::Thick)
            .fg(if tab_selected {
                if selected {
                    if valid { material::BLUE.c500 } else { material::RED.c600 }
                } else {
                    tailwind::STONE.c500
                }
            } else {
                tailwind::STONE.c600
            })
            .title(title)
            .title_style(Style::new().fg(if tab_selected {
                if selected {
                    material::BLUE.c400
                } else {
                    if valid { material::BLUE.c600 } else { material::RED.c600 }
                }
            } else {
                tailwind::STONE.c400
            }))
            .padding(Padding::symmetric(1, 0)),
    );
    input.set_placeholder_text(placeholder_text);
    input.set_placeholder_style(Style::new().fg(material::GRAY.c700));
    input.set_wrap_mode(wrap_mode);

    frame.render_widget(&*input, area);
}

pub fn dropdown(tab_selected: bool, selected: bool, frame: &mut Frame, area: Rect, text: &str) {
    // "▼⬇⬆⏷"
    let sort_block = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .fg(if tab_selected {
            if selected {
                material::BLUE.c600
            } else {
                material::INDIGO.c800
            }
        } else {
            tailwind::SLATE.c700
        });
    frame.render_widget(&sort_block, area);
    frame.render_widget(
        span!(text)
            .bold()
            .fg(if tab_selected {
                if selected {
                    material::TEAL.c100
                } else {
                    material::INDIGO.c200
                }
            } else {
                material::GRAY.c400
            })
            .bg(if tab_selected {
                if selected {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            }),
        sort_block.inner(area),
    );
    frame.render_widget(
        line!(" ▼")
            .right_aligned()
            .bold()
            .fg(if tab_selected {
                if selected {
                    material::TEAL.c100
                } else {
                    material::INDIGO.c200
                }
            } else {
                material::GRAY.c400
            })
            .bg(if tab_selected {
                if selected {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            }),
        sort_block.inner(area),
    );
}

pub fn dropdown_popup(
    items: Vec<Line>,
    selected_index: usize,
    scroll_pos: usize,
    num_visible_items: usize,
    dropdown_widget_area: Rect,
    frame: &mut Frame,
) -> (Rect, usize) {
    let mut items = items
        .into_iter()
        .dropping(scroll_pos)
        .take(num_visible_items)
        .collect_vec();
    let items_len = items.len();

    let selected = items
        .remove(selected_index - scroll_pos)
        .fg(material::BLUE.c100)
        .bg(material::LIGHT_BLUE.c900);
    items.insert(selected_index - scroll_pos, selected);

    let dropdown_popup_area = dropdown_widget_area
        .offset(Offset::new(0, 2))
        .resize(Size::new(
            dropdown_widget_area.width,
            dropdown_widget_area.height + items.len() as u16 - 1,
        ));
    frame.render_widget(
        Clear,
        add_padding(dropdown_popup_area, Padding::vertical(1)),
    );

    let sort_popup_block = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .fg(tailwind::INDIGO.c900);
    let inner_area = sort_popup_block.inner(dropdown_popup_area);
    frame.render_widget(&sort_popup_block, dropdown_popup_area);
    frame.render_widget(
        Block::new().bg(material::BLUE.c600),
        dropdown_popup_area.resize(Size::new(dropdown_popup_area.width, 1)),
    );
    frame.render_widget(Block::new().bg(material::INDIGO.c900), inner_area);
    frame.render_widget(Text::from_iter(items).left_aligned(), inner_area);

    (
        inner_area.resize(Size {
            width:  inner_area.width,
            height: 1,
        }),
        items_len,
    )
}

pub struct Action {
    action:      &'static str,
    action_type: ActionTypes,
    selected:    bool,
    valid:       bool,
}
impl Action {
    pub fn new(
        action: &'static str,
        action_type: ActionTypes,
        selected: bool,
        valid: bool,
    ) -> Self {
        Self {
            action,
            action_type,
            selected,
            valid,
        }
    }
}
impl<'a> Into<Span<'a>> for Action {
    fn into(self) -> Span<'a> {
        span!(self.action)
            .fg(if self.valid {
                if self.selected {
                    tailwind::SLATE.c300
                } else {
                    match self.action_type {
                        ActionTypes::Default => tailwind::SLATE.c300,
                        ActionTypes::Normal => material::BLUE.c500,
                        ActionTypes::Critical => tailwind::RED.c500,
                    }
                }
            } else {
                tailwind::SLATE.c500
            })
            .bg(if self.valid {
                if self.selected {
                    match self.action_type {
                        ActionTypes::Default => material::BLUE.c600,
                        ActionTypes::Normal => material::BLUE.c800,
                        ActionTypes::Critical => tailwind::RED.c800,
                    }
                } else {
                    if matches!(self.action_type, ActionTypes::Default) {
                        material::BLUE.c900
                    } else {
                        tailwind::SLATE.c950
                    }
                }
            } else {
                if self.selected {
                    tailwind::SLATE.c700
                } else {
                    tailwind::SLATE.c800
                }
            })
    }
}

pub enum ActionTypes {
    Default,
    Normal,
    Critical,
}

pub fn action(
    action: Action,
    alignment: HorizontalAlignment,
    bottom: bool,
    area: Rect,
    frame: &mut Frame,
) -> Rect {
    let span: Span<'_> = action.into();

    let area = if bottom {
        vertical![>=1, ==1].split(area)[1]
    } else {
        area
    };

    let mouse_area = match alignment {
        HorizontalAlignment::Left => area,
        HorizontalAlignment::Center => area.offset(Offset::new(
            (area.width as i32 - span.width() as i32) / 2,
            0,
        )),
        HorizontalAlignment::Right =>
            area.offset(Offset::new(area.width as i32 - span.width() as i32, 0)),
    }
    .resize(Size::new(span.width() as u16, 1));
    let line = match alignment {
        HorizontalAlignment::Left => line!(span),
        HorizontalAlignment::Center => line!(span).centered(),
        HorizontalAlignment::Right => line!(span).right_aligned(),
    };

    frame.render_widget(line, area);

    mouse_area
}

pub fn actions<const N: usize>(
    actions: [Action; N],
    alignment: HorizontalAlignment,
    bottom: bool,
    spacing: u16,
    area: Rect,
    frame: &mut Frame,
) -> [Rect; N] {
    let spans: Vec<Span<'_>> = actions.into_iter().map(|x| x.into()).collect_vec();
    let actions_count = spans.len();
    let actions_width =
        spans.iter().fold(0, |a, x| a + x.width()) + spacing as usize * (actions_count - 1);

    let area = if bottom {
        vertical![>=1, ==1].split(area)[1]
    } else {
        area
    };

    let mut mouse_areas = [Rect::default(); N];
    let mut mouse_area = match alignment {
        HorizontalAlignment::Left => area,
        HorizontalAlignment::Center => area.offset(Offset::new(
            (area.width as i32 - actions_width as i32) / 2,
            0,
        )),
        HorizontalAlignment::Right =>
            area.offset(Offset::new(area.width as i32 - actions_width as i32, 0)),
    };
    for (i, span) in spans.iter().enumerate() {
        mouse_area = mouse_area.resize(Size::new(span.width() as u16, 1));
        mouse_areas[i] = mouse_area;

        mouse_area = mouse_area.offset(Offset::new(span.width() as i32 + spacing as i32, 0));
    }

    let mut line = Line::from_iter(
        spans
            .into_iter()
            .flat_map(|x| [x, span!(" ".repeat(spacing as usize))])
            .take(actions_count * 2 - 1),
    );
    line = match alignment {
        HorizontalAlignment::Left => line,
        HorizontalAlignment::Center => line.centered(),
        HorizontalAlignment::Right => line.right_aligned(),
    };

    frame.render_widget(line, area);

    mouse_areas
}

pub fn hyperlink<'content>(
    text: impl Into<Text<'content>>,
    url: &str,
    area: Rect,
    frame: &mut Frame,
) {
    frame.render_widget(&Hyperlink::new(text.into(), url), area);
}

struct Hyperlink<'content> {
    text: Text<'content>,
    url:  String,
}

impl<'content> Hyperlink<'content> {
    fn new(text: impl Into<Text<'content>>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url:  url.into(),
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
            for (i, two_chars) in line.to_string().chars().chunks(2).into_iter().enumerate() {
                let text = two_chars.collect::<String>();
                let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
                buffer[(area.x + i as u16 * 2, area.y + j as u16)].set_symbol(hyperlink.as_str());
            }
        }
    }
}
