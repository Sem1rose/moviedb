use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    widgets::{Block, Clear, Padding},
    Frame,
};

pub fn center_rect(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn dynamic_area(
    max_height: Option<u16>,
    aspect_ratio: f64,
    h_align: Flex,
    v_align: Flex,
    area: Rect,
) -> Rect {
    let mut height = max_height.unwrap_or(area.height).min(area.height);
    let mut width = (height as f64 * aspect_ratio) as u16;

    if width > area.width {
        width = area.width;
        height = (width as f64 / aspect_ratio) as u16;
        if height > area.height {
            height = area.height;
        }
    }

    Layout::vertical([Constraint::Length(height)])
        .flex(v_align)
        .split(
            Layout::horizontal([Constraint::Length(width)])
                .flex(h_align)
                .split(area)[0],
        )[0]
}

pub fn dynamic_popup(
    frame: &mut Frame,
    max_height: Option<u16>,
    aspect_ratio: f64,
    popup_background: Color,
    title: &str,
    title_style: Style,
    title_alignment: Alignment,
    border_style: Style,
) -> Rect {
    let area = dynamic_area(
        max_height.map(|x| x + 2),
        aspect_ratio,
        Flex::Center,
        Flex::Center,
        frame.area(),
    );

    let popup = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .border_style(border_style)
        .title(title)
        .title_alignment(title_alignment)
        .title_style(title_style);

    let top_background = frame.buffer_mut().cell((area.x, area.y)).unwrap().bg;
    let bottom_background = frame
        .buffer_mut()
        .cell((area.x, area.y + area.height - 1))
        .unwrap()
        .bg;

    let popup_area = popup.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    frame.render_widget(
        Block::new().bg(top_background),
        add_padding(area, Padding::bottom(1)),
    );
    frame.render_widget(
        Block::new().bg(bottom_background),
        add_padding(area, Padding::top(1)),
    );
    frame.render_widget(Block::new().bg(popup_background), popup_area);

    popup_area
}

pub fn add_padding(area: Rect, padding: Padding) -> Rect {
    Block::new().padding(padding).inner(area)
}

pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() >= max_width {
        new_string.truncate(max_width - 3);
        new_string += "...";
    }

    new_string
}
