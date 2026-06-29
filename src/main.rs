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
mod widgets;

use std::time::SystemTime;

use app::App;
use chrono::{DateTime, Utc};
use drawer::Drawer;
use key_event_handler::KeyEventHandler;

fn main() -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                DateTime::<Utc>::from(SystemTime::now()).format("%+"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open("output.log")?,
        )
        .apply()?;

    App::new()?.run()
}
