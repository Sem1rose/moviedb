use std::fmt::Display;
use std::fs;
// use termion::{color, cursor};

// pub const thumbnail_dimensions: [usize; 2] = [11, 8];

// impl Display for Movie {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let image_padding = [
//             (0..thumbnail_dimensions[0]).fold(String::from(""), |a, _| a + " "),
//             (0..(thumbnail_dimensions[1] - 5)).fold(String::from(""), |a, _| a + "\n"),
//         ];
//         let title_length = self.name.len();
//         let mut width = title_length + 4;
//         let decoration = format!(
//             "{}{}",
//             color::Fg(color::Rgb(42, 172, 245)),
//             (0..width).fold(String::from(">"), |a, _| a + "=") + "<"
//         );
//         let rating_color = if self.rating >= 9f32 {
//             color::Fg(color::Rgb(0, 227, 99))
//         } else if self.rating >= 8f32 {
//             color::Fg(color::Rgb(9, 227, 0))
//         } else if self.rating >= 7f32 {
//             color::Fg(color::Rgb(164, 227, 0))
//         } else if self.rating >= 6f32 {
//             color::Fg(color::Rgb(241, 229, 0))
//         } else {
//             color::Fg(color::Rgb(241, 115, 0))
//         };
//         width += 2;

//         let name_hyperlink = format!(
//             "   \x1B]8;;{}\x07{}{}\x1B]8;;\x07",
//             self.url,
//             color::Fg(color::Rgb(0, 186, 150)),
//             self.name
//         );

//         write!(
//             f,
//             "{5}{decoration}\n{5}{}\n{5}{}{:^width$}\n{5}{decoration}\n{5}{}{:^width$}{6}",
//             name_hyperlink,
//             color::Fg(color::Rgb(255, 178, 29)),
//             self.year,
//             rating_color,
//             self.rating.to_string(),
//             image_padding[0],
//             image_padding[1],
//         )
//     }
// }

// use rand::prelude::*;
// use std::process::Command;

// pub fn add_poster() -> String {
//     let paths = fs::read_dir("./src")
//         .unwrap()
//         .filter_map(|x| x.ok())
//         .map(|x| x.path())
//         .filter(|x| x.extension().map_or(false, |x| x == "jpg"))
//         .collect::<Vec<_>>();

//     let choice = rand::thread_rng().gen_range(0..paths.len());
//     let path = paths[choice].to_str().expect("ass");

//     let output = Command::new("chafa")
//         .args([
//             "--align",
//             "top,right",
//             "--relative",
//             "on",
//             "--view-size",
//             (thumbnail_dimensions[0].to_string() + "x" + &thumbnail_dimensions[1].to_string())
//                 .as_str(),
//             path,
//         ])
//         .output()
//         .expect("failed to execute chafa");

//     String::from_utf8(output.stdout).expect("ass")
// format!(
//     "{}{}",
//      String::from_utf8(output.stdout).expect("ass"),
//     cursor::Up(thumbnail_dimensions[1] as u16 - 1)
// )
// }
