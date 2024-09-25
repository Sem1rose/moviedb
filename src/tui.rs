use crate::{app::App, draw::Drawer, fetch_movies};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, prelude::*, CompletedFrame};
use std::{
    cell::RefCell,
    error::Error,
    io::{stderr, stdout},
};

pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    app: RefCell<App>,
    drawer: Drawer,
}

impl<B: Backend> Tui<B> {
    pub fn new(terminal: Terminal<B>, app: App) -> Self {
        Self {
            terminal,
            app: RefCell::new(app),
            drawer: Drawer::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        self.app.borrow_mut().set_movies(fetch_movies().unwrap());

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            if self.app.borrow().should_quit {
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
            self.app.borrow_mut().handle(&mut self.drawer)?;
        }
    }

    pub fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<CompletedFrame, std::io::Error> {
        self.terminal
            .draw(|frame| self.drawer.ui(frame, &mut self.app.borrow_mut()).unwrap())
    }
}
