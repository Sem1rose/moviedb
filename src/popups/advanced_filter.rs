use ratatui::{
    Frame,
    layout::Alignment,
    macros::vertical,
    style::{
        Style,
        palette::{material, tailwind},
    },
};

use crate::{
    helpers::{add_padding, create_popup, static_area},
    key_event_handler::{self, KeyEventHandler},
    popups::{PopupTrait, Popups},
    screens::main_screen::FilterCriterion,
    types::Movie,
    widgets::{self, ActionTypes},
};

#[derive(Default)]
pub struct AdvancedFilterPopup {
    tab:             usize,
    item:            usize,
    filter_criteria: Vec<FilterCriterion>,
    movies:          Vec<Movie>,
}

impl AdvancedFilterPopup {
    pub fn new(filter_criteria: &[FilterCriterion]) -> Self {
        Self {
            tab:             0,
            item:            0,
            filter_criteria: filter_criteria.to_vec(),
            movies:          vec![],
        }
    }

    pub fn initialize(&mut self, movies: &[Movie]) {
        self.movies = movies.to_vec();
    }
}

impl PopupTrait for AdvancedFilterPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        // key_event_handler.bind_enter((Some(1), None), "Edit Criterion".into(), );
        // key_event_handler.bind_key((Some(1), None), ' ', "Delete Criterion".into(), );
        key_event_handler.bind_tab((None, None), "".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        advanced_filter_popup.tab += 1;
                        if advanced_filter_popup.tab > 2 {
                            advanced_filter_popup.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        advanced_filter_popup.tab =
                            advanced_filter_popup.tab.checked_sub(1).unwrap_or(2);
                    }
                    _ => {}
                }
            }
        });

        let popup_area = create_popup(
            frame,
            static_area(10, 55, frame.area()),
            " Advanced Filter ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
            tailwind::BLUE.c950,
            false,
        );
    }
}
