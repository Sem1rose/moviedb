use crate::{app::App, draw::Drawer, types::*};
use ratatui::crossterm::{
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{
    crossterm::event::{self},
    prelude::*,
    CompletedFrame,
};
use std::{
    io::{stdout, Stdout},
    time::{Duration, Instant},
};

const FRAME_RATE: f32 = 60.0;
const TICK_RATE: f32 = 10.0;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: App,
    drawer: Drawer,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        Ok(Self {
            drawer: Drawer::default(),
            terminal,
            app: App::new()?,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.app.init()?;
        self.drawer.main_screen.filter_sort_movies(&self.app);
        if true {
            self.drawer.init_screen.skip_authorization();
        }

        self.set_panic_hook();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let frame_time = Duration::from_secs_f32(1.0 / FRAME_RATE);
        let frames_per_tick = (FRAME_RATE / TICK_RATE).floor() as u32;
        let mut last_frame = Instant::now();
        let mut tick_counter = 0;

        loop {
            self.draw()?;

            let timeout = frame_time
                .checked_sub(last_frame.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Ok(event) = event::read() {
                    self.app.handle_app_events(event, &mut self.drawer)?;
                }
            }

            if self.drawer.can_quit() {
                return Ok(());
            }

            if last_frame.elapsed() >= frame_time {
                last_frame = std::time::Instant::now();

                tick_counter += 1;
                if tick_counter >= frames_per_tick {
                    tick_counter = 0;

                    self.drawer.tick();
                }
            }
        }
    }

    pub fn exit() -> Result<()> {
        if let Err(err) = ratatui::try_restore() {
            eprintln!(
                "failed to restore terminal. Run `reset` or restart your terminal to recover: {}",
                err
            );

            return Err(Errors::Io(err));
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

    pub fn draw(&mut self) -> Result<CompletedFrame> {
        self.terminal
            .draw(|frame| {
                self.drawer
                    .render_app(frame, &mut self.app) //, frame_time)
                    .expect("error rendering app.")
            })
            .map_err(Errors::Io)
    }
}
