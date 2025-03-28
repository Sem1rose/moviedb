use ratatui::layout::*;
use ratatui_macros::vertical;

pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() > max_width {
        new_string.truncate(max_width - 3);
        new_string += "...";
    }

    new_string
}

pub fn center_rect(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn v_center(rect: Rect) -> Rect {
    vertical![>=0, ==1, >=0].split(rect)[1]
}
