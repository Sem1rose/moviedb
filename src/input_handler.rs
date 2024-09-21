use crate::app::App;
use crossterm::{
    event,
    event::{Event, KeyCode, KeyEventKind},
};
use std::io::Result;

pub fn handle(app: &mut App) -> Result<()> {
    match event::read()? {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Up => {
                    app.dec_movie_selection();
                }
                KeyCode::Down => {
                    app.inc_movie_selection();
                }
                _ => return Ok(()),
            }
        }
        _ => return Ok(()),
    }
    Ok(())
}
