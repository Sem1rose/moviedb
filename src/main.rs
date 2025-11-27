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
mod types;
// mod update_json;

// use log::debug;
use crate::{app::App, draw::Drawer, types::*};
use anyhow::Context;
use ratatui::crossterm::event::{self};
use std::time::{Duration, Instant};

const FRAME_RATE: f32 = 60.0;
const TICK_RATE: f32 = 10.0;
fn main() -> anyhow::Result<()> {
    env_logger::init();

    // let cli = args::Cli::parse();

    let mut terminal: Term = ratatui::init();
    let mut drawer = Drawer::default();
    let mut app = App::new()?;

    app.init()?;
    drawer.main_screen.filter_sort_movies(&app);
    if true {
        drawer.init_screen.skip_authorization();
    }

    let frame_time = Duration::from_secs_f32(1.0 / FRAME_RATE);
    let frames_per_tick = (FRAME_RATE / TICK_RATE).floor() as u32;
    let mut last_frame = Instant::now();
    let mut tick_counter = 0;

    loop {
        draw(&mut terminal, &mut drawer, &mut app)?;

        let timeout = frame_time
            .checked_sub(last_frame.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Ok(event) = event::read() {
                app.handle_app_events(event, &mut drawer)?;
            }
        }

        if drawer.can_quit() {
            break;
        }

        if last_frame.elapsed() >= frame_time {
            last_frame = std::time::Instant::now();

            tick_counter += 1;
            if tick_counter >= frames_per_tick {
                tick_counter = 0;

                drawer.tick();
            }
        }
    }

    ratatui::restore();

    // update_json::update_movies()?;

    Ok(())
}

pub fn draw(terminal: &mut Term, drawer: &mut Drawer, app: &mut App) -> anyhow::Result<()> {
    terminal
        .draw(|frame| {
            drawer
                .render_app(frame, app) //, frame_time)
                .expect("error rendering app.")
        })
        .context("ass")
        .map(|_| ())
}
