use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use ratatui::{
    layout::{Offset, Rect, Size},
    macros::constraint,
    widgets::{Block, Padding},
};

use crate::types::{FxIndexMap, Movie};

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

pub fn centered_area(height: u16, width: u16, area: Rect) -> Rect {
    area.centered(
        constraint!(==width.min(area.width)),
        constraint!(==height.min(area.height)),
    )
}

pub fn add_padding(area: Rect, padding: Padding) -> Rect {
    Block::new().padding(padding).inner(area)
}

pub fn resize_area_centered(area: Rect, offset: Offset) -> Rect {
    area.resize(Size::new(
        (area.width as i32 + offset.x) as u16,
        (area.height as i32 + offset.y) as u16,
    ))
    .offset(Offset::new(-offset.x / 2, -offset.y / 2))
}

pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() > max_width {
        new_string.truncate(new_string.ceil_char_boundary(max_width - 3));
        new_string += "...";
    }

    new_string
}

pub fn ids_to_movies(ids: &[u32], movies: &FxIndexMap<u32, Movie>) -> Vec<Movie> {
    ids.iter().map(|id| movies[id].clone()).collect()
}

pub fn is_between<T: PartialOrd>(v: T, lb: T, ub: T) -> bool {
    v >= lb && v <= ub
}

pub fn default_rc<T: Default>() -> Rc<RefCell<T>> {
    Rc::new(RefCell::default())
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
