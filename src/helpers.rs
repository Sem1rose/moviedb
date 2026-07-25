use std::ops::Not;

use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Alignment, Flex, Offset, Rect, Size},
    macros::{horizontal, vertical},
    style::{Color, Style, Stylize},
    symbols::border,
    widgets::{Block, Borders, Clear, Padding},
};

pub fn wrap_text(line: &str, width: usize) -> Vec<String> {
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }

    let mut lines = vec![line.to_string()];
    loop {
        let line = lines.pop().unwrap();
        if line.chars().count() <= width {
            lines.push(line);
            break;
        }
        let wrap_whitespace_index = if line.chars().nth(width).unwrap().is_whitespace() {
            width - 1
        } else {
            line.chars()
                .collect_vec()
                .into_iter()
                .take(width)
                .rposition(|x| x.is_whitespace())
                .unwrap_or(width - 1)
        } + 1;

        let mut line = line.chars().collect_vec();
        let remaining_line = line
            .split_off(wrap_whitespace_index)
            .iter()
            .collect::<String>()
            .trim_start()
            .to_string();

        lines.push(line.iter().collect());
        lines.push(remaining_line);
    }

    lines
}

pub fn static_area(height: u16, width: u16, area: Rect) -> Rect {
    vertical![==height.min(area.height)]
        .flex(Flex::Center)
        .split(
            horizontal![==width.min(area.width)]
                .flex(Flex::Center)
                .split(area)[0],
        )[0]
}

pub fn dynamic_area(max_height: u16, aspect_ratio: f64, area: Rect) -> Rect {
    let mut height = max_height.min(area.height);
    let mut width = (height as f64 * aspect_ratio) as u16;

    if width > area.width {
        width = area.width;
        height = (width as f64 / aspect_ratio) as u16;
        if height > area.height {
            height = area.height;
        }
    }

    vertical![==height]
        .flex(Flex::Center)
        .split(horizontal![==width].flex(Flex::Center).split(area)[0])[0]
}

pub fn create_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    title_style: Style,
    title_alignment: Alignment,
    border_style: Style,
    background_color: Color,
    and_a_half: bool,
) -> Rect {
    let popup = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .border_style(border_style)
        .title(title)
        .title_alignment(title_alignment)
        .title_style(title_style);

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
                .borders(Borders::TOP.not())
                .border_set(border::PROPORTIONAL_TALL)
                .border_style(border_style)
                .bg(background_color),
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
    frame.render_widget(Block::new().bg(background_color), popup_area);

    popup_area
}

pub fn add_padding(area: Rect, padding: Padding) -> Rect {
    Block::new().padding(padding).inner(area)
}

pub fn resize_area(area: Rect, offset: Offset) -> Rect {
    area.resize(Size::new(
        (area.width as i32 + offset.x) as u16,
        (area.height as i32 + offset.y) as u16,
    ))
    .offset(Offset::new(-offset.x / 2, -offset.y / 2))
}

pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() > max_width {
        new_string.truncate(max_width - 3);
        new_string += "...";
    }

    new_string
}
