use clap::Parser;

mod app;
mod args;
mod custom_widgets;
mod database;
mod draw;
mod input_handler;
mod tui;

use app::App;
use database::*;
use ratatui::prelude::*;
use std::io::{stdout, Result};
use tui::Tui;

fn main() -> Result<()> {
    let cli = args::Cli::parse();

    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut tui = Tui::new(terminal);
    tui.init()?;
    let mut app = App::new(cli.command.is_some());
    init_movies(&mut app);

    let result = tui.run(&mut app);

    tui.exit()?;
    result?;

    Ok(())
    // if let Some(config) = cli.config {
    //     println!("custom config: {}", config.display());
    // }
}

fn init_movies(app: &mut App) {
    app.set_movies(fetch_movies().unwrap());
}
