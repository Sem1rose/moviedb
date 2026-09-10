use ratatui::{
    Frame,
    layout::{Offset, Rect},
    macros::{horizontal, line},
    style::{
        Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::Block,
};
use ratatui_textarea::TextArea;
use strum::IntoEnumIterator;

use crate::{
    helpers,
    key_event_handler::{self, KeyEventHandler},
    pop_criterion,
    screens::{Screens, main_screen::MainScreen},
    types::{FilterCriterion, RatingSource, Sort},
    widgets,
};

impl MainScreen {
    pub fn render_header(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) -> Rect {
        let tab_selected = self.tab == 2;

        key_event_handler.bind_esc((Some(2), Some(0)), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                if let Sort::Relevance = main_screen.sort {
                    main_screen.sort = *main_screen.get_available_sort_options().first().unwrap();
                }

                main_screen.search_input = TextArea::from([""]);
                let FilterCriterion::Title(_, filter) = pop_criterion!(
                    main_screen.filter_criteria,
                    FilterCriterion::Title(_, _),
                    FilterCriterion::Title(String::new(), false)
                ) else {
                    unreachable!()
                };
                if filter {
                    main_screen.filter_sort_movies(true);
                }
            }
        });
        key_event_handler.bind_esc((Some(2), None), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_tab((Some(2), None), "Change focus".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if main_screen.item == 0 {
                    let FilterCriterion::Title(name, filter) = pop_criterion!(
                        main_screen.filter_criteria,
                        FilterCriterion::Title(_, _),
                        FilterCriterion::Title(String::new(), false)
                    ) else {
                        unreachable!()
                    };

                    if name.is_empty() || !filter {
                        if let Sort::Relevance = main_screen.sort {
                            main_screen.sort = *main_screen.get_available_sort_options().first().unwrap();
                        }
                        main_screen.search_input = TextArea::from([""]);
                    } else if filter {
                        main_screen
                            .filter_criteria
                            .push(FilterCriterion::Title(name, true));
                    }
                } else {
                    // main_screen.sort = Sort::Relevance;
                    main_screen.search_input = TextArea::from([""]);
                    _ = pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _));
                    main_screen
                        .filter_criteria
                        .push(FilterCriterion::Title("".into(), true));
                    main_screen.filter_sort_movies(true);
                }

                match data {
                    key_event_handler::Data::Direction(true, _) => {
                        main_screen.item += 1;
                        if main_screen.item > 2 {
                            main_screen.item = 0;
                        }
                    }
                    key_event_handler::Data::Direction(false, _) => {
                        main_screen.item = main_screen.item.checked_sub(1).unwrap_or(2);
                    }
                    _ => (),
                }
            }
        });

        key_event_handler.bind_enter((Some(2), Some(0)), "Confirm".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                let FilterCriterion::Title(name, filter) = pop_criterion!(
                    main_screen.filter_criteria,
                    FilterCriterion::Title(_, _),
                    FilterCriterion::Title(String::new(), false)
                ) else {
                    unreachable!()
                };

                if name.is_empty() || !filter {
                    if let Sort::Relevance = main_screen.sort {
                        main_screen.sort = *main_screen.get_available_sort_options().first().unwrap();
                    }
                    main_screen.search_input = TextArea::from([""]);
                } else if filter {
                    main_screen
                        .filter_criteria
                        .push(FilterCriterion::Title(name, true));
                }
            }
        });
        key_event_handler.bind_enter((Some(2), None), "Confirm".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        key_event_handler.bind_key((Some(2), Some(1)), ',', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_key((Some(2), Some(2)), ',', "Sort".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.item = 1;
                // main_screen.sort_popup.reset_state();
            }
        });
        key_event_handler.bind_key(
            (Some(2), Some(2)),
            ' ',
            "Toggle sort order".into(),
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.sort_ascending = !main_screen.sort_ascending;
                    main_screen.filter_sort_movies(true);
                }
            },
        );
        key_event_handler.bind_key((Some(2), Some(1)), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_key((Some(2), Some(2)), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });

        if matches!(self.sort, Sort::Rating(_)) {
            key_event_handler.bind_horizontal(
                (Some(2), Some(1)),
                if self.sort_popup.opened_submenu.is_none() {
                    "Open submenu/Navigate"
                } else {
                    "Close submenu"
                }
                .into(),
                |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            key_event_handler::Data::Direction(false, _) => {
                                main_screen.sort_popup.open_submenu(true);
                            }
                            key_event_handler::Data::Direction(true, _) => {
                                if main_screen.sort_popup.opened_submenu.is_some() {
                                    main_screen.sort_popup.close_submenu();
                                } else {
                                    main_screen.item += 1;
                                }
                            }
                            _ => (),
                        }
                        main_screen.filter_sort_movies(true);
                    }
                },
            );
        } else {
            key_event_handler.bind_horizontal(
                (Some(2), Some(1)),
                "Navigate".into(),
                |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        if let key_event_handler::Data::Direction(true, _) = data {
                            main_screen.item += 1;
                        }
                    }
                },
            );
        }
        key_event_handler.bind_horizontal((Some(2), Some(2)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if let key_event_handler::Data::Direction(false, _) = data {
                    main_screen.item -= 1;
                }
            }
        });

        key_event_handler.bind_vertical(
            (Some(2), Some(1)),
            if self.sort_popup.opened_submenu.is_none() {
                "Change sort"
            } else {
                "Change rating source"
            }
            .into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    if let key_event_handler::Data::Direction(direction, _) = data {
                        main_screen.sort_popup.scroll(direction);

                        main_screen.sort = if let Some(submenu_id) =
                            main_screen.sort_popup.opened_submenu.as_ref()
                        {
                            Sort::Rating(
                                RatingSource::from_repr(
                                    main_screen.sort_popup.submenus[submenu_id].id_from_index(
                                        main_screen.sort_popup.submenus[submenu_id].selected_index,
                                    ),
                                )
                                .unwrap(),
                            )
                        } else {
                            Sort::from_repr(
                                main_screen
                                    .sort_popup
                                    .id_from_index(main_screen.sort_popup.selected_index),
                            )
                            .unwrap()
                        };
                    }
                    main_screen.filter_sort_movies(true);
                }
            },
        );
        key_event_handler.bind_vertical(
            (Some(2), Some(2)),
            "Change sort order".into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        key_event_handler::Data::Direction(false, _) => {
                            if !main_screen.sort_ascending {
                                main_screen.sort_ascending = true;
                                main_screen.filter_sort_movies(true);
                            }
                        }
                        key_event_handler::Data::Direction(true, _)
                            if main_screen.sort_ascending =>
                        {
                            main_screen.sort_ascending = false;
                            main_screen.filter_sort_movies(true);
                        }
                        _ => (),
                    }
                }
            },
        );

        key_event_handler.bind_input_field((Some(2), Some(0)), "".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                if let key_event_handler::Data::Key(key_event) = data {
                    main_screen.search_input.input(key_event);

                    let FilterCriterion::Title(_, filter) = pop_criterion!(
                        main_screen.filter_criteria,
                        FilterCriterion::Title(_, _),
                        FilterCriterion::Title(String::new(), false)
                    ) else {
                        unreachable!()
                    };
                    main_screen.sort = if filter && !main_screen.search_input.is_empty() {
                        Sort::Relevance
                    } else {
                        *main_screen.get_available_sort_options().first().unwrap()
                    };
                    main_screen.filter_criteria.push(FilterCriterion::Title(
                        main_screen.search_input.lines()[0].clone(),
                        filter,
                    ));
                    if filter {
                        main_screen.filter_sort_movies(false);
                    } else {
                        main_screen.find_and_goto_movie();
                    }
                }
            }
        });

        let sort_max_width = Sort::iter().map(|x| x.as_ref().len()).max().unwrap() + 4;

        let [_, input_area, _, sort_area, _, direction_area, _] =
            horizontal![>=1, <=25, ==1, ==sort_max_width as u16, ==1, ==3, ==1].areas(area);

        let filter = if let Some(FilterCriterion::Title(n, f)) =
            pop_criterion!(self.filter_criteria, FilterCriterion::Title(_, _))
        {
            if tab_selected || f {
                self.filter_criteria
                    .push(FilterCriterion::Title(n.clone(), f));
            } else {
                self.search_input = TextArea::from([""]);
            }

            if (tab_selected && self.item == 0) || !n.is_empty() {
                Some(f)
            } else {
                None
            }
        } else {
            None
        };

        widgets::input_field(
            tab_selected,
            self.item == 0,
            true,
            &mut self.search_input,
            ratatui_textarea::WrapMode::None,
            frame,
            input_area,
            match filter {
                Some(true) => " Filter ",
                Some(false) => " Find ",
                None => "",
            },
            "Search",
            None,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            input_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 0;

                    let filter = if let Some(FilterCriterion::Title(n, f)) =
                        pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _))
                    {
                        main_screen
                            .filter_criteria
                            .push(FilterCriterion::Title(n, f));
                        true
                    } else {
                        false
                    };

                    if !filter {
                        main_screen.search_input = TextArea::from([""]);
                        let FilterCriterion::Title(_, _) = pop_criterion!(
                            main_screen.filter_criteria,
                            FilterCriterion::Title(_, _),
                            FilterCriterion::Title(String::new(), false)
                        ) else {
                            unreachable!()
                        };

                        main_screen
                            .filter_criteria
                            .push(FilterCriterion::Title("".into(), true));
                        main_screen.filter_sort_movies(true);
                    }
                }
            },
        );

        widgets::dropdown(
            tab_selected,
            self.item == 1,
            frame,
            sort_area,
            helpers::ellipsize_string(self.sort.as_ref(), sort_area.width as usize - 4),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            sort_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 1;
                    // main_screen.sort_popup.reset_state();
                }
            },
        );

        if tab_selected && self.item == 1 {
            let items = self.get_available_sort_options();

            self.sort_popup.model = items
                .iter()
                .map(|&x| {
                    (
                        <usize>::from(x),
                        helpers::ellipsize_string(x.as_ref(), sort_area.width as usize - 2),
                    )
                })
                .collect();

            self.sort_popup.selected_index = self
                .sort_popup
                .index_from_id(<usize>::from(self.sort))
                .unwrap_or_default();
            if let Sort::Rating(source) = self.sort {
                self.sort_popup
                    .submenus
                    .get_mut(&<usize>::from(Sort::Rating(Default::default())))
                    .unwrap()
                    .selected_index = source as usize;
            };

            let areas = self
                .sort_popup
                .render_dropdown(sort_area, frame, key_event_handler);

            for (k, (mut mouse_area, len)) in areas {
                if k.is_empty() {
                    let scroll_pos = self.sort_popup.scroll_pos;
                    for i in 0..len {
                        let index = i + scroll_pos;
                        let repr = *self.sort_popup.model.get_index(index).unwrap().0;
                            key_event_handler.bind_mouse_button_down(
                                ratatui::crossterm::event::MouseButton::Left,
                                mouse_area,
                                move |app, _| {
                                    if let Some(Screens::MainScreen(main_screen)) =
                                        app.drawer.current_screen.as_mut()
                                    {
                                        let new_sort = Sort::from_repr(repr).unwrap();

                                        if main_screen.sort_popup.selected_index != index {
                                            main_screen.sort = new_sort;
                                            main_screen.sort_popup.selected_index = index;
                                            main_screen.filter_sort_movies(true);
                                        }

                                        if !matches!(new_sort, Sort::Rating(_)) {
                                            main_screen.tab = 0;
                                            main_screen.item = 0;
                                        } else {
                                            main_screen.sort_popup.open_submenu(true);
                                        }
                                    }
                                },
                            );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                } else {
                    let scroll_pos = self
                        .sort_popup
                        .submenus
                        .get_mut(&<usize>::from(Sort::Rating(Default::default())))
                        .unwrap()
                        .scroll_pos;
                    for i in 0..len {
                        let index = i + scroll_pos;
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Screens::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.tab = 0;
                                    main_screen.item = 0;
                                    main_screen.sort =
                                        Sort::Rating(RatingSource::from_repr(index).unwrap());
                                    // main_screen.sort_popup.reset_state();
                                    main_screen.filter_sort_movies(true);
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                }
            }
        }

        let direction_block =
            Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(if tab_selected {
                    if self.item == 2 {
                        material::BLUE.c600
                    } else {
                        material::INDIGO.c800
                    }
                } else {
                    tailwind::SLATE.c700
                });
        let direction = if self.sort_ascending { "⬆" } else { "⬇" };
        frame.render_widget(&direction_block, direction_area);
        frame.render_widget(
            line!(direction)
                .centered()
                .bold()
                .fg(if tab_selected {
                    if self.item == 2 {
                        material::TEAL.c100
                    } else {
                        material::INDIGO.c200
                    }
                } else {
                    material::GRAY.c400
                })
                .bg(if tab_selected {
                    if self.item == 2 {
                        material::BLUE.c600
                    } else {
                        material::INDIGO.c800
                    }
                } else {
                    tailwind::SLATE.c700
                }),
            direction_block.inner(direction_area),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            direction_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.sort_ascending = !main_screen.sort_ascending;
                    main_screen.filter_sort_movies(true);
                }
            },
        );

        sort_area
    }
}
