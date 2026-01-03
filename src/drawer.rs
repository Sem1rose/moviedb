use std::path::PathBuf;

use crate::{popups::*, screens::*, KeyEventHandler};
use ratatui::{
    layout::Constraint,
    style::{palette::tailwind::*, Stylize},
    text::{Line, Text},
    widgets::Block,
    Frame,
};

pub struct Drawer {
    pub current_screen: Option<Screens>,
    pub active_popup: Option<Popups>,

    show_term_size_warning: bool,

    refresh_immediate: u8,
}

const MINTERMSIZE: [u32; 2] = [100, 30];
impl Drawer {
    pub fn new(cache_dir: &PathBuf) -> Self {
        Drawer {
            current_screen: Some(Screens::MainScreen(MainScreen::new(cache_dir))),
            active_popup: None,
            show_term_size_warning: false,
            refresh_immediate: 0,
        }
    }

    pub fn render_app(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        self.refresh_immediate = self.refresh_immediate.saturating_sub(1);

        self.check_term_size(frame);
        self.update_image_renderers();

        self.draw_current_screen(frame, key_event_handler)?;

        self.check_popups()?;
        if !self.show_term_size_warning && self.active_popup.is_some() {
            self.draw_popup(frame, key_event_handler)?;
        }

        Ok(())
    }

    fn update_image_renderers(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            main_screen.image_renderer.update();
        }
    }

    fn draw_current_screen(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        frame.render_widget(Block::new().bg(SLATE.c900), frame.area());

        if self.show_term_size_warning {
            self.render_term_size_warning(frame);
        } else if let Some(current_screen) = self.current_screen.as_mut() {
            match current_screen {
                Screens::MainScreen(main_screen) => {
                    main_screen.render(frame, key_event_handler)?;
                }
            }
        }

        Ok(())
    }

    fn check_popups(&mut self) -> anyhow::Result<()> {
        if let Some(popup) = self.active_popup.as_mut() {
            match popup {
                Popups::EditMovie(_) => {}
                Popups::RemoveMovie(_) => {}
            }
        }

        Ok(())
    }

    fn draw_popup(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> anyhow::Result<()> {
        if let Some(active_popup) = self.active_popup.as_mut() {
            match active_popup {
                Popups::EditMovie(edit_movie_popup) => {
                    edit_movie_popup.render(frame, key_event_handler)?;
                }
                Popups::RemoveMovie(remove_movie_popup) => {
                    remove_movie_popup.render(frame, key_event_handler)?;
                }
            }
        }

        Ok(())
    }

    pub fn open_edit_movie_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.active_popup = Some(Popups::EditMovie(EditMoviePopup::new(
                main_screen.current_movie().get_user_rating(),
            )));
        }
    }
    pub fn open_remove_movie_popup(&mut self) {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            self.active_popup = Some(Popups::RemoveMovie(RemoveMoviePopup::new(
                &main_screen.current_movie().name,
            )));
        }
    }

    pub fn close_popups(&mut self) {
        self.active_popup = None;

        self.refresh_immediate += 2;
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_mut() {
            main_screen.redraw_images = 1;
        }
    }

    pub fn check_refresh_immediate(&mut self) -> bool {
        self.refresh_immediate > 0
    }
    pub fn check_refresh_delayed(&mut self) -> bool {
        if let Some(Screens::MainScreen(main_screen)) = self.current_screen.as_ref() {
            return main_screen.drawing_images;
        }

        false
    }

    fn check_term_size(&mut self, frame: &Frame) {
        self.show_term_size_warning = (frame.area().width as u32) < MINTERMSIZE[0]
            || (frame.area().height as u32) < MINTERMSIZE[1];
    }

    fn render_term_size_warning(&mut self, frame: &mut Frame) {
        let frame_area = frame.area();
        let lines = vec![
            Line::from_iter([
                "Terminal is too small: ".into(),
                frame_area.width.to_string().red(),
                "x".into(),
                frame_area.height.to_string().red(),
            ]),
            Line::default(),
            Line::from_iter([
                "Minimum size is: ".into(),
                MINTERMSIZE[0].to_string().green(),
                "x".into(),
                MINTERMSIZE[1].to_string().green(),
            ]),
        ];
        let area = crate::helpers::center_rect(
            frame_area,
            Constraint::Min(0),
            Constraint::Length(lines.len() as u16),
        );
        let text = Text::from(lines).centered();

        frame.render_widget(text, area);
    }
}
