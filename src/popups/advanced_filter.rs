use std::{cmp::Ordering, rc::Rc};

use itertools::Itertools;
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Alignment, HorizontalAlignment, Layout, Margin, Offset},
    macros::{constraints, horizontal, line, vertical},
    style::{
        Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::{Block, Borders, Padding},
};
use ratatui_textarea::TextArea;
use strum::IntoEnumIterator;

use crate::{
    app::App,
    helpers::{add_padding, create_popup, ellipsize_string, resize_area, static_area},
    key_event_handler::{self, KeyEventHandler},
    pop_criterion,
    popups::{PopupTrait, Popups},
    screens::Screens,
    types::{FilterCriterion, FilterCriterionDiscriminants, Movie},
    widgets::{self, Action, ActionTypes},
};

#[derive(Default)]
pub struct AdvancedFilterPopup {
    tab:               usize,
    item:              usize,
    filter_criteria:   Vec<FilterCriterion>,
    last_popup_height: Option<u16>,
    validate:          Option<Box<dyn Fn(&Self) -> bool>>,

    available_criteria: Vec<FilterCriterionDiscriminants>,
    active_criterion:   Option<FilterCriterionDiscriminants>,

    dropdown_selected_item:     Option<usize>,
    dropdown_scroll_pos:        usize,
    dropdown_num_visible_items: usize,

    available_genres:    Vec<String>,
    available_languages: Vec<String>,

    // available_actors:    Vec<String>,
    // available_directors: Vec<String>,
    // available_countries: Vec<String>,
    input0:                   TextArea<'static>,
    input1:                   TextArea<'static>,
    dropdown0:                usize,
    dropdown0_data:           Vec<String>,
    dropdown1:                usize,
    dropdown1_data:           Vec<String>,
    dropdown1_selected_items: Vec<usize>,
    dropdown2:                usize,
    dropdown2_data:           Vec<String>,
    dropdown3:                usize,
    dropdown3_data:           Vec<String>,
    dropdown3_selected_items: Vec<usize>,
}

impl AdvancedFilterPopup {
    pub fn new(filter_criteria: &[FilterCriterion]) -> Self {
        Self {
            tab: 1,
            item: 0,
            available_criteria: FilterCriterionDiscriminants::iter()
                .filter(|x| {
                    !filter_criteria
                        .iter()
                        .any(|y| FilterCriterionDiscriminants::from(y) == *x)
                })
                .collect_vec(),
            filter_criteria: filter_criteria.to_vec(),
            dropdown_selected_item: Some(0),
            dropdown_num_visible_items: 5,
            ..Default::default()
        }
    }

    pub fn initialize(&mut self, movies: &[Movie]) {
        self.available_genres = movies
            .iter()
            .map(|x| x.genres.clone())
            .flatten()
            .unique()
            .sorted()
            .collect_vec();
        self.available_languages = movies
            .iter()
            .map(|x| x.language.clone())
            .unique()
            .sorted()
            .collect_vec();
    }

    fn init_criterion_options(&mut self) {
        let Some(criterion_discriminant) = self.active_criterion.as_ref() else {
            return;
        };
        match criterion_discriminant {
            FilterCriterionDiscriminants::Title => {
                self.input0 = TextArea::from([""]);

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    !advanced_filter_popup.input0.is_empty()
                }));
            }
            FilterCriterionDiscriminants::Director => {
                // self.dropdown0 = 0;
                // self.dropdown1 = 0;
                // self.dropdown_scroll_pos = 0;
                // self.dropdown_num_visible_items = 5;
                // self.dropdown_selected_item = Some(0);
                // self.dropdown0_data = vec!["Directed by".into(), "Not directed by".into()];
                // self.dropdown1_data = self.available_directors.clone();
                // self.dropdown1_selected_items = vec![];
                //
                // self.validate = Some(Box::new(|advanced_filter_popup| {
                //     !advanced_filter_popup.dropdown1_selected_items.is_empty()
                // }));
            }
            FilterCriterionDiscriminants::Actors => {
                // self.dropdown0 = 0;
                // self.dropdown1 = 0;
                // self.dropdown2 = 0;
                // self.dropdown3 = 0;
                // self.dropdown_scroll_pos = 0;
                // self.dropdown_num_visible_items = 5;
                // self.dropdown_selected_item = Some(0);
                // self.dropdown0_data = vec!["any of".into(), "all of".into()];
                // self.dropdown1_data = self.available_actors.clone();
                // self.dropdown1_selected_items = vec![];
                // self.dropdown2_data = vec!["any of".into(), "all of".into()];
                // self.dropdown3_data = self.available_actors.clone();
                // self.dropdown3_selected_items = vec![];
                //
                // self.validate = Some(Box::new(|advanced_filter_popup| {
                //     !advanced_filter_popup.dropdown1_selected_items.is_empty() ||
                //     !advanced_filter_popup.dropdown3_selected_items.is_empty()
                // }));
            }
            FilterCriterionDiscriminants::Genres => {
                self.dropdown0 = 0;
                self.dropdown1 = 0;
                self.dropdown2 = 0;
                self.dropdown3 = 0;
                self.dropdown_scroll_pos = 0;
                self.dropdown_num_visible_items = 5;
                self.dropdown_selected_item = Some(0);
                self.dropdown0_data = vec!["any of".into(), "all of".into()];
                self.dropdown1_data = self.available_genres.clone();
                self.dropdown1_selected_items = vec![];
                self.dropdown2_data = vec!["any of".into(), "all of".into()];
                self.dropdown3_data = self.available_genres.clone();
                self.dropdown3_selected_items = vec![];

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    !advanced_filter_popup.dropdown1_selected_items.is_empty()
                        || !advanced_filter_popup.dropdown3_selected_items.is_empty()
                }));
            }
            FilterCriterionDiscriminants::Released
            | FilterCriterionDiscriminants::FirstWatched
            | FilterCriterionDiscriminants::LastWatched => {
                self.input0 = TextArea::from([""]);
                self.input1 = TextArea::from([""]);
                self.dropdown0 = 0;
                self.dropdown_scroll_pos = 0;
                self.dropdown_num_visible_items = 4;
                self.dropdown_selected_item = Some(0);
                self.dropdown0_data = vec![
                    "In".into(),
                    "After".into(),
                    "Before".into(),
                    "Between".into(),
                ];

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    advanced_filter_popup.input0.lines()[0]
                        .parse::<usize>()
                        .map(|x| x > 1800)
                        .unwrap_or(false)
                        && (advanced_filter_popup.dropdown0 != 3
                            || advanced_filter_popup.input1.lines()[0]
                                .parse::<usize>()
                                .map(|x| x > 1800)
                                .unwrap_or(false))
                }));
            }
            FilterCriterionDiscriminants::Rating | FilterCriterionDiscriminants::UserRating => {
                self.input0 = TextArea::from([""]);
                self.dropdown0 = 0;
                self.dropdown_scroll_pos = 0;
                self.dropdown_num_visible_items = 5;
                self.dropdown_selected_item = Some(0);
                self.dropdown0_data =
                    vec!["<".into(), "<=".into(), ">".into(), ">=".into(), "=".into()];

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    advanced_filter_popup.input0.lines()[0]
                        .parse::<f64>()
                        .map(|x| x <= 10.0)
                        .unwrap_or(false)
                }));
            }
            FilterCriterionDiscriminants::Language => {
                self.dropdown0 = 0;
                self.dropdown1 = 0;
                self.dropdown_scroll_pos = 0;
                self.dropdown_num_visible_items = 4;
                self.dropdown_selected_item = Some(0);
                self.dropdown0_data = vec!["In".into(), "Not in".into()];
                self.dropdown1_data = self.available_languages.clone();

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    !advanced_filter_popup.dropdown1_selected_items.is_empty()
                }));
            }
            FilterCriterionDiscriminants::Country => {
                // self.dropdown0 = 0;
                // self.dropdown1 = 0;
                // self.dropdown_scroll_pos = 0;
                // self.dropdown_num_visible_items = 5;
                // self.dropdown_selected_item = Some(0);
                // self.dropdown0_data = vec!["From".into(), "Not from".into()];
                // self.dropdown1_data = self.available_countries.clone();
                // self.dropdown1_selected_items = vec![];
                //
                // self.validate = Some(Box::new(|advanced_filter_popup| {
                //     !advanced_filter_popup.dropdown1_selected_items.is_empty()
                // }));
            }
            FilterCriterionDiscriminants::Certification => {
                // self.dropdown0 = 0;
                // self.dropdown1 = 0;
                // self.dropdown_scroll_pos = 0;
                // self.dropdown_num_visible_items = 5;
                // self.dropdown_selected_item = Some(0);
                // self.dropdown0_data = vec!["Certified".into(), "Not certified".into()];
                // self.dropdown1_data = self.available_certifications.clone();
                // self.dropdown1_selected_items = vec![];
                //
                // self.validate = Some(Box::new(|advanced_filter_popup| {
                //     !advanced_filter_popup.dropdown1_selected_items.is_empty()
                // }));
            }
        }
    }

    fn recalculate_available_criteria(&mut self) {
        self.available_criteria = FilterCriterionDiscriminants::iter()
            .filter(|x| {
                !self
                    .filter_criteria
                    .iter()
                    .any(|y| FilterCriterionDiscriminants::from(y) == *x)
            })
            .collect_vec();
    }

    fn confirm(app: &mut App) {
        let filter_criteria = if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
            app.drawer.active_popup.as_mut()
        {
            advanced_filter_popup
                .filter_criteria
                .drain(..)
                .collect_vec()
        } else {
            unreachable!()
        };

        if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
            main_screen.filter_criteria = filter_criteria;

            if let Some(FilterCriterion::Title(name, filter)) =
                pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _))
            {
                if !name.is_empty() {
                    main_screen.search_input = TextArea::from([&name]);
                    main_screen
                        .search_input
                        .move_cursor(ratatui_textarea::CursorMove::End);
                    main_screen.sort = crate::types::Sort::Relevance;
                }

                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title(name, filter));
            }

            main_screen.filter_sort_movies(Some(true));
        }
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
                FilterCriterionDiscriminants::Actors | FilterCriterionDiscriminants::Genres => 6,
                _ => 3,
            };

        // key_event_handler.bind_enter((Some(0), None), "Edit Criterion".into(), );
        // key_event_handler.bind_key((Some(0), None), ' ', "Delete Criterion".into(), );

        key_event_handler.bind_tab((None, None), "".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        advanced_filter_popup.dropdown_selected_item = None;

                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.tab += 1;
                        if advanced_filter_popup.tab > 3 {
                            advanced_filter_popup.tab = 0;
                        }
                        if advanced_filter_popup.tab == 2
                            && advanced_filter_popup.active_criterion.is_none()
                        {
                            advanced_filter_popup.tab = 3;
                        } else if advanced_filter_popup.tab == 0
                            && advanced_filter_popup.filter_criteria.is_empty()
                        {
                            advanced_filter_popup.tab = 1;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        advanced_filter_popup.dropdown_selected_item = None;

                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.tab =
                            advanced_filter_popup.tab.checked_sub(1).unwrap_or(3);

                        if advanced_filter_popup.tab == 2
                            && advanced_filter_popup.active_criterion.is_none()
                        {
                            advanced_filter_popup.tab = 1;
                        } else if advanced_filter_popup.tab == 0
                            && advanced_filter_popup.filter_criteria.is_empty()
                        {
                            advanced_filter_popup.tab = 3;
                        }
                    }
                    _ => {}
                }
            }
        });

        key_event_handler.bind_vertical((None, None), "Choose".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                if advanced_filter_popup.dropdown_selected_item.is_some() {
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
            }
        });

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
            let this_tab = 0;
            let tab_selected = self.tab == this_tab;

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
            } else {
                frame.render_widget(
                    format!("{:?}", self.filter_criteria).fg(tailwind::WHITE),
                    inner_applied_block,
                );
            }
        }

        {
            let this_tab = 3;
            let tab_selected = self.tab == this_tab;

            key_event_handler.bind_enter(
                (Some(this_tab), Some(0)),
                "Confirm".into(),
                move |app, _| {
                    Self::confirm(app);

                    app.drawer.close_popup();
                },
            );
            key_event_handler.bind_enter(
                (Some(this_tab), Some(1)),
                "Cancel".into(),
                move |app, _| {
                    app.drawer.close_popup();
                },
            );

            key_event_handler.bind_horizontal(
                (Some(this_tab), None),
                "".into(),
                move |app, data| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                advanced_filter_popup.item += 1;
                                if advanced_filter_popup.item > 1 {
                                    advanced_filter_popup.item = 1;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                advanced_filter_popup.item =
                                    advanced_filter_popup.item.saturating_sub(1);
                            }
                            _ => {}
                        }
                    }
                },
            );

            let actions_mouse_areas = widgets::actions(
                [
                    Action::new(
                        " Confirm ",
                        ActionTypes::Default,
                        tab_selected && self.item == 0,
                        true,
                    ),
                    Action::new(
                        " Cancel ",
                        ActionTypes::Critical,
                        tab_selected && self.item == 1,
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
                        if i == 0 {
                            Self::confirm(app);
                        }
                        app.drawer.close_popup();
                    },
                );
            }
        }

        if let Some(active_criterion) = self.active_criterion.as_ref() {
            let this_tab = 2;
            let tab_selected = self.tab == this_tab;
            let confirm_and_append_criterion: Rc<Box<dyn Fn(&mut AdvancedFilterPopup)>>;

            key_event_handler.bind_esc((Some(this_tab), None), "Back".into(), |app, _| {
                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = None;
                }
            });

            key_event_handler.bind_horizontal((Some(this_tab), None), "".into(), |app, data| {
                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    match data {
                        crate::key_event_handler::Data::Direction(false, _) => {
                            advanced_filter_popup.dropdown_selected_item = None;
                            advanced_filter_popup.item =
                                advanced_filter_popup.item.saturating_sub(1);
                        }
                        crate::key_event_handler::Data::Direction(true, _) => {
                            advanced_filter_popup.dropdown_selected_item = None;
                            advanced_filter_popup.item += 1;
                        }
                        _ => (),
                    }
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
                    Action::new(
                        "  ",
                        ActionTypes::Normal,
                        true,
                        self.validate
                            .as_ref()
                            .and_then(|validate| Some(validate(self)))
                            .unwrap_or(false),
                    ),
                    Action::new("  ", ActionTypes::Critical, true, true),
                ],
                HorizontalAlignment::Right,
                true,
                1,
                add_padding(inner_area, Padding::right(2)),
                frame,
            );

            match active_criterion {
                FilterCriterionDiscriminants::Title => {
                    confirm_and_append_criterion = Rc::new(Box::new(|advanced_filter_popup| {
                        let title = advanced_filter_popup.input0.lines()[0].trim();
                        if !title.is_empty() {
                            advanced_filter_popup
                                .filter_criteria
                                .push(FilterCriterion::Title(title.to_string(), true));
                            advanced_filter_popup.recalculate_available_criteria();
                        }
                    }));

                    let input_area = add_padding(inner_area, Padding::new(2, 2, 0, 1));
                    if self.item > 0 {
                        self.item = 0;
                    }

                    if self.validate.as_ref().unwrap()(self) {
                        let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (Some(this_tab), Some(0)),
                            "".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = this_tab - 1;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                    }
                    key_event_handler.bind_input_field(
                        (Some(this_tab), Some(0)),
                        "".into(),
                        |app, data| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Key(key_event) => {
                                        advanced_filter_popup.input0.input(key_event);
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );

                    widgets::input_field(
                        tab_selected,
                        self.item == 0,
                        true,
                        &mut self.input0,
                        ratatui_textarea::WrapMode::None,
                        frame,
                        input_area,
                        " Filter ",
                        "Search",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        input_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 0;
                                advanced_filter_popup.dropdown_selected_item = None;
                            }
                        },
                    );
                }
                FilterCriterionDiscriminants::Director => {
                    confirm_and_append_criterion = Rc::new(Box::new(|advanced_filter_popup| {}));
                }
                FilterCriterionDiscriminants::Actors => {
                    confirm_and_append_criterion = Rc::new(Box::new(|advanced_filter_popup| {}));
                }
                FilterCriterionDiscriminants::Genres => {
                    confirm_and_append_criterion =
                        Rc::new(Box::new(move |advanced_filter_popup| {
                            let (genres, contains_all) = (
                                advanced_filter_popup
                                    .dropdown1_selected_items
                                    .iter()
                                    .map(|x| advanced_filter_popup.available_genres[*x].clone())
                                    .collect_vec(),
                                advanced_filter_popup.dropdown0 == 1,
                            );
                            let (inv_genres, inv_contains_all) = (
                                advanced_filter_popup
                                    .dropdown3_selected_items
                                    .iter()
                                    .map(|x| advanced_filter_popup.available_genres[*x].clone())
                                    .collect_vec(),
                                advanced_filter_popup.dropdown2 == 1,
                            );

                            if !genres.is_empty() {
                                advanced_filter_popup
                                    .filter_criteria
                                    .push(FilterCriterion::Genres(genres, contains_all, false));
                            }
                            if !inv_genres.is_empty() {
                                advanced_filter_popup.filter_criteria.push(
                                    FilterCriterion::Genres(inv_genres, inv_contains_all, true),
                                );
                            }
                        }));

                    key_event_handler.bind_enter(
                        (Some(this_tab), Some(0)),
                        "Select".into(),
                        |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.dropdown0 =
                                    advanced_filter_popup.dropdown_selected_item.take().unwrap();
                                advanced_filter_popup.item += 1;
                            }
                        },
                    );
                    key_event_handler.bind_enter(
                        (Some(this_tab), Some(2)),
                        "Select".into(),
                        |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.dropdown2 =
                                    advanced_filter_popup.dropdown_selected_item.take().unwrap();
                                advanced_filter_popup.item += 1;
                            }
                        },
                    );

                    if self.validate.as_ref().unwrap()(self) {
                        let _confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (Some(this_tab), Some(1)),
                            "Confirm".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    _confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = 1;
                                    advanced_filter_popup.item = 0;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                        let _confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (Some(this_tab), Some(3)),
                            "Confirm".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    _confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = 1;
                                    advanced_filter_popup.item = 0;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                    }

                    key_event_handler.bind_key(
                        (Some(this_tab), Some(1)),
                        ' ',
                        "Confirm".into(),
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let Some(selected) = advanced_filter_popup.dropdown_selected_item
                                {
                                    if let Some(index) = advanced_filter_popup
                                        .dropdown1_selected_items
                                        .iter()
                                        .position(|x| *x == selected)
                                    {
                                        advanced_filter_popup
                                            .dropdown1_selected_items
                                            .remove(index);
                                    } else {
                                        advanced_filter_popup
                                            .dropdown1_selected_items
                                            .push(selected);
                                    }
                                }
                            }
                        },
                    );
                    key_event_handler.bind_key(
                        (Some(this_tab), Some(3)),
                        ' ',
                        "Confirm".into(),
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let Some(selected) = advanced_filter_popup.dropdown_selected_item
                                {
                                    if let Some(index) = advanced_filter_popup
                                        .dropdown3_selected_items
                                        .iter()
                                        .position(|x| *x == selected)
                                    {
                                        advanced_filter_popup
                                            .dropdown3_selected_items
                                            .remove(index);
                                    } else {
                                        advanced_filter_popup
                                            .dropdown3_selected_items
                                            .push(selected);
                                    }
                                }
                            }
                        },
                    );

                    if self.item > 3 {
                        self.item = 3;
                    }

                    let [normal_area, inverted_area] =
                        vertical![==3; 2].areas(add_padding(inner_area, Padding::new(2, 2, 0, 1)));

                    let [
                        text_area,
                        _,
                        contains_all_dropdown_area,
                        _,
                        genres_dropdown_area,
                    ] = horizontal![==15, ==1, ==10, ==1, >=15].areas(inverted_area);

                    frame.render_widget(
                        "Doesn't contain".fg(tailwind::WHITE),
                        add_padding(text_area, Padding::top(1)),
                    );

                    let dropdown_selected = self.item == 2;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        contains_all_dropdown_area,
                        &self.dropdown2_data[self.dropdown2],
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        contains_all_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 2;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 5;
                                advanced_filter_popup.dropdown_selected_item =
                                    Some(advanced_filter_popup.dropdown2);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown2_data.len() {
                                    if (self.dropdown2_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown2_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 5;
                                Some(self.dropdown0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown2_data
                                    .iter()
                                    .map(|x| {
                                        line!(" ", x, " ")
                                            .fg(material::INDIGO.c200)
                                            .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                contains_all_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown2 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    let dropdown_selected = self.item == 3;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        genres_dropdown_area,
                        "--Genres--",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        genres_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 3;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 5;
                                advanced_filter_popup.dropdown_selected_item = Some(0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown3_data.len() {
                                    if (self.dropdown3_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown3_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 5;
                                Some(0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown3_data
                                    .iter()
                                    .map(|x| {
                                        line!(
                                            " ",
                                            ellipsize_string(
                                                x.as_ref(),
                                                genres_dropdown_area.width as usize - 2
                                            ),
                                            " "
                                        )
                                        .fg(material::INDIGO.c200)
                                        .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                genres_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                if self.dropdown3_selected_items.contains(&index) {
                                    frame.render_widget(
                                        "".bg(tailwind::RED.c500).fg(tailwind::WHITE),
                                        mouse_area.offset(Offset { x: -1, y: 0 }),
                                    );
                                }
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    let [
                        text_area,
                        _,
                        contains_all_dropdown_area,
                        _,
                        genres_dropdown_area,
                    ] = horizontal![==15, ==1, ==10, ==1, >=15].areas(normal_area);

                    frame.render_widget(
                        "Contains".fg(tailwind::WHITE).into_right_aligned_line(),
                        add_padding(text_area, Padding::top(1)),
                    );

                    let dropdown_selected = self.item == 0;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        contains_all_dropdown_area,
                        &self.dropdown0_data[self.dropdown0],
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        contains_all_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 0;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 5;
                                advanced_filter_popup.dropdown_selected_item =
                                    Some(advanced_filter_popup.dropdown0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown0_data.len() {
                                    if (self.dropdown0_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown0_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 5;
                                Some(self.dropdown0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown0_data
                                    .iter()
                                    .map(|x| {
                                        line!(" ", x, " ")
                                            .fg(material::INDIGO.c200)
                                            .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                contains_all_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    let dropdown_selected = self.item == 1;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        genres_dropdown_area,
                        "- Genres -",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        genres_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 1;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 5;
                                advanced_filter_popup.dropdown_selected_item = Some(0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown1_data.len() {
                                    if (self.dropdown1_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown1_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 5;
                                Some(0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown1_data
                                    .iter()
                                    .map(|x| {
                                        line!(
                                            " ",
                                            ellipsize_string(
                                                x.as_ref(),
                                                genres_dropdown_area.width as usize - 2
                                            ),
                                            " "
                                        )
                                        .fg(material::INDIGO.c200)
                                        .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                genres_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                if self.dropdown1_selected_items.contains(&index) {
                                    frame.render_widget(
                                        "".bg(tailwind::GREEN.c500).fg(tailwind::WHITE),
                                        mouse_area.offset(Offset { x: -1, y: 0 }),
                                    );
                                }
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }
                }
                FilterCriterionDiscriminants::Released
                | FilterCriterionDiscriminants::FirstWatched
                | FilterCriterionDiscriminants::LastWatched => {
                    let active_criterion = active_criterion.clone();
                    confirm_and_append_criterion =
                        Rc::new(Box::new(move |advanced_filter_popup| {
                            let ordering = advanced_filter_popup.dropdown0_data
                                [advanced_filter_popup.dropdown0]
                                .as_str();
                            let (lower_bound, upper_bound, inverted) = match ordering {
                                "In" => {
                                    let input0 = advanced_filter_popup.input0.lines()[0]
                                        .parse()
                                        .unwrap_or(u32::MIN);
                                    // let input1 = advanced_filter_popup.input0.lines()[0].parse().unwrap_or(u32::MAX);
                                    (input0, input0, false)
                                }
                                "After" => {
                                    let input0 = advanced_filter_popup.input0.lines()[0]
                                        .parse()
                                        .unwrap_or(u32::MIN);
                                    // let input1 = advanced_filter_popup.input0.lines()[0].parse().unwrap_or(u32::MAX);
                                    (input0, u32::MAX, false)
                                }
                                "Before" => {
                                    let input0 = advanced_filter_popup.input0.lines()[0]
                                        .parse()
                                        .unwrap_or(u32::MIN);
                                    // let input1 = advanced_filter_popup.input0.lines()[0].parse().unwrap_or(u32::MAX);

                                    (input0, u32::MAX, true)
                                }
                                "Between" => {
                                    let input0 = advanced_filter_popup.input0.lines()[0]
                                        .parse()
                                        .unwrap_or(u32::MIN);
                                    let input1 = advanced_filter_popup.input0.lines()[0]
                                        .parse()
                                        .unwrap_or(u32::MAX);

                                    (input0, input1, false)
                                }
                                _ => unreachable!(),
                            };
                            advanced_filter_popup
                                .filter_criteria
                                .push(match active_criterion {
                                    FilterCriterionDiscriminants::Released =>
                                        FilterCriterion::Released(
                                            lower_bound,
                                            upper_bound,
                                            inverted,
                                        ),
                                    FilterCriterionDiscriminants::FirstWatched =>
                                        FilterCriterion::FirstWatched(
                                            lower_bound,
                                            upper_bound,
                                            inverted,
                                        ),
                                    FilterCriterionDiscriminants::LastWatched =>
                                        FilterCriterion::LastWatched(
                                            lower_bound,
                                            upper_bound,
                                            inverted,
                                        ),
                                    _ => unreachable!(),
                                });
                        }));

                    key_event_handler.bind_enter(
                        (Some(this_tab), Some(0)),
                        "Select".into(),
                        |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.dropdown0 =
                                    advanced_filter_popup.dropdown_selected_item.take().unwrap();
                                advanced_filter_popup.item += 1;
                            }
                        },
                    );

                    if self.validate.as_ref().unwrap()(self) {
                        let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (
                                Some(this_tab),
                                Some(if self.dropdown0 == 3 { 2 } else { 1 }),
                            ),
                            "Confirm".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = 1;
                                    advanced_filter_popup.item = 0;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                    }

                    let text: &str = active_criterion.into();
                    let [
                        text_area,
                        _,
                        dropdown_area,
                        _,
                        input_area,
                        _,
                        remaining_area,
                    ] = horizontal![==(text.len() as u16), ==1, ==11, ==1, <=9, ==1, >=1]
                        .areas(add_padding(inner_area, Padding::new(2, 2, 0, 1)));

                    frame.render_widget(
                        text.fg(tailwind::WHITE),
                        add_padding(text_area, Padding::top(1)),
                    );

                    let dropdown_selected = self.item == 0;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        dropdown_area,
                        &self.dropdown0_data[self.dropdown0],
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 0;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 4;
                                advanced_filter_popup.dropdown_selected_item =
                                    Some(advanced_filter_popup.dropdown0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown0_data.len() {
                                    self.dropdown_scroll_pos -= 1;
                                    self.dropdown0_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 4;
                                Some(self.dropdown0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown0_data
                                    .iter()
                                    .take(self.dropdown_num_visible_items)
                                    .map(|x| {
                                        line!(" ", x, " ")
                                            .fg(material::INDIGO.c200)
                                            .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = i;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    if self.dropdown0 == 3 {
                        if self.item > 2 {
                            self.item = 2;
                        }

                        let [dash_area, _, input_area] =
                            horizontal![==1, ==1, <=9].areas(remaining_area);

                        frame.render_widget(
                            "-".fg(tailwind::WHITE).bold(),
                            add_padding(dash_area, Padding::top(1)),
                        );

                        let valid = self.input1.lines()[0]
                            .parse::<usize>()
                            .map(|x| x > 1800)
                            .unwrap_or(false);
                        widgets::input_field(
                            true,
                            tab_selected && self.item == 2,
                            valid,
                            &mut self.input1,
                            ratatui_textarea::WrapMode::None,
                            frame,
                            input_area,
                            " Upper ",
                            "",
                        );
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            input_area,
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    advanced_filter_popup.tab = this_tab;
                                    advanced_filter_popup.item = 2;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                }
                            },
                        );

                        key_event_handler.bind_input_field(
                            (Some(this_tab), Some(2)),
                            "".into(),
                            |app, data| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    match data {
                                        key_event_handler::Data::Key(key_event) => {
                                            if let KeyCode::Char(x) = &key_event.code {
                                                if advanced_filter_popup.input1.lines()[0].len()
                                                    >= 4
                                                {
                                                    return;
                                                }

                                                if !x.is_ascii_digit() {
                                                    return;
                                                }
                                            }

                                            advanced_filter_popup.input1.input(key_event);
                                            if advanced_filter_popup.input1.lines()[0].len() == 4 {
                                                advanced_filter_popup.input1.scroll((0, -1));
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            },
                        );
                    } else {
                        if self.item > 1 {
                            self.item = 1;
                        }
                    }

                    let valid = self.input0.lines()[0]
                        .parse::<usize>()
                        .map(|x| x > 1800)
                        .unwrap_or(false);
                    widgets::input_field(
                        true,
                        tab_selected && self.item == 1,
                        valid,
                        &mut self.input0,
                        ratatui_textarea::WrapMode::None,
                        frame,
                        input_area,
                        if self.dropdown0 == 3 { " Lower " } else { " Year " },
                        "",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        input_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 1;
                                advanced_filter_popup.dropdown_selected_item = None;
                            }
                        },
                    );

                    key_event_handler.bind_input_field(
                        (Some(this_tab), Some(1)),
                        "".into(),
                        |app, data| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Key(key_event) => {
                                        if let KeyCode::Char(x) = &key_event.code {
                                            if advanced_filter_popup.input0.lines()[0].len() >= 4 {
                                                return;
                                            }

                                            if !x.is_ascii_digit() {
                                                return;
                                            }
                                        }

                                        advanced_filter_popup.input0.input(key_event);
                                        if advanced_filter_popup.input0.lines()[0].len() == 4 {
                                            advanced_filter_popup.input0.scroll((0, -1));
                                            if advanced_filter_popup.dropdown0 == 3 {
                                                advanced_filter_popup.item += 1;
                                            }
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                FilterCriterionDiscriminants::Rating | FilterCriterionDiscriminants::UserRating => {
                    let active_criterion = active_criterion.clone();
                    confirm_and_append_criterion =
                        Rc::new(Box::new(move |advanced_filter_popup| {
                            let ordering = advanced_filter_popup.dropdown0_data
                                [advanced_filter_popup.dropdown0]
                                .as_str();
                            let (rating, ordering, inverted) = match ordering {
                                "<" => {
                                    let input0 =
                                        advanced_filter_popup.input0.lines()[0].parse().unwrap();

                                    (input0, Ordering::Less, false)
                                }
                                "<=" => {
                                    let input0 =
                                        advanced_filter_popup.input0.lines()[0].parse().unwrap();

                                    (input0, Ordering::Greater, true)
                                }
                                ">" => {
                                    let input0 =
                                        advanced_filter_popup.input0.lines()[0].parse().unwrap();

                                    (input0, Ordering::Greater, false)
                                }
                                ">=" => {
                                    let input0 =
                                        advanced_filter_popup.input0.lines()[0].parse().unwrap();

                                    (input0, Ordering::Less, true)
                                }
                                "=" => {
                                    let input0 =
                                        advanced_filter_popup.input0.lines()[0].parse().unwrap();

                                    (input0, Ordering::Equal, false)
                                }
                                _ => unreachable!(),
                            };
                            advanced_filter_popup
                                .filter_criteria
                                .push(match active_criterion {
                                    FilterCriterionDiscriminants::Rating =>
                                        FilterCriterion::Rating(rating, ordering, inverted),
                                    FilterCriterionDiscriminants::UserRating =>
                                        FilterCriterion::UserRating(rating, ordering, inverted),
                                    _ => unreachable!(),
                                });
                        }));

                    key_event_handler.bind_enter(
                        (Some(this_tab), Some(0)),
                        "Select".into(),
                        |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.dropdown0 =
                                    advanced_filter_popup.dropdown_selected_item.take().unwrap();
                                advanced_filter_popup.item += 1;
                            }
                        },
                    );

                    if self.validate.as_ref().unwrap()(self) {
                        let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (Some(this_tab), Some(1)),
                            "Confirm".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = 1;
                                    advanced_filter_popup.item = 0;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                    }

                    let text: &str = active_criterion.into();
                    let [text_area, _, dropdown_area, _, input_area] =
                        horizontal![==(text.len() as u16), ==1, ==6, ==1, <=8]
                            .areas(add_padding(inner_area, Padding::new(2, 2, 0, 1)));

                    frame.render_widget(
                        text.fg(tailwind::WHITE),
                        add_padding(text_area, Padding::top(1)),
                    );

                    let dropdown_selected = self.item == 0;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        dropdown_area,
                        &self.dropdown0_data[self.dropdown0],
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 0;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 5;
                                advanced_filter_popup.dropdown_selected_item =
                                    Some(advanced_filter_popup.dropdown0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown0_data.len() {
                                    self.dropdown_scroll_pos -= 1;
                                    self.dropdown0_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 5;
                                Some(self.dropdown0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown0_data
                                    .iter()
                                    .take(self.dropdown_num_visible_items)
                                    .map(|x| {
                                        line!(" ", x)
                                            .fg(material::INDIGO.c200)
                                            .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = i;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    if self.item > 1 {
                        self.item = 1;
                    }
                    let valid = self.input0.lines()[0]
                        .parse::<f64>()
                        .map(|x| x <= 10.0)
                        .unwrap_or(false);
                    widgets::input_field(
                        true,
                        tab_selected && self.item == 1,
                        valid,
                        &mut self.input0,
                        ratatui_textarea::WrapMode::None,
                        frame,
                        input_area,
                        "Rating",
                        "",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        input_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 1;
                                advanced_filter_popup.dropdown_selected_item = None;
                            }
                        },
                    );

                    key_event_handler.bind_input_field(
                        (Some(this_tab), Some(1)),
                        "".into(),
                        |app, data| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                match data {
                                    key_event_handler::Data::Key(key_event) => {
                                        let parsed = advanced_filter_popup.input0.lines()[0]
                                            .parse::<f64>()
                                            .unwrap_or(0.0);
                                        if let KeyCode::Char(x) = &key_event.code {
                                            if advanced_filter_popup.input0.lines()[0].len() >= 3
                                                || parsed >= 10.0
                                            {
                                                return;
                                            }

                                            if !x.is_ascii_digit() && *x != '.' {
                                                return;
                                            }
                                        }

                                        advanced_filter_popup.input0.input(key_event);
                                        if advanced_filter_popup.input0.lines()[0].len() == 3 {
                                            advanced_filter_popup.input0.scroll((0, -1));
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                FilterCriterionDiscriminants::Language => {
                    confirm_and_append_criterion =
                        Rc::new(Box::new(move |advanced_filter_popup| {
                            let (languages, inverted) = (
                                advanced_filter_popup
                                    .dropdown1_selected_items
                                    .iter()
                                    .map(|x| advanced_filter_popup.available_languages[*x].clone())
                                    .collect_vec(),
                                advanced_filter_popup.dropdown0 == 1,
                            );

                            if !languages.is_empty() {
                                advanced_filter_popup
                                    .filter_criteria
                                    .push(FilterCriterion::Language(languages, inverted));
                            }
                        }));

                    key_event_handler.bind_enter(
                        (Some(this_tab), Some(0)),
                        "Select".into(),
                        |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.dropdown0 =
                                    advanced_filter_popup.dropdown_selected_item.take().unwrap();
                                advanced_filter_popup.item += 1;
                            }
                        },
                    );

                    if self.validate.as_ref().unwrap()(self) {
                        let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                        key_event_handler.bind_enter(
                            (Some(this_tab), Some(1)),
                            "Confirm".into(),
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    confirm_and_append_criterion(advanced_filter_popup);

                                    advanced_filter_popup.tab = 1;
                                    advanced_filter_popup.item = 0;
                                    advanced_filter_popup.dropdown_selected_item = None;
                                    advanced_filter_popup.active_criterion = None;
                                }
                            },
                        );
                    }

                    key_event_handler.bind_key(
                        (Some(this_tab), Some(1)),
                        ' ',
                        "Confirm".into(),
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let Some(selected) = advanced_filter_popup.dropdown_selected_item
                                {
                                    if let Some(index) = advanced_filter_popup
                                        .dropdown1_selected_items
                                        .iter()
                                        .position(|x| *x == selected)
                                    {
                                        advanced_filter_popup
                                            .dropdown1_selected_items
                                            .remove(index);
                                    } else {
                                        advanced_filter_popup
                                            .dropdown1_selected_items
                                            .push(selected);
                                    }
                                }
                            }
                        },
                    );

                    if self.item > 1 {
                        self.item = 1;
                    }

                    let [inverted_dropdown_area, _, languages_dropdown_area] =
                        horizontal![==10, ==1, <=17]
                            .areas(add_padding(inner_area, Padding::new(2, 2, 0, 1)));

                    let dropdown_selected = self.item == 0;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        inverted_dropdown_area,
                        &self.dropdown0_data[self.dropdown0],
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        inverted_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 0;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 4;
                                advanced_filter_popup.dropdown_selected_item =
                                    Some(advanced_filter_popup.dropdown0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown0_data.len() {
                                    if (self.dropdown0_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown0_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 4;
                                Some(self.dropdown0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown0_data
                                    .iter()
                                    .map(|x| {
                                        line!(" ", x, " ")
                                            .fg(material::INDIGO.c200)
                                            .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                inverted_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }

                    let dropdown_selected = self.item == 1;
                    widgets::dropdown(
                        true,
                        tab_selected && dropdown_selected,
                        frame,
                        languages_dropdown_area,
                        "--Languages--",
                    );
                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        languages_dropdown_area,
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                advanced_filter_popup.tab = this_tab;
                                advanced_filter_popup.item = 1;
                                advanced_filter_popup.dropdown_scroll_pos = 0;
                                advanced_filter_popup.dropdown_num_visible_items = 4;
                                advanced_filter_popup.dropdown_selected_item = Some(0);
                            }
                        },
                    );
                    if tab_selected && dropdown_selected {
                        self.dropdown_selected_item = self
                            .dropdown_selected_item
                            .map(|x| {
                                if x >= self.dropdown1_data.len() {
                                    if (self.dropdown1_data.len() - 1)
                                        .saturating_sub(self.dropdown_scroll_pos)
                                        < self.dropdown_num_visible_items
                                    {
                                        self.dropdown_scroll_pos =
                                            self.dropdown_scroll_pos.saturating_sub(1);
                                    }
                                    self.dropdown1_data.len() - 1
                                } else {
                                    x
                                }
                            })
                            .or_else(|| {
                                self.dropdown_scroll_pos = 0;
                                self.dropdown_num_visible_items = 4;
                                Some(0)
                            });

                        if let Some(index) = self.dropdown_selected_item.as_ref() {
                            let (mut mouse_area, len) = widgets::dropdown_popup(
                                self.dropdown1_data
                                    .iter()
                                    .map(|x| {
                                        line!(ellipsize_string(
                                            x.as_ref(),
                                            languages_dropdown_area.width as usize - 2
                                        ))
                                        .centered()
                                        .fg(material::INDIGO.c200)
                                        .bg(material::INDIGO.c900)
                                    })
                                    .collect_vec(),
                                *index,
                                self.dropdown_scroll_pos,
                                self.dropdown_num_visible_items,
                                languages_dropdown_area,
                                frame,
                            );
                            for i in 0..len {
                                let index = i + self.dropdown_scroll_pos;
                                key_event_handler.bind_mouse_button_down(
                                    ratatui::crossterm::event::MouseButton::Left,
                                    mouse_area,
                                    move |app, _| {
                                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                            app.drawer.active_popup.as_mut()
                                        {
                                            _ = advanced_filter_popup.dropdown_selected_item.take();

                                            advanced_filter_popup.dropdown0 = index;
                                            advanced_filter_popup.item += 1;
                                        }
                                    },
                                );
                                if self.dropdown1_selected_items.contains(&index) {
                                    frame.render_widget(
                                        if self.dropdown0 == 0 {
                                            "".bg(tailwind::GREEN.c500)
                                        } else {
                                            "".bg(tailwind::RED.c600)
                                        }
                                        .fg(tailwind::WHITE),
                                        mouse_area.offset(Offset { x: -1, y: 0 }),
                                    );
                                }
                                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                            }
                        }
                    }
                }
                FilterCriterionDiscriminants::Country => {
                    confirm_and_append_criterion = Rc::new(Box::new(|advanced_filter_popup| {}));
                }
                FilterCriterionDiscriminants::Certification => {
                    confirm_and_append_criterion = Rc::new(Box::new(|advanced_filter_popup| {}));
                }
            }

            for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate().dropping(
                if self
                    .validate
                    .as_ref()
                    .and_then(|validate| Some(validate(self)))
                    .unwrap_or(false)
                {
                    0
                } else {
                    1
                },
            ) {
                let confirm_and_append_criterion = confirm_and_append_criterion.clone();
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if i == 0 {
                                confirm_and_append_criterion(advanced_filter_popup);
                            }

                            advanced_filter_popup.tab = 1;
                            advanced_filter_popup.item = 0;
                            advanced_filter_popup.dropdown_selected_item = None;
                            advanced_filter_popup.active_criterion = None;
                        }
                    },
                );
            }
        }

        {
            let this_tab = 1;
            let tab_selected = self.tab == this_tab;

            if self.dropdown_selected_item.is_some() {
                key_event_handler.bind_enter(
                    (Some(this_tab), None),
                    "Select".into(),
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.item = 0;
                            advanced_filter_popup.tab = 2;

                            let selected =
                                advanced_filter_popup.dropdown_selected_item.take().unwrap();
                            advanced_filter_popup.active_criterion =
                                Some(advanced_filter_popup.available_criteria[selected]);
                            advanced_filter_popup.init_criterion_options();
                        }
                    },
                );
            } else {
                key_event_handler.bind_enter(
                    (Some(this_tab), None),
                    "Open Dropdown".into(),
                    |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.dropdown_selected_item = Some(
                                if let Some(x) = advanced_filter_popup.active_criterion.as_ref() {
                                    *x as usize
                                } else {
                                    0
                                },
                            );
                            advanced_filter_popup.dropdown_num_visible_items = 5;
                            advanced_filter_popup.dropdown_scroll_pos = advanced_filter_popup
                                .dropdown_selected_item
                                .as_ref()
                                .unwrap()
                                .saturating_sub(
                                    advanced_filter_popup.dropdown_num_visible_items - 1,
                                );
                        }
                    },
                );
            }

            if self.dropdown_selected_item.is_some() {
                key_event_handler.bind_esc((Some(this_tab), None), "Close".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        _ = advanced_filter_popup.dropdown_selected_item.take();
                    }
                });
            } else if self.active_criterion.is_some() {
                key_event_handler.bind_esc((Some(this_tab), None), "Clear".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        advanced_filter_popup.active_criterion = None;
                    }
                });
            }

            let [message_area, _, dropdown_area] = horizontal![==20, ==1, ==20]
                .flex(ratatui::layout::Flex::Center)
                .areas(dropdown_area);

            frame.render_widget(
                "Add a new Criterion:",
                resize_area(message_area, Offset::new(0, -2)),
            );

            let selected = self.item == 0;
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
                move |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        advanced_filter_popup.tab = this_tab;
                        advanced_filter_popup.item = 0;

                        advanced_filter_popup.dropdown_selected_item = Some(
                            if let Some(x) = advanced_filter_popup.active_criterion.as_ref() {
                                *x as usize
                            } else {
                                0
                            },
                        );
                        advanced_filter_popup.dropdown_num_visible_items = 5;
                        advanced_filter_popup.dropdown_scroll_pos = advanced_filter_popup
                            .dropdown_selected_item
                            .as_ref()
                            .unwrap()
                            .saturating_sub(advanced_filter_popup.dropdown_num_visible_items - 1);
                    }
                },
            );

            if tab_selected && selected {
                self.dropdown_selected_item = self.dropdown_selected_item.map(|x| {
                    if x >= self.available_criteria.len() {
                        self.dropdown_scroll_pos -= 1;
                        self.available_criteria.len() - 1
                    } else {
                        x
                    }
                });

                if let Some(index) = self.dropdown_selected_item.as_ref() {
                    let criterion_to_line =
                        |criterion: &FilterCriterionDiscriminants| match criterion {
                            FilterCriterionDiscriminants::Title
                            | FilterCriterionDiscriminants::Director
                            | FilterCriterionDiscriminants::Released
                            | FilterCriterionDiscriminants::FirstWatched
                            | FilterCriterionDiscriminants::LastWatched
                            | FilterCriterionDiscriminants::Rating
                            | FilterCriterionDiscriminants::UserRating
                            | FilterCriterionDiscriminants::Country => line!(
                                " ",
                                ellipsize_string(
                                    criterion.into(),
                                    dropdown_area.width as usize - 2,
                                ),
                                " "
                            )
                            .fg(material::INDIGO.c200),

                            FilterCriterionDiscriminants::Actors =>
                                if !self
                                    .filter_criteria
                                    .iter()
                                    .any(|y| matches!(y, FilterCriterion::Genres(_, _, true)))
                                {
                                    "+"
                                } else {
                                    " "
                                }
                                .green()
                                    + ellipsize_string(
                                        criterion.into(),
                                        dropdown_area.width as usize - 2,
                                    )
                                    .fg(material::INDIGO.c200)
                                    + if !self
                                        .filter_criteria
                                        .iter()
                                        .any(|y| matches!(y, FilterCriterion::Genres(_, _, false)))
                                    {
                                        "-"
                                    } else {
                                        " "
                                    }
                                    .red(),
                            FilterCriterionDiscriminants::Genres =>
                                if !self
                                    .filter_criteria
                                    .iter()
                                    .any(|y| matches!(y, FilterCriterion::Genres(_, _, true)))
                                {
                                    "+"
                                } else {
                                    " "
                                }
                                .green()
                                    + ellipsize_string(
                                        criterion.into(),
                                        dropdown_area.width as usize - 2,
                                    )
                                    .fg(material::INDIGO.c200)
                                    + if !self
                                        .filter_criteria
                                        .iter()
                                        .any(|y| matches!(y, FilterCriterion::Genres(_, _, false)))
                                    {
                                        "-"
                                    } else {
                                        " "
                                    }
                                    .red(),
                            FilterCriterionDiscriminants::Language =>
                                if !self
                                    .filter_criteria
                                    .iter()
                                    .any(|y| matches!(y, FilterCriterion::Genres(_, _, true)))
                                {
                                    "+"
                                } else {
                                    " "
                                }
                                .green()
                                    + ellipsize_string(
                                        criterion.into(),
                                        dropdown_area.width as usize - 2,
                                    )
                                    .fg(material::INDIGO.c200)
                                    + if !self
                                        .filter_criteria
                                        .iter()
                                        .any(|y| matches!(y, FilterCriterion::Genres(_, _, false)))
                                    {
                                        "-"
                                    } else {
                                        " "
                                    }
                                    .red(),
                            FilterCriterionDiscriminants::Certification =>
                                if !self
                                    .filter_criteria
                                    .iter()
                                    .any(|y| matches!(y, FilterCriterion::Genres(_, _, true)))
                                {
                                    "+"
                                } else {
                                    " "
                                }
                                .green()
                                    + ellipsize_string(
                                        criterion.into(),
                                        dropdown_area.width as usize - 2,
                                    )
                                    .fg(material::INDIGO.c200)
                                    + if !self
                                        .filter_criteria
                                        .iter()
                                        .any(|y| matches!(y, FilterCriterion::Genres(_, _, false)))
                                    {
                                        "-"
                                    } else {
                                        " "
                                    }
                                    .red(),
                        };

                    let (mut mouse_area, len) = widgets::dropdown_popup(
                        self.available_criteria
                            .iter()
                            .map(|x| criterion_to_line(x))
                            .collect_vec(),
                        *index,
                        self.dropdown_scroll_pos,
                        self.dropdown_num_visible_items,
                        dropdown_area,
                        frame,
                    );
                    for i in 0..len {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    advanced_filter_popup.tab = 2;
                                    advanced_filter_popup.item = 0;

                                    _ = advanced_filter_popup.dropdown_selected_item.take();
                                    advanced_filter_popup.active_criterion = Some(
                                        advanced_filter_popup.available_criteria
                                            [i + advanced_filter_popup.dropdown_scroll_pos],
                                    );
                                    advanced_filter_popup.init_criterion_options();
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                }
            }
        }
    }
}
