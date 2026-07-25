use std::rc::Rc;

use itertools::Itertools;
use log::info;
use ratatui::{
    Frame,
    layout::{Alignment, HorizontalAlignment, Layout, Margin, Offset, Size},
    macros::{constraints, horizontal, line, span, vertical},
    style::{
        Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Padding},
};
use strum::{EnumCount, IntoEnumIterator};

use crate::{
    helpers::{add_padding, create_popup, ellipsize_string, resize_area, static_area},
    key_event_handler::KeyEventHandler,
    popups::{PopupTrait, Popups},
    types::{FilterCriterion, FilterCriterionDiscriminants, Movie},
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct AdvancedFilterPopup {
    tab:                        usize,
    item:                       usize,
    last_popup_height:          Option<u16>,
    filter_criteria:            Vec<FilterCriterion>,
    movies:                     Vec<Movie>,
    active_criterion:           Option<FilterCriterionDiscriminants>,
    dropdown_selected_item:     Option<usize>,
    dropdown_scroll_pos:        usize,
    dropdown_num_visible_items: usize,
}

impl AdvancedFilterPopup {
    pub fn new(filter_criteria: &[FilterCriterion]) -> Self {
        Self {
            tab: 1,
            item: 0,
            filter_criteria: filter_criteria.to_vec(),
            ..Default::default()
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

        let criterion_options_lines_count =
            |criterion: &FilterCriterionDiscriminants| match criterion {
                FilterCriterionDiscriminants::Title => 3,
                FilterCriterionDiscriminants::Director => 3,
                FilterCriterionDiscriminants::Actors => 3,
                FilterCriterionDiscriminants::Genres => 3,
                FilterCriterionDiscriminants::Released => 3,
                FilterCriterionDiscriminants::DateAdded => 3,
                FilterCriterionDiscriminants::RecentlyWatched => 3,
                FilterCriterionDiscriminants::Rating => 3,
                FilterCriterionDiscriminants::UserRating => 3,
                FilterCriterionDiscriminants::Languages => 3,
                FilterCriterionDiscriminants::Country => 3,
                FilterCriterionDiscriminants::Certification => 3,
            };

        // key_event_handler.bind_enter((Some(0), None), "Edit Criterion".into(), );
        // key_event_handler.bind_key((Some(0), None), ' ', "Delete Criterion".into(), );

        key_event_handler.bind_tab((None, None), "".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.dropdown_selected_item = None;
                        advanced_filter_popup.dropdown_scroll_pos = 0;

                        advanced_filter_popup.tab += 1;
                        if advanced_filter_popup.tab > 2 {
                            advanced_filter_popup.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.dropdown_selected_item = None;
                        advanced_filter_popup.dropdown_scroll_pos = 0;

                        advanced_filter_popup.tab =
                            advanced_filter_popup.tab.checked_sub(1).unwrap_or(2);
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_enter((Some(2), Some(0)), "Confirm".into(), move |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_enter((Some(2), Some(1)), "Cancel".into(), move |app, _| {
            app.drawer.close_popup();
        });

        key_event_handler.bind_horizontal((Some(2), None), "".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        advanced_filter_popup.item += 1;
                        if advanced_filter_popup.item > 1 {
                            advanced_filter_popup.item = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        advanced_filter_popup.item =
                            advanced_filter_popup.item.checked_sub(1).unwrap_or(1);
                    }
                    _ => {}
                }
            }
        });

        if self.dropdown_selected_item.is_some() {
            key_event_handler.bind_vertical((None, None), "Choose".into(), |app, data| {
                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    match data {
                        crate::key_event_handler::Data::Direction(true, _) => {
                            advanced_filter_popup.dropdown_selected_item = advanced_filter_popup
                                .dropdown_selected_item
                                .map(|x| x + 1)
                                .inspect(|x| {
                                    if x - advanced_filter_popup.dropdown_scroll_pos
                                        >= advanced_filter_popup.dropdown_num_visible_items
                                    {
                                        advanced_filter_popup.dropdown_scroll_pos += 1
                                    }
                                });
                        }
                        crate::key_event_handler::Data::Direction(false, _) => {
                            advanced_filter_popup.dropdown_selected_item = advanced_filter_popup
                                .dropdown_selected_item
                                .map(|x| x.saturating_sub(1))
                                .inspect(|x| {
                                    if x < &advanced_filter_popup.dropdown_scroll_pos {
                                        advanced_filter_popup.dropdown_scroll_pos -= 1;
                                    }
                                });
                        }
                        _ => {}
                    }
                }
            });
        }

        let applied_height = 1;
        let options_height = if let Some(active) = self.active_criterion.as_ref() {
            criterion_options_lines_count(active) + 1
        } else {
            0
        };
        let constraints = constraints![==(applied_height + 2), ==3, ==(options_height + 2), ==2];
        let popup_height = applied_height + 2 + 3 + options_height + 2 + 2 + 2;
        if let Some(last_popup_height) = self.last_popup_height {
            if last_popup_height > popup_height {
                key_event_handler.bind_immediate(|app, _| {
                    app.drawer.refresh_images();
                });
            }
        }
        self.last_popup_height = Some(popup_height);

        let popup_area = create_popup(
            frame,
            static_area(popup_height, 55, frame.area()),
            " Advanced Filter ",
            Style::new().fg(material::YELLOW.c800),
            Alignment::Center,
            Style::new().fg(tailwind::VIOLET.c950),
            tailwind::BLUE.c950,
            true,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [applied_area, dropdown_area, options_area, _] =
            Layout::vertical(constraints).areas(add_padding(popup_area, Padding::horizontal(1)));

        {
            let tab_selected = self.tab == 0;

            let applied_block = Block::new()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(tailwind::SLATE.c900);
            let inner_applied_block = applied_block.inner(applied_area);
            frame.render_widget(Block::new().bg(tailwind::SLATE.c900), inner_applied_block);
            frame.render_widget(&applied_block, applied_area);

            if self.filter_criteria.is_empty() {
                frame.render_widget(
                    line!("None").fg(tailwind::WHITE).centered(),
                    inner_applied_block,
                );

                if tab_selected {
                    self.tab = 1;
                }
            } else {
            }
        }

        let actions_mouse_areas = widgets::actions(
            [
                Action::new(
                    " Confirm ",
                    ActionTypes::Default,
                    self.tab == 2 && self.item == 0,
                    true,
                ),
                Action::new(
                    " Cancel ",
                    ActionTypes::Critical,
                    self.tab == 2 && self.item == 1,
                    true,
                ),
            ],
            HorizontalAlignment::Center,
            true,
            1,
            add_padding(popup_area, Padding::right(1)),
            frame,
        );
        for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i == 0 {}
                    app.drawer.close_popup();
                },
            );
        }

        if let Some(active_criterion) = self.active_criterion.as_ref() {
            let tab_selected = self.tab == 1;
            let confirm_and_append_criterion: Rc<Box<dyn Fn(&mut AdvancedFilterPopup)>>;

            key_event_handler.bind_esc((Some(1), None), "Clear".into(), |app, _| {
                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.active_criterion = None;
                }
            });

            let options_block = Block::new()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_set(border::PROPORTIONAL_TALL)
                .fg(tailwind::SKY.c950)
                .bg(tailwind::SKY.c950);
            frame.render_widget(&options_block, options_area);
            let inner_area = options_block.inner(options_area);

            let actions_mouse_areas = widgets::actions(
                [
                    Action::new("  ", ActionTypes::Normal, true, true),
                    Action::new("  ", ActionTypes::Critical, true, true),
                ],
                HorizontalAlignment::Right,
                true,
                1,
                add_padding(inner_area, Padding::right(2)),
                frame,
            );
            for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                // let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            // if i == 0 {
                            //     confirm_and_append_criterion(advanced_filter_popup);
                            // }

                            advanced_filter_popup.item = 0;
                            advanced_filter_popup.active_criterion = None;
                        }
                    },
                );
            }
        }

        {
            let tab_selected = self.tab == 1;
            let selected = self.item == 0;

            if self.dropdown_selected_item.is_some() {
                key_event_handler.bind_enter((Some(1), Some(0)), "Select".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        let selected = advanced_filter_popup.dropdown_selected_item.take().unwrap();
                        advanced_filter_popup.active_criterion =
                            FilterCriterionDiscriminants::from_repr(selected);
                        advanced_filter_popup.item = 1;
                    }
                });
                key_event_handler.bind_esc((Some(1), Some(0)), "Clear".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        _ = advanced_filter_popup.dropdown_selected_item.take();
                    }
                });
            } else {
                key_event_handler.bind_enter(
                    (Some(1), Some(0)),
                    "Open Dropdown".into(),
                    |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.dropdown_scroll_pos = 0;
                            advanced_filter_popup.dropdown_num_visible_items = 5;
                            advanced_filter_popup.dropdown_selected_item = Some(0);
                        }
                    },
                );
            }

            self.dropdown_selected_item = self.dropdown_selected_item.map(|x| {
                if x >= FilterCriterionDiscriminants::COUNT {
                    self.dropdown_scroll_pos -= 1;
                    FilterCriterionDiscriminants::COUNT - 1
                } else {
                    x
                }
            });

            let [message_area, _, dropdown_area] = horizontal![==20, ==1, ==20]
                .flex(ratatui::layout::Flex::Center)
                .areas(dropdown_area);

            frame.render_widget(
                "Add a new Criterion:",
                resize_area(message_area, Offset::new(0, -2)),
            );

            widgets::dropdown(
                true,
                tab_selected && selected,
                frame,
                dropdown_area,
                &ellipsize_string(
                    self.active_criterion
                        .as_ref()
                        .map(|x| x.into())
                        .unwrap_or("None"),
                    dropdown_area.width as usize - 4,
                ),
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                dropdown_area,
                |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        advanced_filter_popup.tab = 1;
                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.dropdown_scroll_pos = 0;
                        advanced_filter_popup.dropdown_num_visible_items = 5;
                        advanced_filter_popup.dropdown_selected_item = Some(0);
                    }
                },
            );

            if tab_selected && selected {
                if let Some(index) = self.dropdown_selected_item.as_ref() {
                    let mut items: Vec<Line> = FilterCriterionDiscriminants::iter()
                        .dropping(self.dropdown_scroll_pos)
                        .take(self.dropdown_num_visible_items)
                        .map(|x| {
                            line!(
                                " ".to_string()
                                    + &ellipsize_string(x.into(), dropdown_area.width as usize - 2)
                                    + " "
                            )
                            .fg(material::INDIGO.c200)
                            .bg(material::INDIGO.c900)
                        })
                        .collect();
                    items[*index - self.dropdown_scroll_pos] = items
                        [*index - self.dropdown_scroll_pos]
                        .clone()
                        .fg(material::BLUE.c100)
                        .bg(material::LIGHT_BLUE.c900);

                    let sort_popup_area =
                        dropdown_area.offset(Offset::new(0, 2)).resize(Size::new(
                            dropdown_area.width,
                            dropdown_area.height + items.len() as u16 - 1,
                        ));
                    let sort_popup_block = Block::bordered()
                        .border_set(border::PROPORTIONAL_WIDE)
                        .fg(material::INDIGO.c900);
                    frame.render_widget(&sort_popup_block, sort_popup_area);
                    frame.render_widget(
                        Block::new().bg(material::BLUE.c600),
                        add_padding(sort_popup_area, Padding::bottom(2)),
                    );

                    let mut mouse_area = sort_popup_block.inner(sort_popup_area).resize(Size {
                        width:  sort_popup_block.inner(sort_popup_area).width,
                        height: 1,
                    });
                    for i in 0..items.len() {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    _ = advanced_filter_popup.dropdown_selected_item.take();

                                    advanced_filter_popup.active_criterion =
                                        FilterCriterionDiscriminants::from_repr(
                                            i + advanced_filter_popup.dropdown_scroll_pos,
                                        );
                                    advanced_filter_popup.item = 1;
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                    frame.render_widget(Clear, sort_popup_block.inner(sort_popup_area));
                    frame.render_widget(
                        Text::from_iter(items).left_aligned(),
                        sort_popup_block.inner(sort_popup_area),
                    );
                }
            }
        }
    }
}
