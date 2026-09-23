use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use ratatui::{
    layout::{Offset, Position, Rect},
    macros::constraint,
    widgets::{Block, Padding},
};
use ratatui_image::sliced::SignedPosition;

use crate::widgets::Orientation;

pub fn centered_area(height: u16, width: u16, area: Rect) -> Rect {
    area.centered(
        constraint!(==width.min(area.width)),
        constraint!(==height.min(area.height)),
    )
}

pub fn add_padding(area: Rect, padding: Padding) -> Rect {
    Block::new().padding(padding).inner(area)
}

pub fn deconstruct_scrollview_area(
    area: Rect,
    orientation: Orientation,
    align_opposite: bool,
    num_hidden_rows: u16,
    buffer_negative_offset: i32,
) -> (Rect, Rect) {
    let visible_area = add_padding(
        area,
        Padding::new(
            if matches!(orientation, Orientation::Horizontal) && align_opposite {
                num_hidden_rows
            } else {
                0
            },
            if matches!(orientation, Orientation::Horizontal) && !align_opposite {
                num_hidden_rows
            } else {
                0
            },
            if matches!(orientation, Orientation::Vertical) && align_opposite {
                num_hidden_rows
            } else {
                0
            },
            if matches!(orientation, Orientation::Vertical) && !align_opposite {
                num_hidden_rows
            } else {
                0
            },
        ),
    );
    let input_area = visible_area.offset(Offset {
        x: if matches!(orientation, Orientation::Horizontal) {
            buffer_negative_offset
        } else {
            0
        },
        y: if matches!(orientation, Orientation::Vertical) {
            buffer_negative_offset
        } else {
            0
        },
    });

    (visible_area, input_area)
}

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.chars().count() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = vec![text.to_string()];
    loop {
        let line = lines.pop().unwrap();
        if line.chars().count() <= max_width {
            lines.push(line);
            break;
        }
        let wrap_whitespace_index = if line.chars().nth(max_width).unwrap().is_whitespace() {
            max_width - 1
        } else {
            line.chars()
                .collect_vec()
                .into_iter()
                .take(max_width)
                .rposition(|x| x.is_whitespace())
                .unwrap_or(max_width - 1)
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
pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() > max_width {
        new_string.truncate(new_string.ceil_char_boundary(max_width - 3));
        new_string += "...";
    }

    new_string
}

pub fn wrap_and_ellipsize(text: &str, max_width: usize, max_height: usize) -> Vec<String> {
    let mut lines = wrap_text(&text, max_width);
    if lines.len() > max_height {
        if max_width - lines[max_height - 1].chars().count() < 3 {
            let index = lines[max_height - 1].floor_char_boundary(max_width - 3);
            _ = lines[max_height - 1].split_off(index);
        }
        lines[max_height - 1].push_str("...");

        lines.resize(max_height, Default::default());
    }

    lines
}

pub trait SuperOrd {
    fn is_between(&self, lb: &Self, ub: &Self) -> bool;
}
impl<T: Ord> SuperOrd for T {
    fn is_between(&self, lb: &Self, ub: &Self) -> bool {
        self >= lb && self <= ub
    }
}

pub fn signed_subtract_pos(lhs: Position, rhs: Position) -> SignedPosition {
    SignedPosition {
        x: (lhs.x as i16 - rhs.x as i16),
        y: (lhs.y as i16 - rhs.y as i16),
    }
}
pub fn signed_pos_add(lhs: SignedPosition, rhs: SignedPosition) -> SignedPosition {
    SignedPosition {
        x: (lhs.x + rhs.x),
        y: (lhs.y + rhs.y),
    }
}

pub fn new_rc<T>(value: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(value))
}

#[macro_export]
macro_rules! load_file {
    ($name:expr, $home_dir:expr) => {
        {
            let path = &$home_dir.join(format!("{}.json", $name));
            match fs::read_to_string(path) {
                Err(error) => {
                    error!("Error reading {} file: {error}.\nRenaming corrupted file and creating a new database.", $name);

                    let mut renamed = $home_dir.join(format!("corrupted_{}.json", $name));
                    let mut i = 1;
                    while renamed.exists() {
                        renamed = $home_dir.join(format!("corrupted_{}_{i}.json", $name));
                        i += 1;
                    }

                    _ = fs::rename(path, renamed);
                    _ = fs::write(path, "[]");

                    None
                }
                Ok(read_result) => {
                    match serde_json::from_str::<Vec<_>>(&read_result) {
                        Err(error) => {
                            error!("Error deserializing {} file: {error}.\nRenaming corrupted file and creating a new database.", $name);

                            let mut renamed = $home_dir.join(format!("corrupted_{}.json", $name));
                            let mut i = 1;
                            while renamed.exists() {
                                renamed = $home_dir.join(format!("corrupted_{}_{i}.json", $name));
                                i += 1;
                            }

                            _ = fs::rename(path, renamed);
                            _ = fs::write(path, "[]");

                            None
                        }
                        Ok(deserialize_result) => {
                            Some(deserialize_result)
                        }
                    }
                }
            }
        }
    };
}
