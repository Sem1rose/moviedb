use crate::{
    app::{App, Result},
    draw::Drawer,
};
use ratatui::{prelude::*, widgets::*, Frame};
use style::palette::tailwind;

#[derive(Default)]
pub enum Phase {
    #[default]
    FetchArtwork,
}

#[derive(Default)]
pub struct InitScreen {
    pub phase: Phase,

    started_step: bool,
}

impl Drawer {
    pub fn render_init_screen(&mut self, frame: &mut Frame, app: &mut App) -> Result<()> {
        let frame_area = frame.area();
        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), frame_area);

        match self.init_screen_options.phase {
            Phase::FetchArtwork => {
                if !self.init_screen_options.started_step {
                    self.open_fetch_artworks_popup(app)?;

                    self.init_screen_options.started_step = true;
                }

                self.handle_init_screen_fetch_artworks();
            }
        }

        Ok(())
    }

    fn init_screen_advance_phase(&mut self) {
        self.init_screen_options.started_step = false;

        match self.init_screen_options.phase {
            Phase::FetchArtwork => {
                self.open_main_screen();
            }
        }
    }

    fn handle_init_screen_fetch_artworks(&mut self) {
        if self.fetch_artwork_popup_options.done {
            self.init_screen_advance_phase();
        }
    }
}
