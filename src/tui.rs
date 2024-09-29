use crate::{app::App, config_tmdb::Conf, draw::Drawer, tmdb};
use color_eyre::Result;
use crossterm::{
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
        loop {
            if self.app.should_quit {
                return Ok(());
            }

            // TODO remove this piece of shit
            if self.drawer.clear_images {
                // *self.terminal.current_buffer_mut() =
                //     Buffer::empty(self.terminal.current_buffer_mut().area);
                // self.terminal.swap_buffers();
                // *self.terminal.current_buffer_mut() =
                //     Buffer::empty(self.terminal.current_buffer_mut().area);

                self.terminal.clear()?;
                self.drawer.clear_images = false;

                // execute!(stdout(), LeaveAlternateScreen)?;
                // println!("ass");
                // execute!(stdout(), EnterAlternateScreen)?;
            }
            self.draw()?;
            self.app.handle(&mut self.drawer)?;
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
        self.terminal
            .draw(|frame| self.drawer.ui(frame, &mut self.app, &self.config).unwrap())
    }
}
