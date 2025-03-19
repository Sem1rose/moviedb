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

use app::Result;
use log::debug;
use tui::Tui;

fn main() -> Result<()> {
    env_logger::init();

    color_eyre::install()?;
    // let cli = args::Cli::parse();

    let mut tui = Tui::new()?;

    debug!("Starting the app...");

    tui.init()?;
    tui.run()?;
    Tui::exit()?;

    Ok(())
}
