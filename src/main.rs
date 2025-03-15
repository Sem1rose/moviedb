mod app;
mod args;
mod config_tmdb;
mod config_trakt;
mod draw;
mod helpers;
mod popups;
mod screens;
mod tmdb;
mod trakt;
mod tui;
mod update_json;

use app::App;
use clap::Parser;
use color_eyre::{eyre::WrapErr, Result};
use log::{debug, info};
use std::error::Error;
use tui::Tui;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    color_eyre::install()?;
    // let cli = args::Cli::parse();

    let mut tui = Tui::new()?;

    debug!("Starting the app...");

    tui.init()?;
    tui.run()?;
    Tui::exit()?;

    // use update_json;
    // update_json::change_ratings()?;
    Ok(())
}
