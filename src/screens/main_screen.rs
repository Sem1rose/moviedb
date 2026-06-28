use std::{cmp::Ordering, ops::Add, path::PathBuf};

use itertools::Itertools;
use nucleo_matcher::{Config, Matcher, pattern::Atom};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use ratatui::{
    crossterm::event::KeyModifiers,
    layout::Offset,
    macros::{constraint, horizontal, line, span, text, vertical},
    prelude::*,
    style::{
        Styled,
        palette::{material, tailwind},
    },
    symbols::{block, border, scrollbar::Set},
    widgets::{Block, Clear, Padding, Paragraph, Scrollbar, ScrollbarState, Wrap},
};
use ratatui_image::sliced::SignedPosition;
use ratatui_textarea::TextArea;

use crate::{
    KeyEventHandler,
    helpers::{add_padding, ellipsize_string, wrap_text},
    image_backend::RatatuiImage,
    screens::Screens,
    types::{Movie, Rating},
};

#[derive(FromPrimitive, ToPrimitive, Default, Clone, Copy)]
pub enum Sort {
    #[default]
    AddedDate,
    UserRating,
    Rating,
    Name,
    ReleaseDate,
    Relevance,
}

#[derive(Default)]
pub struct PlaysTab {
    scroll_pos:        usize,
    alignment_bottom:  bool,
    num_visible_items: usize,
}

pub struct MainScreen {
    tab:                usize,
    item:               usize,
    filter:             bool,
    pub sort:           Sort,
    pub redraw_images:  u8,
    pub drawing_images: bool,
    pub sort_ascending: bool,
    search_input:       TextArea<'static>,

    movies:             Vec<Movie>,
    filtered_movies:    Vec<Movie>,
    pub image_renderer: RatatuiImage,

    movies_list_scroll_pos:             usize,
    movies_list_selected_item:          usize,
    movies_list_alignment_bottom:       bool,
    movies_list_partially_visible_item: bool,
    movies_list_num_visible_items:      usize,
    movies_description_selected_tab:    usize,

    context_menu_pos:      Option<Position>,
    context_menu_selected: usize,

    movies_description_plays_tab:       PlaysTab,
    movies_description_overview_scroll: usize,
}

const MOVIE_WIDGET_HEIGHT: usize = 10;

impl MainScreen {
    pub fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (
            Some(self.tab),
            Some(
                self.item
                    + if self.tab == 1 {
                        self.movies_description_selected_tab << 9
                    } else {
                        0
                    },
            ),
        )
    }

    pub fn new(cache_dir: &PathBuf) -> Self {
        Self {
            tab:            0,
            item:           0,
            filter:         false,
            sort:           Sort::default(),
            redraw_images:  0,
            drawing_images: false,
            sort_ascending: false,
            search_input:   TextArea::default(),

            movies:          vec![],
            filtered_movies: vec![],
            image_renderer:  RatatuiImage::new(cache_dir),

            movies_list_scroll_pos:             0,
            movies_list_selected_item:          0,
            movies_list_alignment_bottom:       false,
            movies_list_partially_visible_item: false,
            movies_list_num_visible_items:      0,
            movies_description_selected_tab:    0,

            context_menu_pos:      None,
            context_menu_selected: 0,

            movies_description_plays_tab:       PlaysTab::default(),
            movies_description_overview_scroll: 0,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        if !self.movies.is_empty() {
            for tab in 0..=1 {
                key_event_handler.bind_key((Some(tab), None), 'A', "Add play".into(), |app, _| {
                    app.drawer.open_add_play_popup();
                });
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'e',
                    "Edit movie".into(),
                    |app, _| {
                        app.drawer.open_edit_movie_popup();
                    },
                );
                key_event_handler.bind_key(
                    (Some(tab), None),
                    'd',
                    "Delete movie".into(),
                    |app, _| {
                        app.drawer.open_delete_movie_popup();
                    },
                );
            }
        }
        key_event_handler.bind_key((Some(0), None), ',', "Sort by".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 1;
            }
        });
        key_event_handler.bind_key((Some(0), None), '/', "Find".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 0;
                main_screen.filter = false;

                main_screen.search_input = TextArea::from([""]);
            }
        });
        key_event_handler.bind_key((Some(0), None), 'f', "Filter".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 2;
                main_screen.item = 0;
                main_screen.filter = true;

                main_screen.sort = Sort::Relevance;
                if !main_screen.search_input.is_empty() {
                    main_screen.search_input = TextArea::from([""]);
                    main_screen.filter_sort_movies(true);
                } else {
                    main_screen.search_input = TextArea::from([""]);
                }
            }
        });
        key_event_handler.bind_key((Some(0), None), 'a', "Add movie".into(), |app, _| {
            app.drawer.open_add_movie_popup(
                app.trakt_tokens.clone(),
                app.tmdb_tokens.clone(),
                app.omdb_tokens.clone(),
            );
        });
        if !self.search_input.is_empty() {
            key_event_handler.bind_esc((Some(0), None), "Clear search".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.search_input = TextArea::from([""]);
                    if main_screen.filter {
                        main_screen.filter_sort_movies(true);
                    }
                }
            });
        }

        let frame_area = frame.area();

        // let num_movies = ((frame_area.height - 5) as f32 / 9.0).floor() as usize;
        // let footer_height = (((frame_area.height - 5) % 9) % num_movies as u16) + 2;

        let [header, vert, footer] = vertical![==3, >=1, ==2].areas(frame_area);
        let [description, list] = horizontal![>=30, ==2/3].areas(vert);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), header);

        self.drawing_images = false;
        self.render_movies_list(frame, list, key_event_handler);
        self.render_movie_description(frame, description, key_event_handler);
        self.render_footer(frame, footer, key_event_handler);
        self.redraw_images = self.redraw_images.saturating_sub(1);

        let sort_area = self.render_header(frame, header, key_event_handler);
        if self.tab == 2 && self.item == 1 {
            let sort_popup_area = sort_area.offset(Offset::new(0, 2)).resize(Size {
                width:  sort_area.width,
                height: sort_area.height + 5,
            });
            let sort_popup_block = Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .style(Style::new().fg(material::INDIGO.c900));
            frame.render_widget(&sort_popup_block, sort_popup_area);
            frame.render_widget(
                Block::new().bg(material::BLUE.c600),
                add_padding(sort_popup_area, Padding::bottom(2)),
            );

            let mut items: Vec<Line> = vec![
                " Added",
                " Rating",
                " IMDB",
                " Name",
                " Release",
                " Relevance",
            ]
            .iter()
            .map(|&x| {
                line!(x).style(
                    Style::new()
                        .fg(material::INDIGO.c200)
                        .bg(material::INDIGO.c900),
                )
            })
            .collect();
            items[ToPrimitive::to_usize(&self.sort).unwrap()] = items
                [ToPrimitive::to_usize(&self.sort).unwrap()]
            .clone()
            .style(
                Style::new()
                    .fg(material::BLUE.c100)
                    .bg(material::LIGHT_BLUE.c900),
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
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.sort = FromPrimitive::from_usize(i).unwrap();
                            main_screen.filter_sort_movies(true);
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
            }

            frame.render_widget(
                Text::from_iter(items).left_aligned(),
                sort_popup_block.inner(sort_popup_area),
            );
        }

        if let Some(pos) = self.context_menu_pos {
            key_event_handler.clear();
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                frame.area(),
                |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                        main_screen.redraw_images = 1;
                    }
                    app.drawer.refresh_immediate += 2;
                },
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Right,
                frame.area(),
                |app, _| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.context_menu_pos = None;
                        main_screen.redraw_images = 1;
                    }
                    app.drawer.refresh_immediate += 2;
                },
            );
            key_event_handler.bind_vertical((None, None), "Navigate".into(), |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(false, _) => {
                            main_screen.context_menu_selected =
                                main_screen.context_menu_selected.saturating_sub(1);
                        }
                        crate::key_event_handler::Data::Direction(true, _) => {
                            main_screen.context_menu_selected =
                                main_screen.context_menu_selected.add(1).min(2);
                        }
                        _ => (),
                    }
                }
            });
            key_event_handler.bind_enter((None, None), "Choose".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;

                    if main_screen.context_menu_selected == 0 {
                        app.drawer.open_add_play_popup();
                    } else if main_screen.context_menu_selected == 1 {
                        app.drawer.open_edit_movie_popup();
                    } else if main_screen.context_menu_selected == 2 {
                        app.drawer.open_delete_movie_popup();
                    }
                }
                app.drawer.refresh_immediate += 2;
            });
            key_event_handler.bind_esc((None, None), "Cancel".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;
                }
                app.drawer.refresh_immediate += 2;
            });
            key_event_handler.bind_key((None, None), 'q', "Cancel".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;
                }
                app.drawer.refresh_immediate += 2;
            });
            key_event_handler.bind_key((None, None), 'A', "Add play".into(), |app, _| {
                app.drawer.open_add_play_popup();
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;
                }
                app.drawer.refresh_immediate += 2;
            });
            key_event_handler.bind_key((None, None), 'e', "Edit movie".into(), |app, _| {
                app.drawer.open_edit_movie_popup();
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;
                }
                app.drawer.refresh_immediate += 2;
            });
            key_event_handler.bind_key((None, None), 'd', "Delete movie".into(), |app, _| {
                app.drawer.open_delete_movie_popup();
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.context_menu_pos = None;
                    main_screen.redraw_images = 1;
                }
                app.drawer.refresh_immediate += 2;
            });
            self.render_footer(frame, footer, key_event_handler);

            let mut actions = vec![" add play ", " edit ", " delete "]
                .iter()
                .map(|&x| {
                    line!(x).style(
                        Style::new()
                            .fg(material::INDIGO.c200)
                            .bg(material::INDIGO.c900),
                    )
                })
                .collect_vec();
            actions[self.context_menu_selected] =
                actions[self.context_menu_selected].clone().style(
                    Style::new()
                        .fg(material::BLUE.c100)
                        .bg(material::LIGHT_BLUE.c900),
                );
            let width = actions.iter().map(|x| x.width()).max().unwrap() as u16 + 4;
            let height = actions.len() as u16 + 2;

            let x = if pos.x + width - 1 >= frame.area().width {
                frame.area().width - width
            } else {
                pos.x
            };
            let y = if pos.y + height - 1 >= frame.area().height {
                frame.area().height - height
            } else {
                pos.y
            };
            let actions_popup_area = Rect::new(x, y, width, height);
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                actions_popup_area.outer(Margin::new(1, 1)),
                |_, _| {},
            );

            let bg_top = frame
                .buffer_mut()
                .cell(Position::new(
                    actions_popup_area.x + actions_popup_area.width / 2,
                    actions_popup_area.y,
                ))
                .unwrap()
                .bg;
            let bg_bottom = frame
                .buffer_mut()
                .cell(Position::new(
                    actions_popup_area.x + actions_popup_area.width / 2,
                    actions_popup_area.y + actions_popup_area.height - 1,
                ))
                .unwrap()
                .bg;

            let actions_popup_block = Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .style(Style::new().fg(material::INDIGO.c900));
            frame.render_widget(Clear, actions_popup_area);
            frame.render_widget(&actions_popup_block, actions_popup_area);
            frame.render_widget(
                Block::new().bg(bg_top),
                add_padding(actions_popup_area, Padding::bottom(1)),
            );
            frame.render_widget(
                Block::new().bg(bg_bottom),
                add_padding(actions_popup_area, Padding::top(1)),
            );
            let actions_inner_area = actions_popup_block.inner(actions_popup_area);
            let mut mouse_area = actions_inner_area.resize(Size {
                width:  actions_inner_area.width,
                height: 1,
            });
            for i in 0..actions.len() {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if i == 0 {
                            app.drawer.open_add_play_popup();
                        } else if i == 1 {
                            app.drawer.open_edit_movie_popup();
                        } else if i == 2 {
                            app.drawer.open_delete_movie_popup();
                        }
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.context_menu_pos = None;
                            main_screen.redraw_images = 1;
                        }
                        app.drawer.refresh_immediate += 2;
                    },
                );
                mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
            }

            frame.render_widget(Text::from_iter(actions).left_aligned(), actions_inner_area);
        }
    }

    fn render_header(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) -> Rect {
        key_event_handler.bind_esc((Some(2), None), "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                if let Sort::Relevance = main_screen.sort {
                    main_screen.sort = Sort::default();
                }

                main_screen.search_input = TextArea::from([""]);
                if main_screen.filter {
                    main_screen.filter_sort_movies(true);
                }
            }
        });
        key_event_handler.bind_enter((Some(2), None), "Confirm".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;

                if !main_screen.filter {
                    main_screen.search_input = TextArea::from([""]);
                }
            }
        });
        key_event_handler.bind_tab((Some(2), Some(0)), "Change focus".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.item = 1;
            }
        });
        key_event_handler.bind_tab((Some(2), None), "Change focus".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_horizontal((Some(2), Some(1)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        main_screen.item += 1;
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_horizontal((Some(2), Some(2)), "Navigate".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false, _) => {
                        main_screen.item -= 1;
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_vertical((Some(2), Some(1)), "Change sort".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(false, _) => {
                        main_screen.sort = FromPrimitive::from_usize(
                            ToPrimitive::to_usize(&main_screen.sort)
                                .unwrap()
                                .checked_sub(1)
                                .unwrap_or(0),
                        )
                        .unwrap();
                    }
                    crate::key_event_handler::Data::Direction(true, _) => {
                        main_screen.sort = FromPrimitive::from_usize(
                            ToPrimitive::to_usize(&main_screen.sort).unwrap() + 1,
                        )
                        .unwrap_or(main_screen.sort);
                    }
                    _ => (),
                }
                main_screen.filter_sort_movies(true);
            }
        });
        key_event_handler.bind_vertical(
            (Some(2), Some(2)),
            "Change sort order".into(),
            |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(false, _) => {
                            if main_screen.sort_ascending == false {
                                main_screen.sort_ascending = true;
                                main_screen.filter_sort_movies(true);
                            }
                        }
                        crate::key_event_handler::Data::Direction(true, _) => {
                            if main_screen.sort_ascending == true {
                                main_screen.sort_ascending = false;
                                main_screen.filter_sort_movies(true);
                            }
                        }
                        _ => (),
                    }
                }
            },
        );
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
        key_event_handler.bind_key((Some(2), None), 'q', "Close".into(), |app, _| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                main_screen.tab = 0;
                main_screen.item = 0;
            }
        });
        key_event_handler.bind_input_field((Some(2), Some(0)), "".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Key(key_event) => {
                        main_screen.search_input.input(key_event);

                        if main_screen.filter {
                            main_screen.filter_sort_movies(true);
                        } else {
                            main_screen.goto_movie();
                        }
                    }
                    _ => {}
                }
            }
        });

        let [debug_area, input_area, _, sort_area, _, direction_area, _] =
            horizontal![>=1, <=25, ==1, <=14, ==1, ==3, ==1].areas(area);

        frame.render_widget(Paragraph::new(format!("scroll_pos: {} selected_item: {} movies_num: {} num_visible_items: {} partially_visible: {} alignment_bottom: {}", self.movies_list_scroll_pos, self.movies_list_selected_item, self.filtered_movies.len(), self.movies_list_num_visible_items, self.movies_list_partially_visible_item, self.movies_list_alignment_bottom)).wrap(Wrap { trim: false }), debug_area);

        let tab_selected = self.tab == 2;
        self.search_input
            .set_style(Style::new().fg(if tab_selected {
                if self.item == 0 {
                    tailwind::SLATE.c300
                } else {
                    tailwind::STONE.c400
                }
            } else {
                tailwind::STONE.c500
            }));
        self.search_input.set_cursor_style(
            Style::new()
                .fg(if tab_selected {
                    if self.item == 0 {
                        tailwind::SLATE.c300
                    } else {
                        tailwind::STONE.c400
                    }
                } else {
                    tailwind::STONE.c500
                })
                .add_modifier(if tab_selected && self.item == 0 {
                    Modifier::REVERSED
                } else {
                    Modifier::default()
                }),
        );
        self.search_input.set_block(
            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Thick)
                .style(Style::new().fg(if tab_selected {
                    if self.item == 0 {
                        material::BLUE.c500
                    } else {
                        tailwind::SLATE.c500
                    }
                } else {
                    tailwind::STONE.c600
                }))
                .padding(Padding::symmetric(1, 0)),
        );
        self.search_input.set_placeholder_text("Search");
        self.search_input
            .set_placeholder_style(Style::new().fg(material::GRAY.c700));
        frame.render_widget(&self.search_input, input_area);
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            input_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 0;

                    main_screen.filter = true;
                    main_screen.search_input = TextArea::from([""]);
                    main_screen.filter_sort_movies(true);
                }
            },
        );

        let bg = |x: usize| -> Color {
            if tab_selected {
                if self.item == x {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            }
        };
        let fg = |x: usize| -> Color {
            if tab_selected {
                if self.item == x {
                    material::TEAL.c100
                } else {
                    material::INDIGO.c200
                }
            } else {
                material::GRAY.c400
            }
        };

        // "▼⬇⬆⏷"
        let sort_block = Block::bordered()
            .border_set(border::PROPORTIONAL_WIDE)
            .style(Style::new().fg(bg(1)));
        let sort = ellipsize_string(
            match self.sort {
                Sort::AddedDate => "Added",
                Sort::UserRating => "Rating",
                Sort::Rating => "IMDB",
                Sort::Name => "Name",
                Sort::ReleaseDate => "Release",
                Sort::Relevance => "Relevance",
            },
            10,
        );
        frame.render_widget(&sort_block, sort_area);
        frame.render_widget(
            line![span!(sort)].style(Style::new().bold().fg(fg(1)).bg(bg(1))),
            sort_block.inner(sort_area),
        );
        frame.render_widget(
            span!(" ▼")
                .into_right_aligned_line()
                .style(Style::new().bold().fg(fg(1)).bg(bg(1))),
            sort_block.inner(sort_area),
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            sort_area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 2;
                    main_screen.item = 1;
                }
            },
        );

        let direction_block = Block::bordered()
            .border_set(border::PROPORTIONAL_WIDE)
            .style(Style::new().fg(bg(2)));
        let direction = if self.sort_ascending { "⬆" } else { "⬇" };
        frame.render_widget(&direction_block, direction_area);
        frame.render_widget(
            span!(direction)
                .into_centered_line()
                .style(Style::new().bold().fg(fg(2)).bg(bg(2))),
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

        return sort_area;
    }

    fn render_movies_list(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) {
        if self.filtered_movies.len() > 0 {
            let num_visible_items = self.movies_list_num_visible_items;
            key_event_handler.bind_vertical((Some(0), None), "Scroll".into(), move |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(true, modifiers) => {
                            if modifiers.contains(KeyModifiers::SHIFT) {
                                main_screen.goto_index(
                                    (main_screen.movies_list_selected_item
                                        + num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.movies_list_selected_item = main_screen
                                    .movies_list_selected_item
                                    .add(1)
                                    .min(main_screen.filtered_movies.len().saturating_sub(1));
                                if main_screen.movies_list_selected_item
                                    - main_screen.movies_list_scroll_pos
                                    >= main_screen.movies_list_num_visible_items
                                {
                                    main_screen.movies_list_scroll_pos += 1;
                                }
                            }
                        }
                        crate::key_event_handler::Data::Direction(false, modifiers) => {
                            if modifiers.contains(KeyModifiers::SHIFT) {
                                main_screen.goto_index(
                                    main_screen
                                        .movies_list_selected_item
                                        .saturating_sub(num_visible_items.saturating_sub(1))
                                        as isize,
                                );
                            } else {
                                main_screen.movies_list_selected_item =
                                    main_screen.movies_list_selected_item.saturating_sub(1);
                                if main_screen.movies_list_selected_item
                                    < main_screen.movies_list_scroll_pos
                                {
                                    main_screen.movies_list_scroll_pos -= 1;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });

            key_event_handler.bind_key((Some(0), None), "gg", "Jump to top".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(0);
                }
            });
            key_event_handler.bind_key((Some(0), None), 'G', "Jump to bottom".into(), |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.goto_index(-1);
                }
            });
            key_event_handler.bind_tab((Some(0), None), "Change focus".into(), |app, data| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    match data {
                        crate::key_event_handler::Data::Direction(true, _) => {
                            main_screen.tab += 1;
                            if main_screen.tab > 1 {
                                main_screen.tab = 0;
                            }
                        }
                        crate::key_event_handler::Data::Direction(false, _) => {
                            main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                        }
                        _ => (),
                    }
                }
            });
        }

        if self.movies_list_selected_item >= self.filtered_movies.len() {
            self.movies_list_selected_item = self.filtered_movies.len().saturating_sub(1);
            self.movies_list_scroll_pos = self
                .movies_list_selected_item
                .saturating_sub(self.movies_list_num_visible_items.saturating_sub(1));
        }

        let num_visible_movies = area.height as usize / MOVIE_WIDGET_HEIGHT;
        let partially_visible_movie_height =
            area.height as usize - num_visible_movies * MOVIE_WIDGET_HEIGHT;
        let render_partially_visible_movie = partially_visible_movie_height > 0;
        if self.movies_list_num_visible_items
            != num_visible_movies + if render_partially_visible_movie { 1 } else { 0 }
            || self.movies_list_partially_visible_item != render_partially_visible_movie
        {
            let selected_movie_index = self
                .movies_list_selected_item
                .saturating_sub(self.movies_list_scroll_pos)
                .saturating_sub(
                    if self.movies_list_partially_visible_item && self.movies_list_alignment_bottom
                    {
                        1
                    } else {
                        0
                    },
                )
                .min(num_visible_movies.saturating_sub(1));

            if self
                .movies_list_selected_item
                .saturating_sub(selected_movie_index)
                == 0
            {
                self.movies_list_scroll_pos = 0;
                self.movies_list_alignment_bottom = false;
            } else {
                self.movies_list_scroll_pos = self
                    .movies_list_selected_item
                    .saturating_sub(selected_movie_index)
                    - if render_partially_visible_movie && self.movies_list_alignment_bottom {
                        1
                    } else {
                        0
                    };
            }
        }
        self.movies_list_num_visible_items =
            num_visible_movies + if render_partially_visible_movie { 1 } else { 0 };
        self.movies_list_partially_visible_item = render_partially_visible_movie;

        if self.movies_list_scroll_pos + self.movies_list_num_visible_items
            > self.filtered_movies.len()
        {
            self.movies_list_scroll_pos = self
                .filtered_movies
                .len()
                .saturating_sub(self.movies_list_num_visible_items);
            self.movies_list_alignment_bottom = true;
        }
        if self.movies_list_partially_visible_item {
            if self.filtered_movies.len() <= num_visible_movies {
                self.movies_list_alignment_bottom = false;
            } else if self.movies_list_selected_item == self.movies_list_scroll_pos {
                self.movies_list_alignment_bottom = false;
            } else if self.movies_list_selected_item - self.movies_list_scroll_pos
                == self.movies_list_num_visible_items - 1
            {
                self.movies_list_alignment_bottom = true;
            }
        }

        let [movies_area, scrollbar_area] = horizontal![>=0, ==1].areas(area);
        let mut remaining_area = movies_area;
        for i in 0..self.movies_list_num_visible_items {
            let [area, remaining] = if self.movies_list_partially_visible_item
                && ((i == 0 && self.movies_list_alignment_bottom)
                    || (i == self.movies_list_num_visible_items - 1
                        && !self.movies_list_alignment_bottom))
            {
                vertical![==partially_visible_movie_height as u16, >= 0]
            } else {
                vertical![==MOVIE_WIDGET_HEIGHT as u16, >= 0]
            }
            .areas(remaining_area);
            remaining_area = remaining;

            if !self.filtered_movies.is_empty()
                && i + self.movies_list_scroll_pos < self.filtered_movies.len()
            {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;

                            main_screen.movies_list_selected_item =
                                i + main_screen.movies_list_scroll_pos;
                        }
                    },
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Right,
                    area,
                    move |app, data| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 0;
                            main_screen.item = 0;

                            main_screen.movies_list_selected_item =
                                i + main_screen.movies_list_scroll_pos;

                            if let crate::key_event_handler::Data::Mouse(mouse_event) = data {
                                main_screen.context_menu_pos =
                                    Some(Position::new(mouse_event.column, mouse_event.row));
                                main_screen.context_menu_selected = 0;
                            }
                        }
                    },
                );

                self.draw_movie_widget(i, frame, area);
            } else {
                frame.render_widget(
                    Block::new().bg(if i & 1 == 1 {
                        tailwind::NEUTRAL.c900
                    } else {
                        tailwind::STONE.c900
                    }),
                    area,
                );
            }
        }

        if self.filtered_movies.len() > num_visible_movies {
            let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .symbols(Set {
                    track: block::FULL,
                    thumb: block::FULL,
                    begin: "▲",
                    end:   "▼",
                })
                .begin_style(
                    Style::new()
                        .bg(material::LIGHT_BLUE.c700)
                        .fg(tailwind::INDIGO.c900),
                )
                .end_style(
                    Style::new()
                        .bg(material::LIGHT_BLUE.c700)
                        .fg(tailwind::INDIGO.c900),
                )
                .track_style(Style::new().fg(tailwind::SLATE.c900))
                .thumb_style(
                    Style::new()
                        .fg(material::BLUE.c800)
                        .bg(tailwind::SLATE.c900),
                );

            let mut scrollbar_state = ScrollbarState::new(
                self.filtered_movies
                    .len()
                    .saturating_sub(num_visible_movies),
            )
            .position(self.movies_list_scroll_pos);

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    fn draw_movie_widget(&mut self, id: usize, frame: &mut Frame, area: Rect) {
        let is_partially_visible = MOVIE_WIDGET_HEIGHT > area.height as usize;
        let movie_index = self.movies_list_scroll_pos + id;
        let selected = self.movies_list_selected_item == movie_index;
        let tab_selected = self.tab == 0;
        let alt = movie_index & 1 == 1;
        let movie = &self.filtered_movies[movie_index];

        let (background, text) = if selected {
            if tab_selected {
                (tailwind::EMERALD.c800, tailwind::EMERALD.c200)
            } else {
                (tailwind::TEAL.c900, tailwind::BLUE.c200)
            }
        } else if !alt {
            (tailwind::ZINC.c800, material::BLUE_GRAY.c200)
        } else {
            (tailwind::GRAY.c800, material::GRAY.c400)
        };

        frame.render_widget(Block::new().bg(background).fg(text), area);

        let vert_lay = add_padding(
            area,
            if is_partially_visible {
                Padding::new(
                    2,
                    2,
                    if self.movies_list_alignment_bottom { 0 } else { 1 },
                    if self.movies_list_alignment_bottom { 1 } else { 0 },
                )
            } else {
                Padding::proportional(1)
            },
        );
        let poster_width = ((MOVIE_WIDGET_HEIGHT - 2) as f32 / 1.5).ceil() as u16 * 2 + 1;
        let [poster_area, _, description_area] =
            horizontal![==poster_width, ==2, >=0].areas(vert_lay);
        let highlight_area = area
            .resize(Size::new(2, area.height.saturating_sub(2)))
            .offset(Offset::new(0, 1));

        let name = ellipsize_string(&movie.name, description_area.width as usize - 11);

        let rating = movie.get_user_rating();
        let rating_color = if rating >= 9.0 {
            tailwind::SKY.c400
        } else if rating >= 8.0 {
            tailwind::GREEN.c500
        } else if rating >= 7.5 {
            tailwind::LIME.c400
        } else if rating >= 7.0 {
            material::AMBER.c400
        } else if rating >= 6.0 {
            material::DEEP_ORANGE.c300
        } else {
            material::RED.c400
        };

        let mut description_lines = vec![
            name.bold() + " ".into() + movie.year.clone().italic(),
            format!("{:.1}", rating)
                .set_style(rating_color)
                .bold()
                .into(),
        ];

        const TAGLINE_LINES: usize = 2;
        let mut tagline_lines = wrap_text(&movie.tagline, description_area.width as usize);
        for _ in 0..(TAGLINE_LINES.saturating_sub(tagline_lines.len())) {
            description_lines.push("".into());
        }
        tagline_lines.reverse();
        for _ in 0..TAGLINE_LINES.min(tagline_lines.len()) {
            description_lines.push(tagline_lines.pop().unwrap().into());
        }

        let areas = Layout::vertical(vec![constraint!(==1); description_area.height as usize])
            .split(description_area);
        for i in 0..description_area.height {
            let index = if is_partially_visible {
                if self.movies_list_alignment_bottom {
                    i + (MOVIE_WIDGET_HEIGHT as u16 - 1 - area.height)
                } else {
                    i
                }
            } else {
                i
            };

            let area = areas[i as usize];
            match index {
                0 => frame.render_widget(
                    line!(format!("#{}", movie_index + 1))
                        .right_aligned()
                        .bold()
                        .style(Style::new().fg(if selected {
                            tailwind::GRAY.c200
                        } else {
                            tailwind::GRAY.c400
                        })),
                    area,
                ),
                _ =>
                    for i in 0..4 {
                        if index == MOVIE_WIDGET_HEIGHT as u16 - 2 - i - 1 {
                            frame.render_widget(&description_lines[3 - i as usize], area)
                        }
                    },
            }
        }

        let unfocused_rating_color = if rating >= 9.0 {
            tailwind::SKY.c600
        } else if rating >= 8.0 {
            tailwind::GREEN.c700
        } else if rating >= 7.5 {
            tailwind::LIME.c700
        } else if rating >= 7.0 {
            material::YELLOW.c700
        } else if rating >= 6.0 {
            tailwind::AMBER.c600
        } else {
            material::DEEP_ORANGE.c800
        };
        if selected {
            frame.render_widget(
                text![line!["▐"]; highlight_area.height as usize].fg(if tab_selected {
                    rating_color
                } else {
                    unfocused_rating_color
                }),
                highlight_area,
            );
        }

        if self.redraw_images < 1 {
            self.drawing_images |= !self.image_renderer.draw_image(
                self.filtered_movies[movie_index].id.tmdb,
                false,
                poster_area,
                if is_partially_visible {
                    Some(SignedPosition {
                        x: 0,
                        y: if self.movies_list_alignment_bottom {
                            -(MOVIE_WIDGET_HEIGHT as i16 - 2 - poster_area.height as i16)
                        } else {
                            0
                        },
                    })
                } else {
                    None
                },
                frame,
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), poster_area);
        }
    }

    fn render_movie_description(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) {
        const TABS: [&str; 2] = ["Overview", "Plays"];
        const TABS_COUNT: usize = TABS.len();
        key_event_handler.bind_horizontal((Some(1), None), "Change tab".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        main_screen.movies_description_selected_tab = main_screen
                            .movies_description_selected_tab
                            .add(1)
                            .min(TABS_COUNT - 1);
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        main_screen.movies_description_selected_tab = main_screen
                            .movies_description_selected_tab
                            .checked_sub(1)
                            .unwrap_or(0);
                    }
                    _ => (),
                }
            }
        });
        key_event_handler.bind_tab((Some(1), None), "Change focus".into(), |app, data| {
            if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        main_screen.tab += 1;
                        if main_screen.tab > 1 {
                            main_screen.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        main_screen.tab = main_screen.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => (),
                }

                main_screen.item = 0;
            }
        });
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            area,
            |app, _| {
                if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
                    main_screen.tab = 1;
                    main_screen.item = 0;
                }
            },
        );

        let description_selected = self.tab == 1;
        let movie = if self.filtered_movies.is_empty() {
            None
        } else {
            Some(&self.filtered_movies[self.movies_list_selected_item].clone())
        };

        let inner = add_padding(area, Padding::proportional(1));
        let backdrop_height = ((inner.width - 4) as f32 * 9.0 / 32.0).ceil() as u16;
        let [backdrop_area, title_area, description_area] =
            vertical![==backdrop_height, ==8, >=1].areas(inner);

        frame.render_widget(Block::new().bg(tailwind::SLATE.c800), area);

        if let Some(movie) = movie {
            let [title_area, _, ratings_area, _, tabs_area] =
                vertical![==2, ==1, ==2, ==1, ==2].areas(title_area);

            let mut name = movie.name.clone();
            name = ellipsize_string(&name, title_area.width as usize);

            frame.render_widget(
                text![
                    name.bold().into_centered_line(),
                    movie.year.as_str().italic().into_centered_line(),
                ],
                title_area,
            );
            self.draw_ratings(movie, frame, ratings_area);

            const BGS: [Color; 2] = [material::GREEN.c600, material::LIGHT_BLUE.c600];
            const FGS: [Color; 2] = [material::BLUE.c100, material::YELLOW.c100];
            const _BGS: [Color; 2] = [material::TEAL.c800, material::INDIGO.c600];
            const _FGS: [Color; 2] = [material::BLUE_GRAY.c200, material::BLUE_GRAY.c200];
            let mut tabs = TABS
                .iter()
                .enumerate()
                .flat_map(|(i, &x)| {
                    [
                        span!(format!(" {} ", x))
                            .fg(if description_selected { FGS[i] } else { _FGS[i] })
                            .bg(if description_selected { BGS[i] } else { _BGS[i] })
                            .add_modifier(if i == self.movies_description_selected_tab {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                        " ".into(),
                    ]
                })
                .take(TABS.len() * 2 - 1)
                .collect_vec();
            let mut mouse_area = tabs_area;
            for (i, tab) in tabs.iter_mut().enumerate() {
                if i & 1 == 1 {
                    mouse_area = mouse_area.offset(Offset { x: 1, y: 0 });
                    continue;
                }
                mouse_area = mouse_area.resize(Size {
                    width:  tab.width() as u16,
                    height: 1,
                });

                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Screens::MainScreen(main_screen)) =
                            app.drawer.current_screen.as_mut()
                        {
                            main_screen.tab = 1;
                            main_screen.item = 0;
                            main_screen.movies_description_selected_tab = i / 2;
                        }
                    },
                );
                mouse_area = mouse_area.offset(Offset {
                    x: mouse_area.width as i32,
                    y: 0,
                });
            }
            frame.render_widget(
                text![
                    tabs,
                    Line::from("🮂".repeat(title_area.width as usize)).fg(if description_selected {
                        BGS[self.movies_description_selected_tab]
                    } else {
                        _BGS[self.movies_description_selected_tab]
                    }),
                ],
                tabs_area,
            );

            match self.movies_description_selected_tab {
                0 => {
                    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), description_area);

                    let mut overview_lines =
                        wrap_text(&movie.overview, description_area.width as usize);
                    let line_count = overview_lines.len();
                    self.movies_description_overview_scroll = self
                        .movies_description_overview_scroll
                        .min(line_count.saturating_sub(description_area.height as usize));
                    frame.render_widget(
                        Text::from_iter(
                            overview_lines.split_off(self.movies_description_overview_scroll),
                        ),
                        description_area,
                    );

                    key_event_handler.bind_vertical(
                        (Some(1), Some(self.movies_description_selected_tab << 9)),
                        "Scroll".into(),
                        move |app, data| {
                            if let Some(Screens::MainScreen(main_screen)) =
                                app.drawer.current_screen.as_mut()
                            {
                                match data {
                                    crate::key_event_handler::Data::Direction(false, _) => {
                                        main_screen.movies_description_overview_scroll =
                                            main_screen
                                                .movies_description_overview_scroll
                                                .saturating_sub(1);
                                    }
                                    crate::key_event_handler::Data::Direction(true, _) => {
                                        main_screen.movies_description_overview_scroll += 1;
                                    }
                                    _ => (),
                                }
                            }
                        },
                    );
                }
                1 => self.draw_plays_tab(key_event_handler, movie, frame, description_area),
                _ => (),
            };
        }

        if self.redraw_images < 1 && movie.is_some() {
            self.drawing_images |= !self.image_renderer.draw_image(
                self.current_movie().unwrap().id.tmdb,
                true,
                backdrop_area,
                None,
                frame,
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::SLATE.c700), backdrop_area);
        }
    }

    fn render_footer(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        key_event_handler: &mut KeyEventHandler,
    ) {
        frame.render_widget(Clear, area);
        frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), area);

        // ↔↕⇆⬌⬍⮀⬅⬆⬇←↑→↓↹•↵⏎
        let bind_to_string = |bind: &crate::key_event_handler::Bind| {
            match bind {
                crate::key_event_handler::Bind::Horizontal => {
                    span!(" ←•→ ")
                }
                crate::key_event_handler::Bind::Vertical => span!(" ↕ "),
                crate::key_event_handler::Bind::Enter => span!(" ↵ "),
                crate::key_event_handler::Bind::Esc => span!(" Esc "),
                crate::key_event_handler::Bind::Tab => span!(" ↹ "),
                crate::key_event_handler::Bind::Key(x) => {
                    span!(format!(" {} ", if x == " " { "␣" } else { x }))
                }
                _ => "_".into(),
            }
            .bold()
            .fg(material::ORANGE.c600)
        };
        let binds = key_event_handler.get_key_binds_descriptions(
            self.get_state(),
            ((area.width as f64 / 10.0).floor() as u16 * area.height) as usize,
        );

        let num_items_per_row = (binds.len() as f64 / area.height as f64).ceil() as usize;
        let len_item = ((area.width - 2 * (num_items_per_row as u16 - 1)) as f32
            / num_items_per_row as f32)
            .floor() as u16;

        let verts = Layout::vertical(vec![constraint!(==1); area.height as usize]).split(area);
        let mut areas = verts.into_iter().flat_map(|&area| {
            Layout::horizontal(vec![constraint!(==len_item); num_items_per_row])
                .split(area)
                .into_iter()
                .map(|&x| x)
                .collect::<Vec<_>>()
        });

        binds.into_iter().for_each(|x| {
            let bind = bind_to_string(&x.0);
            let desc = ellipsize_string(&x.1, len_item as usize - bind.width());
            frame.render_widget(
                line![bind, span!(desc).fg(material::BLUE_GRAY.c200)],
                areas.next().unwrap(),
            );
        });
    }

    fn draw_ratings(&self, movie: &Movie, frame: &mut Frame, area: Rect) {
        let imdb_bg = Color::Rgb(245, 197, 24);
        let imdb_fg = Color::Black;
        let imdb_label_fg = Color::Rgb(250, 225, 120);
        let trakt_bg = Color::Rgb(165, 61, 185);
        let trakt_fg = Color::White;
        let trakt_label_fg = Color::Rgb(230, 140, 245);
        let tmdb_bg = Color::Rgb(42, 187, 209);
        let tmdb_fg = Color::Black;
        let tmdb_label_fg = Color::Rgb(140, 205, 215);

        let mut ratings = vec![];
        for rating in movie.ratings {
            if let Rating::IMDB(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
            if let Rating::Trakt(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
            if let Rating::TMDB(a, _) = rating {
                if a > 0.0 {
                    ratings.push(rating);
                }
            }
        }

        if ratings.is_empty() {
            frame.render_widget(Line::from("NA").centered(), area);

            return;
        }

        let spaces = ((area.width - 5 * (ratings.len() as u16)) as f64 / (ratings.len() + 1) as f64)
            .ceil() as usize;

        let mut widgets = Line::from(" ".repeat(spaces));
        let mut labels = Line::from(" ".repeat(spaces));
        for (i, rating) in ratings.iter().enumerate() {
            let (bg, fg, r) = if let Rating::IMDB(a, _) = rating {
                labels.push_span(Span::from("IMDB").fg(imdb_label_fg));
                if i != ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces + 1));
                }

                (imdb_bg, imdb_fg, a)
            } else if let Rating::Trakt(a, _) = rating {
                labels.push_span(Span::from("Trakt").fg(trakt_label_fg));
                if i != ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces));
                }

                (trakt_bg, trakt_fg, a)
            } else if let Rating::TMDB(a, _) = rating {
                labels.push_span(Span::from("TMDB").fg(tmdb_label_fg));
                if i != ratings.len() - 1 {
                    labels.push_span(" ".repeat(spaces + 1));
                }

                (tmdb_bg, tmdb_fg, a)
            } else {
                continue;
            };

            widgets.push_span("".fg(bg));
            widgets.push_span(format!("{:.1}", r).bg(bg).fg(fg).bold());
            widgets.push_span("".fg(bg));
            if i != ratings.len() - 1 {
                widgets.push_span(" ".repeat(spaces));
            }
        }

        frame.render_widget(text![labels, widgets], area);
    }

    fn draw_plays_tab(
        &mut self,
        key_event_handler: &mut KeyEventHandler,
        movie: &Movie,
        frame: &mut Frame,
        area: Rect,
    ) {
        let tab_selected = self.tab == 1;
        let num_plays = movie.plays.len();
        let num_visible_plays = area.height as usize / 3;
        let partially_visible_play_height = area.height as usize - num_visible_plays * 3;
        let render_partially_visible_play = partially_visible_play_height > 0;
        self.movies_description_plays_tab.num_visible_items =
            num_visible_plays + if render_partially_visible_play { 1 } else { 0 };

        if num_plays > num_visible_plays {
            key_event_handler.bind_vertical(
                (Some(1), Some(self.movies_description_selected_tab << 9)),
                "Scroll".into(),
                move |app, data| {
                    if let Some(Screens::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(false, _) => {
                                if main_screen.movies_description_plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description_plays_tab.alignment_bottom =
                                        false;
                                } else {
                                    main_screen.movies_description_plays_tab.scroll_pos =
                                        main_screen
                                            .movies_description_plays_tab
                                            .scroll_pos
                                            .saturating_sub(1);
                                }
                            }
                            crate::key_event_handler::Data::Direction(true, _) => {
                                if !main_screen.movies_description_plays_tab.alignment_bottom
                                    && render_partially_visible_play
                                {
                                    main_screen.movies_description_plays_tab.alignment_bottom =
                                        true;
                                } else {
                                    main_screen.movies_description_plays_tab.scroll_pos += 1;
                                }
                            }
                            _ => (),
                        }
                    }
                },
            );

            self.movies_description_plays_tab.scroll_pos =
                0.max(self.movies_description_plays_tab.scroll_pos.min(
                    num_plays.saturating_sub(self.movies_description_plays_tab.num_visible_items),
                ));
            self.movies_description_plays_tab.alignment_bottom =
                self.movies_description_plays_tab.alignment_bottom & render_partially_visible_play;
        } else {
            self.movies_description_plays_tab.scroll_pos = 0;
            self.movies_description_plays_tab.alignment_bottom = false;
        }

        frame.render_widget(Block::new().bg(tailwind::SLATE.c900), area);

        let mut remaining_area = add_padding(area, Padding::left(1));
        for i in 0..self.movies_description_plays_tab.num_visible_items {
            let [area, remaining] = if render_partially_visible_play
                && i == 0
                && self.movies_description_plays_tab.alignment_bottom
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else if render_partially_visible_play
                && i == self.movies_description_plays_tab.num_visible_items - 1
                && !self.movies_description_plays_tab.alignment_bottom
            {
                vertical![==partially_visible_play_height as u16, >= 0]
            } else {
                vertical![==3, >= 0]
            }
            .areas(remaining_area);

            if self.movies_description_plays_tab.scroll_pos + i < num_plays {
                let partially_visible = area.height < 3;
                let play =
                    &movie.plays[num_plays - self.movies_description_plays_tab.scroll_pos - i - 1];

                let alternate = i & 1 == 1;
                let latest = self.movies_description_plays_tab.scroll_pos + i == 0;
                let last =
                    self.movies_description_plays_tab.scroll_pos + i == movie.plays.len() - 1;

                frame.render_widget(
                    Block::new().bg(if latest {
                        if tab_selected {
                            tailwind::ZINC.c600
                        } else {
                            tailwind::ZINC.c700
                        }
                    } else if !alternate {
                        if tab_selected {
                            tailwind::GRAY.c600
                        } else {
                            tailwind::GRAY.c700
                        }
                    } else {
                        if tab_selected {
                            tailwind::SLATE.c700
                        } else {
                            tailwind::SLATE.c800
                        }
                    }),
                    add_padding(area, Padding::left(2)),
                );

                let areas =
                    Layout::vertical(vec![constraint!(==1); area.height as usize]).split(area);

                let rating_color = if play.1 >= 9.0 {
                    tailwind::SKY.c400
                } else if play.1 >= 8.0 {
                    tailwind::GREEN.c500
                } else if play.1 >= 7.5 {
                    tailwind::LIME.c400
                } else if play.1 >= 7.0 {
                    material::AMBER.c400
                } else if play.1 >= 6.0 {
                    material::DEEP_ORANGE.c300
                } else {
                    material::RED.c400
                };

                for i in 0..area.height {
                    let index = if partially_visible {
                        if self.movies_description_plays_tab.alignment_bottom {
                            i + 3 - area.height
                        } else {
                            i
                        }
                    } else {
                        i
                    };
                    match index {
                        0 =>
                            if !latest {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            } else {
                                frame.render_widget(
                                    Line::from("▔".repeat(area.width as usize)).fg(
                                        if tab_selected {
                                            tailwind::ZINC.c500
                                        } else {
                                            tailwind::ZINC.c600
                                        },
                                    ),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            },
                        1 => {
                            frame.render_widget(
                                span!("●").fg(if latest {
                                    if tab_selected {
                                        material::YELLOW.c800
                                    } else {
                                        material::CYAN.c500
                                    }
                                } else {
                                    if tab_selected {
                                        material::CYAN.c500
                                    } else {
                                        material::CYAN.c700
                                    }
                                }),
                                areas[i as usize],
                            );
                            frame.render_widget(
                                line![
                                    format!("{:.1}", play.1).fg(rating_color).add_modifier(
                                        if latest { Modifier::BOLD } else { Modifier::empty() }
                                    ),
                                    span!(" @ "),
                                    play.0.format("%d/%m/%Y %H:%M").to_string().fg(if latest {
                                        if tab_selected {
                                            material::YELLOW.c700
                                        } else {
                                            material::CYAN.c600
                                        }
                                    } else {
                                        if tab_selected {
                                            material::CYAN.c500
                                        } else {
                                            material::CYAN.c700
                                        }
                                    }),
                                ],
                                add_padding(areas[i as usize], Padding::left(4)),
                            );
                        }
                        2 => {
                            if !last {
                                frame.render_widget(
                                    span!("│").fg(material::GRAY.c600),
                                    areas[i as usize],
                                );
                            }
                            if latest {
                                frame.render_widget(
                                    Line::from("▁".repeat(area.width as usize)).fg(
                                        if tab_selected {
                                            tailwind::ZINC.c500
                                        } else {
                                            tailwind::ZINC.c600
                                        },
                                    ),
                                    add_padding(areas[i as usize], Padding::left(2)),
                                );
                            }
                        }
                        _ => (),
                    }
                }
            }

            remaining_area = remaining;
        }
    }

    pub fn goto_index(&mut self, index: isize) {
        let index = if index.is_negative() {
            self.filtered_movies.len() - 1
        } else {
            (index as usize).min(self.filtered_movies.len() - 1)
        };

        self.movies_list_selected_item = index;
        self.movies_list_scroll_pos = self
            .movies_list_scroll_pos
            .min(self.movies_list_selected_item);
        if self.movies_list_selected_item - self.movies_list_scroll_pos
            >= self.movies_list_num_visible_items
        {
            self.movies_list_scroll_pos =
                self.movies_list_selected_item - self.movies_list_num_visible_items + 1;
        }
    }

    pub fn current_movie(&self) -> Option<&Movie> {
        self.filtered_movies.get(self.movies_list_selected_item)
    }

    pub fn set_movies(&mut self, movies: &[Movie]) {
        self.movies = movies.to_vec();
        self.image_renderer
            .preload_images(&self.movies.iter().map(|x| x.id.tmdb).collect::<Vec<_>>());
        self.filter_sort_movies(false);
    }

    fn goto_movie(&mut self) {
        let search_text = &self.search_input.lines()[0];
        if search_text.is_empty() {
            return;
        }

        let mut conf = Config::DEFAULT;
        conf.prefer_prefix = true;
        let mut matcher = Matcher::new(conf);
        let pattern = Atom::parse(
            search_text,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Never,
        );
        let mut scores = vec![];
        for movie in &self.filtered_movies {
            if let Some(score) = pattern.score(
                nucleo_matcher::Utf32Str::Ascii(
                    (movie.name.clone() + " " + &movie.year)
                        .to_string()
                        .as_bytes(),
                ),
                &mut matcher,
            ) {
                scores.push((score, movie));
            }
        }

        scores.sort_by_key(|x| x.0);
        scores.reverse();

        if let Some(&(_, movie)) = scores.first() {
            let index = self
                .filtered_movies
                .iter()
                .position(|x| x == movie)
                .unwrap();

            self.movies_list_selected_item = index;
            self.movies_list_scroll_pos = index
                .saturating_sub(self.movies_list_num_visible_items / 2)
                .min(
                    self.filtered_movies
                        .len()
                        .saturating_sub(self.movies_list_num_visible_items),
                );
        }
    }

    fn filter_movies(&mut self) {
        let search_text = &self.search_input.lines()[0];
        if search_text.is_empty() {
            self.filtered_movies = self.movies.iter().map(|x| x.clone()).collect();
            return;
        }

        let mut conf = Config::DEFAULT;
        conf.prefer_prefix = true;
        let mut matcher = Matcher::new(conf);
        let pattern = Atom::parse(
            search_text,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Never,
        );
        let mut scores = vec![];
        for movie in &self.movies {
            if let Some(score) = pattern.score(
                nucleo_matcher::Utf32Str::Ascii(
                    (movie.name.clone() + " " + &movie.year)
                        .to_string()
                        .as_bytes(),
                ),
                &mut matcher,
            ) {
                scores.push((score, movie));
            }
        }

        self.filtered_movies = if let Sort::Relevance = self.sort {
            scores.sort_by_key(|x| x.0);
            if !self.sort_ascending {
                scores.reverse();
            }
            scores.iter().map(|&(_, movie)| movie.clone()).collect()
        } else {
            scores.iter().map(|&(_, movie)| movie.clone()).collect()
        }
    }

    fn sort_movies(&mut self) {
        match self.sort {
            Sort::UserRating => {
                self.filtered_movies.sort_by(|x, y| {
                    x.get_user_rating()
                        .partial_cmp(&y.get_user_rating())
                        .unwrap()
                });
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Rating => {
                self.filtered_movies.sort_by(MainScreen::cmp_ratings);
                if !self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::Name => {
                self.filtered_movies.sort_by_key(|x| x.name.clone());
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::ReleaseDate => {
                self.filtered_movies.sort_by_key(|x| x.year.clone());
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            Sort::AddedDate => {
                self.filtered_movies.sort_by_key(|x| {
                    x.plays
                        .last()
                        .map(|y| y.0)
                        .unwrap_or(chrono::DateTime::default())
                        .clone()
                });
                if self.sort_ascending {
                    self.filtered_movies.reverse();
                }
            }
            _ => (),
        }
    }

    fn filter_sort_movies(&mut self, reset: bool) {
        let selected_movie_id = self
            .current_movie()
            .map(|x| x.id.imdb.clone())
            .unwrap_or("".into());

        self.filter_movies();

        match self.sort {
            // Sort::AddedDate => {
            //     if self.sort_ascending {
            //         self.filtered_movies.reverse();
            //     }
            // }
            Sort::Relevance => {}
            _ => {
                self.sort_movies();
            }
        }

        if reset {
            let pos = self
                .filtered_movies
                .iter()
                .position(|x| x.id.imdb == selected_movie_id);
            if let Some(index) = pos {
                self.movies_list_selected_item = index;
                self.movies_list_scroll_pos = index
                    .saturating_sub(self.movies_list_num_visible_items / 2)
                    .min(
                        self.filtered_movies
                            .len()
                            .saturating_sub(self.movies_list_num_visible_items),
                    );
            } else {
                self.movies_list_selected_item = 0;
                self.movies_list_scroll_pos = 0;
            }
        }
    }

    fn cmp_ratings(a: &Movie, b: &Movie) -> Ordering {
        let mut rating_a: f64 = 0.0;
        let mut rating_b: f64 = 0.0;

        for i in (0..a.ratings.len()).rev() {
            if let Rating::IMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::IMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::Trakt(r_a, c_a) = a.ratings[i] {
                if let Rating::Trakt(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::TMDB(r_a, c_a) = a.ratings[i] {
                if let Rating::TMDB(r_b, c_b) = b.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
        }

        rating_a.partial_cmp(&rating_b).unwrap()
    }
}
