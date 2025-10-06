use crate::app::App;
use ratatui::{layout::Rect, Frame};

pub mod ratatui_image;

pub trait ImageBackend {
    fn new() -> Self
    where
        Self: Sized;
    fn update(&mut self);
    fn reload_images(&mut self, app: &App, start_index: usize, count: Option<usize>);
    fn draw_image(
        &mut self,
        app: &App,
        tmdb_id: u32,
        backdrop: bool,
        area: Rect,
        frame: &mut Frame,
    );
    fn remove_cached_image(&mut self, tmdb_id: u32);
}
