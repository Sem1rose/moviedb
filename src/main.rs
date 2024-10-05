use clap::Parser;

mod app;
mod args;
mod config_tmdb;
mod config_trakt;
mod draw;
mod tmdb;
mod trakt;
mod tui;
mod update_json;

use app::App;
use color_eyre::Result;

use std::error::Error;
use tui::Tui;

fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let cli = args::Cli::parse();

    let app = App::new(cli.command.is_some())?;
    let mut tui = Tui::new(app)?;

    tui.init()?;
    tui.run()?;
    Tui::exit()?;

    // use update_json;
    // update_json::change_ratings()?;
    Ok(())
}
