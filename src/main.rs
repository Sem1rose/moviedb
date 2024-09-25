use clap::Parser;

mod add;
mod app;
mod args;
mod database;
mod draw;
mod tui;

use app::App;
use database::*;
use ratatui::prelude::*;
use std::{error::Error, io::stdout};
use tui::Tui;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = args::Cli::parse();

    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let app = App::new(cli.command.is_some());
    let mut tui = Tui::new(terminal, app);
    tui.init()?;

    let result = tui.run();

    tui.exit()?;
    result?;

    Ok(())
}
