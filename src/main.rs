use clap::Parser;

mod app;
mod args;
mod config_tmdb;
mod config_trakt;
mod database;
mod draw;
mod tmdb;
mod trakt;
mod tui;

use app::App;
// use config_trakt::Conf;
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};

use config_tmdb::Conf;
use database::*;
use ratatui::prelude::*;
use std::{cell::RefCell, error::Error, io::{stdout, Stdout}, rc::Rc};
// use trakt::*;
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
    // tmdb::get_movie_poster_banner(&mut config, app.movies[00]);
    let result = tui.run();
    Tui::<CrosstermBackend<Stdout>>::exit()?;

    result?;
    Ok(())
}
