use crate::{
    app::App,
    config_tmdb::Conf,
    draw::{CurrentScreen, Drawer},
    tmdb,
};
use color_eyre::Result;
use ratatui::crossterm::{
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::Backend, prelude::*, CompletedFrame};
use std::{error::Error, io::stdout};

pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    app: App,
    drawer: Drawer,
    config: Conf,
}

impl<B: Backend> Tui<B> {
    pub fn new(terminal: Terminal<B>, app: App, config: Conf) -> Self {
        Self {
            terminal,
            app,
            config,
            drawer: Drawer::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.config.init()?;
        tmdb::populate_tokens(&mut self.config)?;
        self.app.fetch_movies(&self.config);

        self.set_panic_hook();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // let mut frame_time = 0.0;
        let mut draw_time = 0.0;
        let mut now = std::time::Instant::now();
        loop {
            if self.app.should_quit
                && (*self
                    .drawer
                    .fetch_artwork_popup_options
                    .init_progress
                    .lock()
                    .unwrap() as usize)
                    == self.app.movies.len()
            {
                return Ok(());
            }

            if now.elapsed().as_secs_f64() >= 1.0 / 130.0 - draw_time {
                // if self.drawer.current_screen != CurrentScreen::TermSizeWarn
                //     && (
                if self.drawer.update
                    || self.drawer.throbber_visible
                    || self.drawer.clear_images
                    || !self.drawer.backdrop_displayed
                    || !self.drawer.all_movies_displayed
                {
                    self.drawer.update = false;
                    self.drawer.throbber_visible = false;

                    let start_draw_instant = std::time::Instant::now();
                    // TODO remove this piece of shit
                    if self.drawer.clear_images {
                        self.terminal.clear()?;
                        self.drawer.clear_images = false;
                    }
                    // self.draw(frame_time)?;
                    self.draw()?;

                    // frame_time = now.elapsed().as_secs_f64();
                    now = std::time::Instant::now();
                    draw_time = start_draw_instant.elapsed().as_secs_f64();
                }
                self.app.handle(&mut self.drawer)?;
            }
        }
    }

    pub fn exit() -> Result<(), Box<dyn Error>> {
        // disable_raw_mode()?;
        // execute!(stdout(), LeaveAlternateScreen)?;
        if let Err(err) = ratatui::try_restore() {
            eprintln!(
                "failed to restore terminal. Run `reset` or restart your terminal to recover: {}",
                err
            );

            return Err(Box::new(err));
        }

        Ok(())
    }

    fn set_panic_hook(&self) {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = Self::exit(); // ignore any errors as we are already failing
            hook(panic_info);
        }));
    }

    pub fn draw(&mut self) -> Result<CompletedFrame, std::io::Error> {
        // pub fn draw(&mut self, frame_time: f64) -> Result<CompletedFrame, std::io::Error> {
        self.terminal.draw(|frame| {
            self.drawer
                .render_app(frame, &mut self.app, &self.config)
                // .ui(frame, &mut self.app, &self.config, frame_time)
                .unwrap()
        })
    }
}
