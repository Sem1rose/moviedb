use clap::Parser;

mod app;
mod args;
mod config_tmdb;
mod config_trakt;
mod draw;
mod tmdb;
mod trakt;
mod tui;

use app::App;
use color_eyre::Result;

use config_tmdb::Conf;
use ratatui::prelude::*;
use std::{
    error::Error,
    io::{stdout, Stdout},
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
    Ok(())
}

// fn change_ratings() {
//     let file_path = dirs::config_dir()
//         .expect("Couldn't get user's config dir")
//         .join("moviedb")
//         .join("ratings.json");

//     let file_contents = fs::read_to_string(&file_path)
//         .unwrap_or_else(|_| panic!("Couldn't read database contents at {}", file_path.display()));
// let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
//     panic!(
//         "Couldn't parse database contents at {}",
//         file_path.display()
//     )
// });

// let movies = json_contents
//     .members()
//     .map(|x| x["name"].to_string())
//     .collect::<Vec<String>>();

// let mut config = Conf::new();
// config.init();
// let mut out: Vec<Movie> = vec![];
// for i in movies {
//     let x = tmdb::find_movie(&config, i.as_str())
//         .unwrap_or_else(|_| panic!("couldn't get for movie {i}"));
//     for y in x.iter() {
//         println!(
//             "{} - {} - {} - {}",
//             y.id, y.title, y.release_date, y.vote_average
//         );
//     }

//     print!("Choose: ");
//     let _ = stdout().flush();

//     let mut input = String::new();
//     stdin()
//         .read_line(&mut input)
//         .expect("Did not enter a correct string");
//     if let Some('\n') = input.chars().next_back() {
//         input.pop();
//     }
//     if let Some('\r') = input.chars().next_back() {
//         input.pop();
//     }

//     let j: u32 = input.parse().expect("couldn't parse input");
//     let movie_details = tmdb::get_movie_details(&config, x[j as usize].id as u32)
//         .expect("couldn't get movie details");

//     out.push(Movie::new(
//         movie_details.title,
//         movie_details.vote_average,
//         movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
//         movie_details.id,
//     ));
// }

// let string = serde_json::to_string_pretty(out.as_slice()).unwrap();

//     // let string = serde_json::to_string_pretty(&file_contents).unwrap();

// println!("{string}");

// fs::write(
//     dirs::config_dir()
//         .expect("Couldn't get user's config dir")
//         .join("moviedb")
//         .join("ratings4.json"),
//     string,
// );
// }
