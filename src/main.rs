mod app;
// mod args;
mod config;
mod custom;
mod draw;
mod image_backends;
mod omdb;
mod popups;
mod screens;
mod tmdb;
mod trakt;
mod tui;
mod types;
// mod update_json;

// use log::debug;
use tui::Tui;
use types::Result;

fn main() -> Result<()> {
    env_logger::init();

    color_eyre::install()?;
    // let cli = args::Cli::parse();

    let mut tui = Tui::new()?;

    // debug!("Starting the app...");

    tui.init()?;
    tui.run()?;
    Tui::exit()?;
    // update_json::update_movies()?;

    Ok(())
}
