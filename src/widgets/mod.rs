pub use context_menu::*;
use itertools::Itertools;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, HorizontalAlignment, Offset, Rect, Size},
    macros::{line, span, text, vertical},
    style::{
        Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Widget},
};
use ratatui_textarea::{TextArea, WrapMode};
pub use scrolling_gallery::*;
pub use scrolling_list::*;

use crate::helpers::add_padding;

mod context_menu;
mod scrolling_gallery;
mod scrolling_list;

#[allow(clippy::too_many_arguments)]
pub fn input_field(
    tab_selected: bool,
    selected: bool,
    valid: bool,
    input: &mut TextArea<'static>,
    wrap_mode: WrapMode,
    frame: &mut Frame,
    area: Rect,
    title: &str,
    placeholder_text: &str,
    custom_padding: Option<Padding>,
) {
    input.set_style(Style::new().fg(if tab_selected && selected {
        tailwind::SLATE.c200
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
            .title(title.to_string())
            .title_style(Style::new().fg(if tab_selected {
                if selected {
                    material::BLUE.c400
                } else {
                    if valid { material::BLUE.c600 } else { material::RED.c600 }
                }
            } else {
                tailwind::STONE.c400
            }))
            .padding(custom_padding.unwrap_or(Padding::symmetric(1, 0))),
    );
    input.set_placeholder_text(placeholder_text);
    input.set_placeholder_style(Style::new().fg(material::GRAY.c700));
    input.set_wrap_mode(wrap_mode);

    frame.render_widget(&*input, area);
}

pub fn dropdown(tab_selected: bool, selected: bool, frame: &mut Frame, area: Rect, text: String) {
    // "▼⬇⬆⏷▲▴▼▾◥◤◣◢⥡⥝⥜⥠🡙🢓🢑"
    let dropdown_block =
        Block::bordered()
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
    frame.render_widget(&dropdown_block, area);
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
        dropdown_block.inner(area),
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
        dropdown_block.inner(area),
    );
}

pub fn window(frame: &mut Frame, area: Rect, title: &str, and_a_half: bool) -> Rect {
    let popup = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .border_style(Style::new().fg(tailwind::VIOLET.c950))
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(Style::new().fg(material::YELLOW.c800));

    let mut top_colors = vec![];
    for x in 0..area.width {
        top_colors.push(frame.buffer_mut().cell((area.x + x, area.y)).unwrap().bg);
    }
    let mut bottom_colors = vec![];
    for x in 0..area.width {
        bottom_colors.push(
            frame
                .buffer_mut()
                .cell((area.x + x, area.y + area.height - 1))
                .unwrap()
                .bg,
        );
    }

    let popup_area = popup.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    for x in 0..area.width {
        frame
            .buffer_mut()
            .cell_mut((area.x + x, area.y))
            .unwrap()
            .bg = top_colors[x as usize];
    }
    if and_a_half {
        frame.render_widget(
            Block::new()
                .borders(!Borders::TOP)
                .border_set(border::PROPORTIONAL_TALL)
                .border_style(Style::new().fg(tailwind::VIOLET.c950))
                .bg(tailwind::BLUE.c950),
            add_padding(area, Padding::top(1)),
        );
    } else {
        for x in 0..area.width {
            frame
                .buffer_mut()
                .cell_mut((area.x + x, area.y + area.height - 1))
                .unwrap()
                .bg = bottom_colors[x as usize];
        }
    }
    frame.render_widget(Block::new().bg(tailwind::BLUE.c950), popup_area);

    popup_area
}

pub fn scroll_bar(
    items_count: usize,
    scroll_pos: usize,
    num_visible_items: usize,
    frame: &mut Frame,
    area: Rect,
) {
    const BLOCKS: [char; 9] = ['█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' '];

    frame.render_widget(
        text!["█".repeat(area.height as usize),].fg(tailwind::INDIGO.c950),
        area,
    );
    frame.render_widget("▲".bg(tailwind::INDIGO.c700).fg(material::BLUE.c300), area);

    let num_pixels = (area.height as usize - 2) * 8;
    let max_scroll_amount = items_count.saturating_sub(num_visible_items);

    let mut handle_size = num_pixels.saturating_sub(max_scroll_amount);
    let mut scroll_pixels = handle_size.div_ceil(max_scroll_amount)
        // - if handle_size % max_scroll_amount == 0 { 1 } else { 0 })
    .min(3);
    handle_size -= scroll_pixels.saturating_sub(1) * max_scroll_amount;
    while handle_size < 8 && scroll_pixels > 1 {
        handle_size += max_scroll_amount;
        scroll_pixels -= 1;
    }

    if handle_size >= 8 {
        let mut top_margin = scroll_pos * scroll_pixels;
        let mut lines = Text::default();

        while top_margin >= 8 {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
            top_margin -= 8;
        }
        if top_margin > 0 {
            lines.push_line(
                BLOCKS[top_margin]
                    .bg(tailwind::INDIGO.c950)
                    .fg(material::BLUE.c300),
            );
            handle_size -= 8 - top_margin;
        }
        while handle_size >= 8 {
            lines.push_line(" ".bg(material::BLUE.c300));
            handle_size -= 8;
        }
        if handle_size > 0 {
            lines.push_line(
                BLOCKS[handle_size]
                    .fg(tailwind::INDIGO.c950)
                    .bg(material::BLUE.c300),
            );
        }
        while lines.lines.len() < area.height as usize - 2 {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }

        frame.render_widget(lines, area.offset(Offset::new(0, 1)));
    } else {
        let cycle_every = area.height as usize - 3;
        let scroll_fraction =
            scroll_pos as f32 / items_count.saturating_sub(num_visible_items) as f32;
        let phase = (scroll_fraction * cycle_every as f32) as usize;
        let block = BLOCKS[(scroll_fraction * 9.0 * cycle_every as f32) as usize % 9]
            .bg(tailwind::INDIGO.c950)
            .fg(material::BLUE.c300);

        let mut lines = Text::default();
        for _ in 0..phase {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }
        lines.push_line(block.clone());
        lines.push_line(block.reversed());
        for _ in 0..(area.height as usize - lines.lines.len()) {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }

        frame.render_widget(&lines, area);
    }

    frame.render_widget(
        "▼"
            .bg(tailwind::INDIGO.c700)
            .fg(material::BLUE.c300)
            .not_reversed(),
        area.offset(Offset::new(0, area.height as i32 - 1)),
    );
}

pub enum ActionType {
    Default,
    Normal,
    Critical,
}

pub struct Action {
    action:      &'static str,
    action_type: ActionType,
    selected:    bool,
    valid:       bool,
}
impl Action {
    pub fn new(action: &'static str, action_type: ActionType, selected: bool, valid: bool) -> Self {
        Self {
            action,
            action_type,
            selected,
            valid,
        }
    }
}
impl<'a> From<Action> for Span<'a> {
    fn from(value: Action) -> Span<'a> {
        span!(value.action)
            .fg(if value.valid {
                if value.selected {
                    tailwind::SLATE.c300
                } else {
                    match value.action_type {
                        ActionType::Default => tailwind::SLATE.c300,
                        ActionType::Normal => material::BLUE.c500,
                        ActionType::Critical => tailwind::RED.c500,
                    }
                }
            } else {
                tailwind::SLATE.c500
            })
            .bg(if value.valid {
                if value.selected {
                    match value.action_type {
                        ActionType::Default => material::BLUE.c600,
                        ActionType::Normal => material::BLUE.c800,
                        ActionType::Critical => tailwind::RED.c800,
                    }
                } else {
                    if matches!(value.action_type, ActionType::Default) {
                        material::BLUE.c900
                    } else {
                        tailwind::SLATE.c950
                    }
                }
            } else {
                if value.selected {
                    tailwind::SLATE.c700
                } else {
                    tailwind::SLATE.c800
                }
            })
    }
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
    buffer: &mut Buffer,
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

    line.render(area, buffer);

    mouse_areas
}

pub struct Hyperlink<'content> {
    pub text: Text<'content>,
    pub url:  String,
}

impl Widget for Hyperlink<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        (&self.text).render(area, buffer);

        if !self.text.lines.is_empty() {
            for (j, line) in self.text.lines.iter().enumerate() {
                if line.width() > 0 {
                    for (i, char) in line.to_string().chars().enumerate() {
                        let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, char);

                        buffer[(area.x + i as u16, area.y + j as u16)]
                            .set_symbol(hyperlink.as_str());
                        buffer[(area.x + i as u16, area.y + j as u16)].set_diff_option(
                            ratatui::buffer::CellDiffOption::ForcedWidth(
                                core::num::NonZero::new(1).unwrap(),
                            ),
                        );
                    }
                }
            }
        }
    }
}
