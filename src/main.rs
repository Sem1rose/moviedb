mod app;
mod drawer;
mod helpers;
mod image_backend;
mod key_event_handler;
mod omdb;
mod popups;
mod screens;
mod tmdb;
mod tokens;
mod trakt;
mod types;

use app::App;
use drawer::Drawer;
use key_event_handler::KeyEventHandler;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::new()
        .format_source_path(true)
        .format_timestamp_millis()
        .init();

    App::new()?.run()?;

    ratatui::restore();

    Ok(())
}
