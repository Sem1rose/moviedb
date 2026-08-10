use itertools::Itertools;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, HorizontalAlignment, Offset, Position, Rect, Size},
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

use crate::{helpers::add_padding, key_event_handler::KeyEventHandler};

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
            .padding(Padding::symmetric(1, 0)),
    );
    input.set_placeholder_text(placeholder_text);
    input.set_placeholder_style(Style::new().fg(material::GRAY.c700));
    input.set_wrap_mode(wrap_mode);

    frame.render_widget(&*input, area);
}

pub fn dropdown(tab_selected: bool, selected: bool, frame: &mut Frame, area: Rect, text: String) {
    // "▼⬇⬆⏷▲▴▼▾◥◤◣◢⥡⥝⥜⥠🡙🢓🢑"
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
    key_event_handler: &mut KeyEventHandler,
) -> (Rect, usize) {
    let result = normal_popup(
        items,
        selected_index,
        scroll_pos,
        num_visible_items,
        dropdown_widget_area.offset(Offset::new(0, 2)).as_position(),
        dropdown_widget_area.width,
        frame,
        key_event_handler,
    );

    frame.render_widget(
        Block::new().bg(material::BLUE.c600),
        dropdown_widget_area
            .offset(Offset::new(0, 2))
            .resize(Size::new(dropdown_widget_area.width, 1)),
    );

    result
}

pub fn normal_popup(
    items: Vec<Line>,
    selected_index: usize,
    scroll_pos: usize,
    num_visible_items: usize,
    position: Position,
    width: u16,
    frame: &mut Frame,
    key_event_handler: &mut KeyEventHandler,
) -> (Rect, usize) {
    let items_len = items.len();
    let mut visible_items = items
        .into_iter()
        .dropping(scroll_pos)
        .take(num_visible_items)
        .collect_vec();
    let visible_items_len = visible_items.len();

    let selected = visible_items
        .remove(selected_index.saturating_sub(scroll_pos))
        .fg(material::BLUE.c100)
        .bg(material::LIGHT_BLUE.c900);
    visible_items.insert(selected_index.saturating_sub(scroll_pos), selected);

    let area = Rect {
        x: position.x,
        y: position.y,
        width,
        height: visible_items_len as u16 + 2,
    };
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
    frame.render_widget(Clear, area);
    key_event_handler.bind_mouse_button_down(
        ratatui::crossterm::event::MouseButton::Left,
        area,
        |_, _| {},
    );

    let sort_popup_block = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .fg(tailwind::INDIGO.c900);
    let inner_area = sort_popup_block.inner(area);
    frame.render_widget(&sort_popup_block, area);
    frame.render_widget(Block::new().bg(material::INDIGO.c900), inner_area);
    frame.render_widget(Text::from_iter(visible_items).left_aligned(), inner_area);

    if items_len > num_visible_items {
        scroll_bar(
            items_len,
            scroll_pos,
            num_visible_items,
            frame,
            inner_area
                .offset(Offset::new(inner_area.width as i32, 0))
                .resize(Size::new(1, inner_area.height)),
        );
    }

    for x in 0..area.width {
        frame
            .buffer_mut()
            .cell_mut((area.x + x, area.y))
            .unwrap()
            .bg = top_colors[x as usize];
    }
    for x in 0..area.width {
        frame
            .buffer_mut()
            .cell_mut((area.x + x, area.y + area.height - 1))
            .unwrap()
            .bg = bottom_colors[x as usize];
    }

    (
        inner_area.resize(Size {
            width:  inner_area.width,
            height: 1,
        }),
        visible_items_len,
    )
}

pub fn window_popup(frame: &mut Frame, area: Rect, title: &str, and_a_half: bool) -> Rect {
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
            .bg(tailwind::INDIGO.c600)
            .fg(material::BLUE.c300);

        let mut lines = Text::default();
        lines.push_line(block.clone());
        lines.push_line(block.reversed());

        frame.render_widget(lines, area.offset(Offset::new(0, phase as i32 + 1)));
    }

    frame.render_widget(
        "▼"
            .bg(tailwind::INDIGO.c700)
            .fg(material::BLUE.c300)
            .not_reversed(),
        area.offset(Offset::new(0, area.height as i32 - 1)),
    );
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
            // for (i, two_chars) in line.to_string().chars().chunks(2).into_iter().enumerate() {
            // let text = two_chars.collect::<String>();
            let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, line.to_string());
            buffer[(area.x, area.y + j as u16)].set_symbol(hyperlink.as_str());
            // }
        }
    }
}
