use crate::draw;
use crate::{app::App, input_handler};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, prelude::*, CompletedFrame};
use std::io::{stdout, Result};

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

            self.draw(app)?;
            input_handler::handle(app)?;
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
