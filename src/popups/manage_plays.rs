use chrono::{DateTime, Local, Utc};
use itertools::Itertools;
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{HorizontalAlignment, Margin, Offset, Rect, Size},
    macros::{line, vertical},
    style::{
        Modifier, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::{Block, Padding},
};
use ratatui_textarea::{TextArea, WrapMode};

use crate::{
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{Data, KeyEventHandler},
    popups::{Popup, PopupTrait},
    types::Entry,
    widgets::{self, Action, ActionType, ScrollList},
};

#[derive(Default)]
enum Phase {
    #[default]
    Overview,
    EnterDetails,
}

#[derive(Default)]
pub struct ManagePlaysPopup {
    tab:            usize,
    item:           usize,
    phase:          Phase,
    confirm_delete: bool,
    one_shot:       bool,
    new_play:       bool,
    entry:          Option<Entry>,
    pub scrollview: ScrollList,

    pub rating_input: TextArea<'static>,
    pub date_input:   TextArea<'static>,
}

impl ManagePlaysPopup {
    pub fn new(entry: Option<Entry>) -> Self {
        Self {
            entry: entry.or_else(|| Some(Default::default())),
            scrollview: ScrollList::new(3),

            ..Default::default()
        }
    }

    pub fn new_edit_rating(entry: &Entry) -> Self {
        let mut s = Self {
            one_shot: true,
            phase: Phase::EnterDetails,
            rating_input: TextArea::from([format!("{:.1}", entry.get_user_rating())]),
            date_input: TextArea::from([entry.get_latest_play().with_timezone(&Local).to_string()]),

            ..Default::default()
        };

        s.rating_input
            .move_cursor(ratatui_textarea::CursorMove::End);
        s.date_input.move_cursor(ratatui_textarea::CursorMove::End);

        s
    }

    pub fn new_add_play() -> Self {
        Self {
            one_shot: true,
            new_play: true,
            phase: Phase::EnterDetails,

            ..Default::default()
        }
    }

    pub fn add_play(&mut self) {
        let rating = format!(
            "{:.1}",
            self.rating_input.lines()[0].parse::<f64>().unwrap()
        )
        .parse()
        .unwrap();
        let input = self.date_input.lines()[0].to_lowercase();
        let date = if ["now", ""].contains(&input.trim()) {
            chrono::Local::now()
        } else if input.trim() == "unknown" {
            Default::default()
        } else {
            input.parse().unwrap()
        }
        .to_utc();

        if let Some(entry) = self.entry.as_mut() {
            entry.add_play(date, rating, None);
        }
    }

    pub fn edit_play(&mut self) {
        let rating = format!(
            "{:.1}",
            self.rating_input.lines()[0].parse::<f64>().unwrap()
        )
        .parse()
        .unwrap();
        let input = self.date_input.lines()[0].to_lowercase();
        let date = if ["now", ""].contains(&input.trim()) {
            chrono::Local::now()
        } else if input.trim() == "unknown" {
            Default::default()
        } else {
            input.parse().unwrap()
        }
        .to_utc();

        if let Some(entry) = self.entry.as_mut() {
            let len = entry.history.len();
            entry.history[len - 1 - self.scrollview.selected_index].date = date;
            entry.history[len - 1 - self.scrollview.selected_index].rating = rating;

            entry
                .history
                .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
        }
    }

    pub fn delete_play(&mut self) {
        if let Some(entry) = self.entry.as_mut() {
            entry
                .history
                .remove(entry.history.len() - 1 - self.scrollview.selected_index);
        }
    }

    pub fn validate_rating(&mut self) -> bool {
        if self.rating_input.is_empty() {
            return false;
        }

        if let Ok(x) = self.rating_input.lines()[0].parse::<f64>() {
            return (0.0..=10.0).contains(&x);
        }
        false
    }

    pub fn validate_input_date(&mut self) -> bool {
        ["now", "unknown", ""].contains(&self.date_input.lines()[0].trim().to_lowercase().as_str())
            || self.date_input.lines()[0]
                .parse::<DateTime<Local>>()
                .is_ok()
    }
}

impl PopupTrait for ManagePlaysPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        image_renderer: &mut RatatuiImage,
    ) {
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

        match &self.phase {
            Phase::Overview => {
                let history_entries = self.entry.as_ref().unwrap().history.as_slice();
                let num_entries = history_entries.len();
                key_event_handler.bind_esc((None, None), "Back".into(), move |app, _| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.tab = 2;
                        manage_plays_popup.item = 0;
                        manage_plays_popup.confirm_delete = false;
                    }
                });
                key_event_handler.bind_esc((Some(2), None), "Back".into(), move |app, _| {
                    app.drawer.close_popup();
                });
                key_event_handler.bind_tab((None, None), "Navigate".into(), move |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.item = 0;
                        manage_plays_popup.confirm_delete = false;

                        match data {
                            Data::Direction(true, _) => {
                                manage_plays_popup.tab += 1;
                                if manage_plays_popup.tab > 2 {
                                    manage_plays_popup.tab = 0;
                                }
                            }
                            Data::Direction(false, _) => {
                                manage_plays_popup.tab =
                                    manage_plays_popup.tab.checked_sub(1).unwrap_or(2);
                                if manage_plays_popup.tab == 0 && num_entries == 0 {
                                    manage_plays_popup.tab = 2;
                                }
                            }
                            _ => (),
                        }
                    }
                });

                if num_entries != 0 {
                    key_event_handler.bind_vertical(
                        (Some(0), None),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.item = 0;
                                manage_plays_popup.confirm_delete = false;

                                if let Data::Direction(direction, _) = data {
                                    manage_plays_popup.scrollview.scroll(direction, num_entries);
                                }
                            }
                        },
                    );

                    key_event_handler.bind_horizontal(
                        (Some(0), None),
                        "Select".into(),
                        move |app, data| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.confirm_delete = false;

                                if matches!(data, Data::Direction(true, _) if manage_plays_popup.item != 0) ||
                                    matches!(data, Data::Direction(false, _) if manage_plays_popup.item == 0) {
                                    manage_plays_popup.item = (manage_plays_popup.item == 0) as usize;
                                }
                            }
                        },
                    );

                    key_event_handler.bind_enter(
                        (Some(0), Some(0)),
                        "Edit".into(),
                        move |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.item = 0;
                                manage_plays_popup.confirm_delete = false;

                                let entry = &manage_plays_popup.entry.as_ref().unwrap().history
                                    [num_entries
                                        - 1
                                        - manage_plays_popup.scrollview.selected_index];

                                manage_plays_popup.phase = Phase::EnterDetails;
                                manage_plays_popup.rating_input =
                                    TextArea::from([format!("{:.1}", entry.rating)]);
                                manage_plays_popup.date_input =
                                    TextArea::from([entry.date.with_timezone(&Local).to_string()]);

                                manage_plays_popup
                                    .rating_input
                                    .move_cursor(ratatui_textarea::CursorMove::End);
                                manage_plays_popup
                                    .date_input
                                    .move_cursor(ratatui_textarea::CursorMove::End);
                            }
                        },
                    );
                    key_event_handler.bind_enter(
                        (Some(0), Some(1)),
                        if self.confirm_delete { "Confirm" } else { "Delete" }.into(),
                        |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_ref()
                            {
                                if manage_plays_popup.confirm_delete {
                                    app.remove_movie_play();
                                }
                            }

                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if manage_plays_popup.confirm_delete {
                                    manage_plays_popup.item = 0;
                                    manage_plays_popup.delete_play();
                                }
                                manage_plays_popup.confirm_delete ^= true;
                            }
                        },
                    );

                    key_event_handler.bind_key(
                        (Some(0), None),
                        'e',
                        "Edit".into(),
                        move |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.item = 0;
                                manage_plays_popup.confirm_delete = false;

                                let entry = &manage_plays_popup.entry.as_ref().unwrap().history
                                    [num_entries
                                        - 1
                                        - manage_plays_popup.scrollview.selected_index];

                                manage_plays_popup.phase = Phase::EnterDetails;
                                manage_plays_popup.rating_input =
                                    TextArea::from([format!("{:.1}", entry.rating)]);
                                manage_plays_popup.date_input =
                                    TextArea::from([entry.date.with_timezone(&Local).to_string()]);

                                manage_plays_popup
                                    .rating_input
                                    .move_cursor(ratatui_textarea::CursorMove::End);
                                manage_plays_popup
                                    .date_input
                                    .move_cursor(ratatui_textarea::CursorMove::End);
                            }
                        },
                    );
                    if !self.confirm_delete {
                        key_event_handler.bind_key(
                            (Some(0), None),
                            'd',
                            "Delete".into(),
                            |app, _| {
                                if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    manage_plays_popup.item = 1;
                                    manage_plays_popup.confirm_delete = true;
                                }
                            },
                        );
                    }
                } else if self.tab == 0 {
                    self.tab = 1;
                }

                key_event_handler.bind_key((Some(0), None), 'a', "New".into(), |app, _| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.item = 0;
                        manage_plays_popup.confirm_delete = false;

                        manage_plays_popup.new_play = true;

                        manage_plays_popup.phase = Phase::EnterDetails;
                        manage_plays_popup.rating_input = TextArea::default();
                        manage_plays_popup.date_input = TextArea::default();
                    }
                });
                key_event_handler.bind_key((Some(1), None), 'a', "New".into(), |app, _| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.item = 0;
                        manage_plays_popup.confirm_delete = false;

                        manage_plays_popup.new_play = true;

                        manage_plays_popup.phase = Phase::EnterDetails;
                        manage_plays_popup.rating_input = TextArea::default();
                        manage_plays_popup.date_input = TextArea::default();
                    }
                });
                key_event_handler.bind_enter((Some(1), None), "New".into(), |app, _| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.item = 0;
                        manage_plays_popup.confirm_delete = false;

                        manage_plays_popup.new_play = true;

                        manage_plays_popup.phase = Phase::EnterDetails;
                        manage_plays_popup.rating_input = TextArea::default();
                        manage_plays_popup.date_input = TextArea::default();
                    }
                });
                key_event_handler.bind_enter((Some(2), None), "Close".into(), |app, _| {
                    app.drawer.close_popup();
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(17, 50, frame.area()),
                    " Manage Plays ",
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [list_area, new_area, _] = vertical![>=10, ==1, ==1].areas(popup_area);

                {
                    let tab_selected = self.tab == 0;

                    let list_area = helpers::add_padding(list_area, Padding::horizontal(1));
                    let list_block = Block::bordered()
                        .border_set(border::PROPORTIONAL_WIDE)
                        .fg(tailwind::SLATE.c900);
                    frame.render_widget(&list_block, list_area);
                    let list_block_inner = list_block.inner(list_area);
                    let scrollbar_area = list_area
                        .offset(Offset::new(list_area.width as i32 - 1, 1))
                        .resize(Size::new(1, list_area.height - 2));

                    self.scrollview.render(
                        num_entries,
                        list_block_inner,
                        scrollbar_area,
                        frame,
                        key_event_handler,
                        |scroll_view,
                         area,
                         index,
                         selected,
                         alternate,
                         frame,
                         key_event_handler| {
                            key_event_handler.bind_mouse_button_down(
                                ratatui::crossterm::event::MouseButton::Left,
                                area,
                                move |app, _| {
                                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                        app.drawer.active_popup.as_mut()
                                    {
                                        manage_plays_popup.tab = 0;
                                        manage_plays_popup.item = 0;
                                        manage_plays_popup.confirm_delete = false;
                                        manage_plays_popup.scrollview.goto_index(
                                            index,
                                            false,
                                            num_entries,
                                        );
                                    }
                                },
                            );

                            frame.render_widget(
                                Block::new().bg(if selected {
                                    if tab_selected {
                                        tailwind::TEAL.c700
                                    } else {
                                        tailwind::TEAL.c900
                                    }
                                } else if !alternate {
                                    tailwind::GRAY.c800
                                } else {
                                    tailwind::SLATE.c700
                                }),
                                area,
                            );

                            let entry = &history_entries[num_entries - 1 - index];
                            let latest = index == 0;
                            let local_date = entry.date.with_timezone(&chrono::Local);

                            let areas = (0..area.height)
                                .map(|i| Rect::new(area.x, area.y + i, area.width, 1))
                                .collect_vec();

                            for i in 0..area.height {
                                let index = if area.height < scroll_view.item_height {
                                    if scroll_view.alignment_bottom {
                                        i + (scroll_view.item_height - area.height)
                                    } else {
                                        i
                                    }
                                } else {
                                    i
                                };
                                if index == 0 && selected {
                                    frame.render_widget(
                                        line!("▔".repeat(area.width as usize)).fg(
                                            if tab_selected {
                                                tailwind::EMERALD.c800
                                            } else {
                                                tailwind::SLATE.c700
                                            },
                                        ),
                                        areas[i as usize],
                                    );
                                } else if index == 1 {
                                    let area = helpers::add_padding(
                                        areas[i as usize],
                                        Padding::horizontal(2),
                                    );

                                    frame.render_widget(
                                        line![
                                            format!("{:.1}", entry.rating)
                                                .fg(if entry.rating >= 9.0 {
                                                    tailwind::SKY.c400
                                                } else if entry.rating >= 8.0 {
                                                    tailwind::GREEN.c500
                                                } else if entry.rating >= 7.5 {
                                                    tailwind::LIME.c400
                                                } else if entry.rating >= 7.0 {
                                                    material::AMBER.c400
                                                } else if entry.rating >= 6.0 {
                                                    material::DEEP_ORANGE.c300
                                                } else {
                                                    material::RED.c400
                                                })
                                                .add_modifier(if latest {
                                                    Modifier::BOLD
                                                } else {
                                                    Modifier::empty()
                                                }),
                                            " @ ".white().bold(),
                                            if entry.date == DateTime::<Utc>::default() {
                                                "Unknown".into()
                                            } else {
                                                local_date.format("%d/%m/%Y %H:%M").to_string()
                                            }
                                            .fg(
                                                if latest {
                                                    if selected && tab_selected {
                                                        tailwind::YELLOW.c500
                                                    } else if tab_selected {
                                                        tailwind::YELLOW.c600
                                                    } else {
                                                        tailwind::AMBER.c600
                                                    }
                                                } else {
                                                    if selected && tab_selected {
                                                        tailwind::INDIGO.c200
                                                    } else if tab_selected {
                                                        tailwind::INDIGO.c300
                                                    } else {
                                                        tailwind::INDIGO.c400
                                                    }
                                                }
                                            ),
                                        ],
                                        area,
                                    );

                                    if tab_selected && selected {
                                        let actions_mouse_areas = widgets::actions(
                                            [
                                                Action::new(
                                                    if self.confirm_delete {
                                                        " Confirm "
                                                    } else {
                                                        " D "
                                                    },
                                                    ActionType::Critical,
                                                    self.item == 1,
                                                    true,
                                                ),
                                                Action::new(
                                                    " E ",
                                                    ActionType::Normal,
                                                    self.item == 0,
                                                    true,
                                                ),
                                            ],
                                            HorizontalAlignment::Right,
                                            false,
                                            1,
                                            area,
                                            frame,
                                        );
                                        for (i, mouse_area) in
                                            actions_mouse_areas.into_iter().enumerate()
                                        {
                                            key_event_handler.bind_mouse_button_down(
                                                ratatui::crossterm::event::MouseButton::Left,
                                                mouse_area,
                                                move |app, _| {
                                                    if i == 0 {
                                                        if let Some(Popup::ManagePlays(
                                                            manage_plays_popup,
                                                        )) = app.drawer.active_popup.as_ref()
                                                        {
                                                            if manage_plays_popup.confirm_delete {
                                                                app.remove_movie_play();
                                                            }
                                                        }

                                                        if let Some(Popup::ManagePlays(
                                                            manage_plays_popup,
                                                        )) = app.drawer.active_popup.as_mut()
                                                        {
                                                            manage_plays_popup.item = 1;

                                                            if manage_plays_popup.confirm_delete {
                                                                manage_plays_popup.item = 0;
                                                                manage_plays_popup.delete_play();
                                                            }
                                                            manage_plays_popup.confirm_delete ^=
                                                                true;
                                                        }
                                                    } else {
                                                        if let Some(Popup::ManagePlays(
                                                            manage_plays_popup,
                                                        )) = app.drawer.active_popup.as_mut()
                                                        {
                                                            manage_plays_popup.item = 0;

                                                            let entry = &manage_plays_popup
                                                                .entry
                                                                .as_ref()
                                                                .unwrap()
                                                                .history[num_entries
                                                                - 1
                                                                - manage_plays_popup
                                                                    .scrollview
                                                                    .selected_index];

                                                            manage_plays_popup.phase =
                                                                Phase::EnterDetails;
                                                            manage_plays_popup.rating_input =
                                                                TextArea::from([format!(
                                                                    "{:.1}",
                                                                    entry.rating
                                                                )]);
                                                            manage_plays_popup.date_input =
                                                                TextArea::from([entry
                                                                    .date
                                                                    .with_timezone(&Local)
                                                                    .to_string()]);

                                                            manage_plays_popup
                                                                .rating_input
                                                                .move_cursor(
                                                                ratatui_textarea::CursorMove::End,
                                                            );
                                                            manage_plays_popup
                                                                .date_input
                                                                .move_cursor(
                                                                ratatui_textarea::CursorMove::End,
                                                            );
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                    }
                                } else if index == 2 && selected {
                                    frame.render_widget(
                                        line!("▁".repeat(area.width as usize)).fg(
                                            if tab_selected {
                                                tailwind::EMERALD.c800
                                            } else {
                                                tailwind::SLATE.c700
                                            },
                                        ),
                                        areas[i as usize],
                                    );
                                }
                            }
                        },
                    );

                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        scrollbar_area.resize(Size::new(1, 1)),
                        move |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if manage_plays_popup.scrollview.alignment_bottom
                                    && manage_plays_popup.scrollview.partially_visible
                                {
                                    manage_plays_popup.scrollview.alignment_bottom = false;
                                } else if manage_plays_popup.scrollview.scroll_pos > 0 {
                                    manage_plays_popup.scrollview.scroll_pos -= 1;
                                }
                            }
                        },
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        scrollbar_area
                            .resize(Size::new(1, 1))
                            .offset(Offset::new(0, scrollbar_area.height as i32 - 1)),
                        move |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if !manage_plays_popup.scrollview.alignment_bottom
                                    && manage_plays_popup.scrollview.partially_visible
                                {
                                    manage_plays_popup.scrollview.alignment_bottom = true;
                                } else if manage_plays_popup.scrollview.scroll_pos
                                    < num_entries.saturating_sub(
                                        manage_plays_popup.scrollview.num_visible_items,
                                    )
                                {
                                    manage_plays_popup.scrollview.scroll_pos += 1;
                                }
                            }
                        },
                    );
                }

                let add_mouse_area = widgets::action(
                    Action::new(" + ", ActionType::Default, self.tab == 1, true),
                    HorizontalAlignment::Right,
                    false,
                    helpers::add_padding(new_area, Padding::horizontal(1)),
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    add_mouse_area,
                    |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.new_play = true;

                            manage_plays_popup.phase = Phase::EnterDetails;
                            manage_plays_popup.rating_input = TextArea::default();
                            manage_plays_popup.date_input = TextArea::default();
                        }
                    },
                );

                let close_mouse_area = widgets::action(
                    Action::new(" Close ", ActionType::Normal, self.tab == 2, true),
                    HorizontalAlignment::Center,
                    true,
                    popup_area,
                    frame,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    close_mouse_area,
                    |app, _| app.drawer.close_popup(),
                );
            }
            Phase::EnterDetails => {
                let rating_valid = self.validate_rating();
                let date_valid = self.validate_input_date();

                let new_play = self.new_play;
                let one_shot = self.one_shot;
                let mut last_item = 3;

                key_event_handler.bind_esc((None, None), "Back".into(), |app, _| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        manage_plays_popup.item = 3;
                    }
                });
                if one_shot {
                    key_event_handler.bind_esc((None, Some(3)), "Close".into(), |app, _| {
                        app.drawer.close_popup();
                    });
                    key_event_handler.bind_enter((None, Some(3)), "Close".into(), |app, _| {
                        app.drawer.close_popup();
                    });

                    key_event_handler.bind_horizontal(
                        (None, Some(2)),
                        "Navigate".into(),
                        |app, data| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let Data::Direction(true, _) = data {
                                    manage_plays_popup.item = 3;
                                }
                            }
                        },
                    );
                    key_event_handler.bind_horizontal(
                        (None, Some(3)),
                        "Navigate".into(),
                        |app, data| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let Data::Direction(false, _) = data {
                                    manage_plays_popup.item = 2;
                                }
                            }
                        },
                    );

                    last_item = 2;
                } else {
                    key_event_handler.bind_esc((None, Some(3)), "Back".into(), |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.tab = 0;
                            manage_plays_popup.item = 0;
                            manage_plays_popup.confirm_delete = false;
                            manage_plays_popup.phase = Phase::Overview;
                        }
                    });
                    key_event_handler.bind_enter((None, Some(3)), "Back".into(), |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.tab = 0;
                            manage_plays_popup.item = 0;
                            manage_plays_popup.confirm_delete = false;
                            manage_plays_popup.phase = Phase::Overview;
                        }
                    });
                }

                key_event_handler.bind_tab((None, None), "Navigate".into(), move |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            Data::Direction(true, _) => {
                                manage_plays_popup.item += 1;
                                if manage_plays_popup.item > last_item {
                                    manage_plays_popup.item = 0;
                                }
                            }
                            Data::Direction(false, _) => {
                                manage_plays_popup.item =
                                    manage_plays_popup.item.checked_sub(1).unwrap_or(last_item);
                            }
                            _ => {}
                        }
                    }
                });

                if rating_valid {
                    key_event_handler.bind_enter((None, Some(0)), "Next".into(), |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.item = 1;
                        }
                    });
                    if date_valid {
                        key_event_handler.bind_enter(
                            (None, None),
                            "Confirm".into(),
                            move |app, _| {
                                if new_play {
                                    app.add_play();
                                } else {
                                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                        app.drawer.active_popup.as_ref()
                                    {
                                        if manage_plays_popup.scrollview.selected_index == 0 {
                                            app.edit_movie();
                                        } else {
                                            app.edit_movie_play();
                                        }
                                    }
                                }

                                if one_shot {
                                    app.drawer.close_popup();
                                } else {
                                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                        app.drawer.active_popup.as_mut()
                                    {
                                        manage_plays_popup.tab = 0;
                                        manage_plays_popup.item = 0;
                                        manage_plays_popup.confirm_delete = false;
                                        manage_plays_popup.phase = Phase::Overview;

                                        if manage_plays_popup.new_play {
                                            manage_plays_popup.add_play();
                                        } else {
                                            manage_plays_popup.edit_play();
                                        }
                                    }
                                }
                            },
                        );
                    }
                }

                key_event_handler.bind_vertical((None, Some(0)), "Navigate".into(), |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let Data::Direction(true, _) = data {
                            manage_plays_popup.item = 1;
                        }
                    }
                });
                key_event_handler.bind_vertical((None, Some(1)), "Navigate".into(), |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let Data::Direction(false, _) = data {
                            manage_plays_popup.item = 0;
                        }
                    }
                });

                key_event_handler.bind_input_field((None, Some(0)), "".into(), |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let Data::Key(key_event) = data {
                            let parsed = manage_plays_popup.rating_input.lines()[0]
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            if let KeyCode::Char(x) = &key_event.code {
                                if manage_plays_popup.rating_input.lines()[0].len() >= 3
                                    || parsed >= 10.0
                                {
                                    return;
                                }

                                if !x.is_ascii_digit() && *x != '.' {
                                    return;
                                }
                            }

                            manage_plays_popup.rating_input.input(key_event);
                        }
                    }
                });
                key_event_handler.bind_input_field((None, Some(1)), "".into(), |app, data| {
                    if let Some(Popup::ManagePlays(manage_plays_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if let Data::Key(key_event) = data {
                            manage_plays_popup.date_input.input(key_event);
                        }
                    }
                });

                let popup_area = widgets::window(
                    frame,
                    helpers::centered_area(11, 44, frame.area()),
                    if new_play { " Add a new play " } else { " Edit rating " },
                    true,
                );
                image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    popup_area.outer(Margin::new(1, 1)),
                    |_, _| {},
                );
                let [rating_input_area, date_input_area, _] = vertical![==3, ==3, >=1]
                    .areas(helpers::add_padding(popup_area, Padding::proportional(1)));

                let rating_input_selected = self.item == 0;
                widgets::input_field(
                    true,
                    rating_input_selected,
                    rating_valid,
                    &mut self.rating_input,
                    WrapMode::None,
                    frame,
                    rating_input_area,
                    " Rating ",
                    "Enter a rating",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    helpers::add_padding(rating_input_area, Padding::horizontal(2)),
                    |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.item = 0;
                        }
                    },
                );

                let date_input_selected = self.item == 1;
                widgets::input_field(
                    true,
                    date_input_selected,
                    date_valid,
                    &mut self.date_input,
                    WrapMode::None,
                    frame,
                    date_input_area,
                    " Watched At ",
                    "Now",
                    None,
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    helpers::add_padding(date_input_area, Padding::horizontal(2)),
                    |app, _| {
                        if let Some(Popup::ManagePlays(manage_plays_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            manage_plays_popup.item = 1;
                        }
                    },
                );

                if one_shot {
                    let actions_mouse_areas = widgets::actions(
                        [
                            Action::new(
                                " Confirm ",
                                ActionType::Default,
                                self.item == 2,
                                rating_valid && date_valid,
                            ),
                            Action::new(" Cancel ", ActionType::Critical, self.item == 3, true),
                        ],
                        HorizontalAlignment::Right,
                        true,
                        1,
                        helpers::add_padding(popup_area, Padding::right(1)),
                        frame,
                    );
                    for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if i == 1 {
                                    app.drawer.close_popup();
                                } else {
                                    if new_play {
                                        app.add_play();
                                    } else {
                                        app.edit_movie();
                                    }

                                    app.drawer.close_popup();
                                }
                            },
                        );
                    }
                } else {
                    let back_mouse_area = widgets::action(
                        Action::new(" Back ", ActionType::Normal, self.item == 3, true),
                        HorizontalAlignment::Left,
                        false,
                        popup_area,
                        frame,
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        back_mouse_area,
                        |app, _| {
                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.tab = 0;
                                manage_plays_popup.item = 0;
                                manage_plays_popup.confirm_delete = false;
                                manage_plays_popup.phase = Phase::Overview;
                            }
                        },
                    );
                    let confirm_mouse_area = widgets::action(
                        Action::new(
                            " Confirm ",
                            ActionType::Default,
                            self.item == 2,
                            rating_valid && date_valid,
                        ),
                        HorizontalAlignment::Center,
                        true,
                        helpers::add_padding(popup_area, Padding::right(1)),
                        frame,
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        confirm_mouse_area,
                        move |app, _| {
                            if new_play {
                                app.add_play();
                            } else {
                                if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                    app.drawer.active_popup.as_ref()
                                {
                                    if manage_plays_popup.scrollview.selected_index == 0 {
                                        app.edit_movie();
                                    } else {
                                        app.edit_movie_play();
                                    }
                                }
                            }

                            if let Some(Popup::ManagePlays(manage_plays_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                manage_plays_popup.tab = 0;
                                manage_plays_popup.item = 0;
                                manage_plays_popup.confirm_delete = false;
                                manage_plays_popup.phase = Phase::Overview;

                                if manage_plays_popup.new_play {
                                    manage_plays_popup.add_play();
                                } else {
                                    manage_plays_popup.edit_play();
                                }
                            }
                        },
                    );
                }
            }
        }
    }
}
