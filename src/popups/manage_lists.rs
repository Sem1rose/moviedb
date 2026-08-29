use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{HorizontalAlignment, Layout, Margin, Offset, Rect, Size},
    macros::{constraint, horizontal, line, vertical},
    style::{
        Modifier, Style, Styled, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::{Block, Borders, Padding},
};
use ratatui_textarea::{TextArea, WrapMode};
use strum::{AsRefStr, EnumCount, EnumIter, IntoEnumIterator};

use crate::{
    helpers,
    key_event_handler::{Data, KeyEventHandler},
    popups::{Popup, PopupTrait},
    types::Entry,
    widgets::{self, Action, ActionType, ScrollView},
};

#[derive(Default, Clone, Copy, EnumCount, EnumIter, PartialEq, AsRefStr)]
#[repr(u8)]
#[strum(serialize_all = "title_case")]
enum Screen {
    #[default]
    Overview,
    Add,
}

#[derive(Default)]
pub struct ManageListsPopup {
    tab:    usize,
    item:   usize,
    screen: Screen,
}

impl ManageListsPopup {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl PopupTrait for ManageListsPopup {
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
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        let popup_area = widgets::window(
            frame,
            helpers::centered_area(28, 74, frame.area()),
            "Manage Lists",
            false,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );

        key_event_handler.bind_vertical((None, None), "".into(), |app, _| {
            if let Some(Popup::ManageLists(manage_lists_popup)) = app.drawer.active_popup.as_mut() {
                manage_lists_popup.screen = if manage_lists_popup.screen == Screen::Add {
                    Screen::Overview
                } else {
                    Screen::Add
                };
            }
        });

        let [tabs_area, main_area] = horizontal![==13, >=1].areas(popup_area);

        frame.render_widget(Block::new().bg(tailwind::STONE.c950), tabs_area);

        let tabs_areas: [Rect; Screen::COUNT] = vertical![==2; Screen::COUNT].areas(tabs_area);
        for (i, (tab, area)) in Screen::iter().zip(tabs_areas).enumerate() {
            let first = i == 0;
            let last = i == Screen::COUNT - 1;

            if !first {
                frame.render_widget(
                    Block::new()
                        .border_set(border::THICK)
                        .borders(Borders::TOP)
                        .fg(tailwind::STONE.c700),
                    area,
                );
            }
            let area = if last {
                area.resize(Size::new(area.width, area.height + 1))
            } else {
                area
            };

            frame.render_widget(Block::new().bg(tailwind::SLATE.c900), area);
            frame.render_widget(
                tab.as_ref()
                    .fg(tailwind::TEAL.c600)
                    .into_right_aligned_line(),
                helpers::add_padding(area, Padding::new(2, 2, 1, 0)),
            );
        }
        frame.render_widget(
            Block::new()
                .border_set(border::PROPORTIONAL_WIDE)
                .borders(Borders::LEFT)
                .fg(tailwind::VIOLET.c900),
            tabs_area
                .offset(Offset::new(tabs_area.width as i32 - 1, 0))
                .resize(Size::new(1, tabs_area.height)),
        );
        let selected_area = tabs_area
            .resize(Size::new(tabs_area.width, 3))
            .offset(Offset::new(0, (self.screen as u8 * 2) as i32));
        frame.render_widget(
            Block::new()
                .bg(tailwind::BLUE.c950)
                .fg(tailwind::VIOLET.c900)
                .border_set(border::PROPORTIONAL_TALL)
                .borders(!Borders::RIGHT),
            selected_area,
        );
        frame.render_widget(
            border::QUADRANT_BLOCK
                .fg(tailwind::BLUE.c950)
                .into_right_aligned_line(),
            helpers::add_padding(selected_area, Padding::top(1)),
        );
        frame.render_widget(
            self.screen
                .as_ref()
                .fg(tailwind::ORANGE.c400)
                .bold()
                .into_right_aligned_line(),
            helpers::add_padding(selected_area, Padding::new(2, 2, 1, 0)),
        );

        match &self.screen {
            Screen::Overview => {}
            Screen::Add => {}
        }
    }
}
