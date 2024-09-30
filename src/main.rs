use clap::Parser;

mod app;
mod args;
mod config_tmdb;
mod config_trakt;
mod draw;
mod tmdb;
mod trakt;
mod tui;

use app::{App, Movie};
use color_eyre::Result;

use config_tmdb::Conf;
use ratatui::prelude::*;
use std::{
    error::Error,
    fs,
    io::{stdin, stdout, Stdout, Write},
};
use tui::Tui;

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let cli = args::Cli::parse();

    // trakt::populate_tokens(&mut config)?;
    // trakt::new(&config)?;
    // tmdb::populate_tokens(&mut config);
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let config = Conf::new();
    let app = App::new(cli.command.is_some());
    let mut tui = Tui::new(terminal, app, config);

    tui.init()?;
    let result = tui.run();
    Tui::<CrosstermBackend<Stdout>>::exit()?;

    result?;
    // change_ratings();
    Ok(())
}

// fn change_ratings() {
//     let file_path = dirs::config_dir()
//         .expect("Couldn't get user's config dir")
//         .join("moviedb")
//         .join("ratings4.json");

//     let file_contents = fs::read_to_string(&file_path)
//         .unwrap_or_else(|_| panic!("Couldn't read database contents at {}", file_path.display()));
//     let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
//         panic!(
//             "Couldn't parse database contents at {}",
//             file_path.display()
//         )
//     });

//     let user_ratings = json_contents
//         .members()
//         .map(|x| {
//             x["user_rating"]
//                 .to_string()
//                 .parse()
//                 .expect("couldn't parse ratings!")
//         })
//         .collect::<Vec<f32>>();
//     let mut config = Conf::new();
//     config.init();
//     let mut out: Vec<Movie> = vec![];

//     let movies = json_contents
//         .members()
//         .map(|x| x["id"].to_string().parse().unwrap())
//         .collect::<Vec<u32>>();
//     for (i, x) in movies.iter().enumerate() {
//         let movie_details =
//             tmdb::get_movie_details(&config, *x).expect("couldn't get movie details");

//         let mut collection: Option<String> = None;
//         let mut collection_id: Option<u32> = None;
//         if movie_details.belongs_to_collection.is_some() {
//             collection = Some(movie_details.belongs_to_collection.clone().unwrap().name);
//             collection_id = Some(movie_details.belongs_to_collection.clone().unwrap().id);
//         }
//         out.push(Movie::new(
//             movie_details.title,
//             user_ratings[i],
//             movie_details.vote_average,
//             movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
//             movie_details.id,
//             movie_details
//                 .genres
//                 .iter()
//                 .map(|x| x.name.to_string())
//                 .collect(),
//             movie_details.overview,
//             collection,
//             collection_id,
//             movie_details.runtime,
//             movie_details.status == "Released",
//             movie_details.tagline,
//             movie_details.vote_count,
//         ));
//     }

//     // let movies = json_contents
//     //     .members()
//     //     .map(|x| x["name"].to_string())
//     //     .collect::<Vec<String>>();
//     // for (i, x) in movies.iter().enumerate() {
//     //     let x = tmdb::find_movie(&config, x.as_str())
//     //         .unwrap_or_else(|_| panic!("couldn't get for movie {x}"));
//     //     for y in x.iter() {
//     //         println!(
//     //             "{} - {} - {} - {}",
//     //             y.id, y.title, y.release_date, y.vote_average
//     //         );
//     //     }

//     //     print!("Choose: ");
//     //     let _ = stdout().flush();

//     //     let mut input = String::new();
//     //     let mut j: u32 = 0;

//     //     if stdin().read_line(&mut input).is_ok() && input.trim() != "" {
//     //         if let Some('\n') = input.chars().next_back() {
//     //             input.pop();
//     //         }
//     //         if let Some('\r') = input.chars().next_back() {
//     //             input.pop();
//     //         }

//     //         j = input.parse().expect("couldn't parse input");
//     //     }
//     //     let movie_details = tmdb::get_movie_details(&config, x[j as usize].id as u32)
//     //         .expect("couldn't get movie details");

//     //     out.push(Movie::new(
//     //         movie_details.title,
//     //         user_ratings[i],
//     //         movie_details.vote_average,
//     //         movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
//     //         movie_details.id,
//     //         movie_details
//     //             .genres
//     //             .iter()
//     //             .map(|x| x.name.to_string())
//     //             .collect(),
//     //         movie_details.overview,
//     //         if movie_details.belongs_to_collection.is_some() {
//     //             Some(movie_details.belongs_to_collection.unwrap().name)
//     //         } else {
//     //             None
//     //         },
//     //         movie_details.runtime,
//     //         movie_details.status == "Released",
//     //         movie_details.tagline,
//     //         movie_details.vote_count,
//     //     ));
//     // }

//     let string = serde_json::to_string_pretty(out.as_slice()).unwrap();

//     println!("{string}");

//     fs::write(
//         dirs::config_dir()
//             .expect("Couldn't get user's config dir")
//             .join("moviedb")
//             .join("ratings3.json"),
//         string,
//     );
// }
