use clap::Parser;

mod add;
mod app;
mod args;
mod config;
mod database;
mod draw;
mod tmdb;
mod trakt;
mod tui;

use app::App;
use config::Conf;
use database::*;
use ratatui::prelude::*;
use std::{error::Error, io::stdout};
use tmdb::*;
use trakt::*;
use tui::Tui;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = args::Cli::parse();

    let mut config = Conf::new();
    config.init()?;

    trakt::populate_tokens(&mut config)?;
    trakt::new(&config)?;
    // let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // let app = App::new(cli.command.is_some());
    // let mut tui = Tui::new(terminal, app);

    // tui.init()?;
    // let result = tui.run();
    // tui.exit()?;
    config.save_creds()?;

    // result?;
    Ok(())
}
