use crate::app::Movie;

use std::{env, fs, path::Path};
// use std::io::{stdin, stdout, Write};
// use termion::{
//     clear, cursor, event,
//     input::TermRead,
//     raw::IntoRawMode,
//     screen::{IntoAlternateScreen, ToAlternateScreen, ToMainScreen},
//     scroll, terminal_size,
// };

pub fn fetch_movies() -> Result<Vec<Movie>, ()> {
    let file_path = Path::new(&env::var("HOME").ok().unwrap()).join("Videos/ratings.json");

    let file_contents = fs::read_to_string(&file_path).expect(&format!(
        "Couldn't read database contents at {}",
        file_path.display()
    ));
    let json_contents = json::parse(&file_contents).expect(&format!(
        "Couldn't parse database contents at {}",
        file_path.display()
    ));

    // let movies_factory = |a: String, x: &(String, String, String, f32)| {
    //     format!("{}-  [{}]({})\n  -  {}\n", a, &x.0, &x.2, &x.3.to_string())
    // };

    let movies = json_contents
        .members()
        .map(|x| {
            Movie::new(
                x["name"].to_string(),
                x["rating"]
                    .to_string()
                    .parse::<f32>()
                    .expect("Rating was not a number!"),
                x["url"].to_string(),
                {
                    let split_pos = x["url"].to_string().char_indices().nth_back(3).unwrap().0;
                    String::from(&x["url"].to_string()[split_pos..])
                },
                0,
            )
        })
        .collect::<Vec<Movie>>();

    // let output = movies.iter().fold(String::from(""), |a, x| {
    //     format!(
    //         "{}{}\n",
    //         // "{}{}{}{}\n",
    //         a,
    //         // &add_poster(),
    //         // cursor::Up(thumbnail_dimensions[1] as u16 - 1),
    //         x
    //     )
    // });

    // let output = output
    //     .trim_end()
    //     .lines()
    //     .map(String::from)
    //     .collect::<Vec<String>>();

    // let mut screen = stdout().into_alternate_screen().unwrap();
    // let term_size = terminal_size().unwrap();
    // let count = (term_size.1 as f32 / thumbnail_dimensions[1] as f32).floor() as u32;
    // write!(
    //     screen,
    //     "{}{}{}{}{}{}",
    //     cursor::Hide,
    //     clear::All,
    //     cursor::Goto(1, 1),
    //     add_images(&count),
    //     cursor::Goto(1, 1),
    //     lines2string(
    //         &output[0..(std::cmp::min(terminal_size().unwrap().1 as usize, output.len() - 1))]
    //     ),
    // )
    // .unwrap();
    // screen.flush().unwrap();

    // let mut screen = stdout()
    //     .into_raw_mode()
    //     .unwrap()
    //     .into_alternate_screen()
    //     .unwrap();

    // let mut selection = 0usize;
    // let mut page_top = 0usize;
    // stdin().keys().for_each(|x| {
    //     let term_size = terminal_size().unwrap();
    //     if let Ok(key) = x {
    //         let count = (term_size.1 as f32 / thumbnail_dimensions[1] as f32).floor() as u32;

    //         if key == event::Key::Up {
    //             if selection > 0 {
    //                 selection -= 1;
    //                 if selection < page_top {
    //                     page_top -= 1
    //                 }
    //                 write!(
    //                     screen,
    //                     "{}{}{}{}{} {} {}",
    //                     // "{}{}{}",
    //                     clear::All,
    //                     cursor::Goto(1, 1),
    //                     add_images(&count),
    //                     cursor::Goto(1, 1),
    //                     lines2string(
    //                         &output[(page_top * 8)
    //                             ..(std::cmp::min(
    //                                 term_size.1 as usize + page_top * 8,
    //                                 output.len()
    //                             ))]
    //                     ),
    //                     selection,
    //                     page_top
    //                 )
    //                 .unwrap();
    //             }
    //         } else if key == event::Key::Down {
    //             if selection < movies.len() - 1 {
    //                 // if (term_size.1 as u32 + row as u32) <= (output.len() as u32 - 10) {
    //                 selection += 1;
    //                 if selection > (page_top + (count - 1) as usize) {
    //                     page_top += 1;
    //                 }
    //                 write!(
    //                     screen,
    //                     "{}{}{}{}{} {} {}",
    //                     // "{}{}{}",
    //                     clear::All,
    //                     cursor::Goto(1, 1),
    //                     add_images(&count),
    //                     cursor::Goto(1, 1),
    //                     lines2string(
    //                         &output[(page_top * 8)
    //                             ..(std::cmp::min(
    //                                 term_size.1 as usize + page_top * 8,
    //                                 output.len()
    //                             ))]
    //                     ),
    //                     selection,
    //                     page_top
    //                 )
    //                 .unwrap();
    //             }
    //         } else if key == event::Key::Char('q') {
    //             panic!("ass");
    //         }

    //         // write!(
    //         //     screen,
    //         //     " {} {} {}",
    //         //     movie_index, bot_movie_index, scroll_count
    //         // )
    //         // .unwrap();

    //         screen.flush().unwrap();
    //     }
    // });

    Ok(movies)
}

// fn add_images(count: &u32) -> String {
// let top_movie_index = (row as f32 / thumbnail_dimensions[1] as f32).floor() as u32;
// let bot_movie_index =
//     ((term_height as f32 - 1.0 + row as f32) / thumbnail_dimensions[1] as f32).floor() as u32;
// // let scroll_count = (row as f32 % thumbnail_dimensions[1] as f32).floor() as u32;

// let posters = (top_movie_index..(bot_movie_index + 1))
//     .fold(String::from(""), |a, _| format!("{}{}\n", a, add_poster()));
// let posters = (0..*count).fold(String::from(""), |a, _| format!("{}{}\n", a, add_poster()));

// posters.trim().to_string()
// bot_movie_index.to_string()
// format!(
//     // "{} {} {} {}{}",
//     "{}",
//     posters.trim(),
//     // top_mocvie_index,
//     // bot_movie_index,
//     // scroll_count,
//     // scroll::Down(if scroll_count == 0 {
//     //     0
//     // } else {
//     //     8 - scroll_count as u16
//     // }),
// )
// }

// fn lines2string(lines: &[String]) -> String {
//     lines
//         .iter()
//         .fold(String::from(""), |a, x| a + x + "\r\n")
//         .trim_end()
//         .to_string()
// }
