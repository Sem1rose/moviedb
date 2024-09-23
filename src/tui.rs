use crate::draw;
use crate::{app::App, input_handler};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, prelude::*, CompletedFrame};
use std::io::{stderr, stdout, Result};
use std::time;

pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
}

impl<B: Backend> Tui<B> {
    pub fn new(_terminal: Terminal<B>) -> Self {
        Self {
            terminal: _terminal,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn run(&mut self, app: &mut App) -> Result<()> {
        loop {
            if app.should_quit {
                return Ok(());
            }

            // if now.elapsed().as_millis() > 30 {
            // TEMPORARY
            // TODO remove this piece of shit
            if app.clear_images {
                // *self.terminal.current_buffer_mut() =
                //     Buffer::empty(self.terminal.current_buffer_mut().area);
                // self.terminal.swap_buffers();
                // *self.terminal.current_buffer_mut() =
                //     Buffer::empty(self.terminal.current_buffer_mut().area);

                self.terminal.clear();
                app.update_images = true;
                app.clear_images = false;

                // execute!(stdout(), LeaveAlternateScreen)?;
                // println!("ass");
                // execute!(stdout(), EnterAlternateScreen)?;
            }
            self.draw(app)?;
            input_handler::handle(app)?;
            // now = time::Instant::now();
            // }
        }
    }

    pub fn exit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> Result<CompletedFrame> {
        self.terminal.draw(|frame| draw::ui(frame, app))
    }
}
