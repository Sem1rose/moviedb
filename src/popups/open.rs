use itertools::Itertools;
use log::error;
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Offset, Rect, Size},
    style::{Modifier, Stylize, palette::tailwind},
    symbols::border,
    widgets::{Clear, Fill, Padding},
};
use webbrowser;

use crate::{
    event_handler::{self, EventHandler},
    helpers,
    image_backend::RatatuiImage,
    popups::{Popup, PopupTrait},
    screens::Screen,
    types::Movie,
    widgets::{self, Orientation},
};

const MAX_NUM_ITEMS: usize = 4;
const POPUP_OFFSET: Offset = Offset {
    x: crate::screens::main_screen::LIST_POSTER_WIDTH as i32 + 3,
    y: 1,
};

#[derive(Default)]
pub struct OpenPopup {
    scroll_pos:          usize,
    item:                usize,
    selected_movie_area: Rect,
    selected_movie:      Movie,
    model:               Vec<String>,
}

impl OpenPopup {
    pub fn new(selected_movie: Movie, selected_movie_area: Rect) -> Self {
        Self {
            selected_movie_area,
            model: [
                "Collection".to_string(),
                "TMDB".to_string(),
                "PunchPlay".to_string(),
                "Simkl".to_string(),
            ]
            .into_iter()
            .filter(|x| match x.as_str() {
                "Collection" => selected_movie.tmdb_collection.is_some(),
                _ => true,
            })
            .collect_vec(),
            selected_movie,
            ..Default::default()
        }
    }

    fn open(&self) {
        match self.model[self.item].as_str() {
            "TMDB" => {
                if let Err(error) = webbrowser::open(
                    format!(
                        "https://www.themoviedb.org/movie/{}",
                        self.selected_movie.id
                    )
                    .as_str(),
                ) {
                    error!("Error while opening {}: {}", self.model[self.item], error);
                }
            }
            "PunchPlay" => {
                if let Err(error) = webbrowser::open(
                    format!(
                        "https://punchplay.tv/title/movie/{}",
                        self.selected_movie.id
                    )
                    .as_str(),
                ) {
                    error!("Error while opening {}: {}", self.model[self.item], error);
                }
            }
            "Simkl" => {
                if let Err(error) = webbrowser::open(
                    format!(
                        "https://api.simkl.com/redirect?to=Simkl&type=movie&tmdb={}",
                        self.selected_movie.id
                    )
                    .as_str(),
                ) {
                    error!("Error while opening {}: {}", self.model[self.item], error);
                }
            }
            _ => (),
        }
    }
}

impl PopupTrait for OpenPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, Some(self.item))
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut EventHandler,
        _image_renderer: &mut RatatuiImage,
    ) {
        key_event_handler.clear();
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'o', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        key_event_handler.bind_enter((None, None), "Open".into(), move |app, _| {
            if let Some(Popup::Open(open_popup)) = app.drawer.active_popup.as_ref() {
                if open_popup.model[open_popup.item] == "Collection" {
                    if let Some(Screen::MainScreen(main_screen)) =
                        app.drawer.current_screen.as_mut()
                    {
                        main_screen.try_open_temp_list(
                            crate::types::ListID::Collection(
                                open_popup.selected_movie.tmdb_collection.unwrap(),
                            ),
                            &mut app.key_event_handler,
                        );
                    }
                } else {
                    open_popup.open();
                }
                app.drawer.close_popup();
            }
        });

        let popup_area = self
            .selected_movie_area
            .offset(POPUP_OFFSET)
            .resize(Size::new(
                20,
                self.model.len().min(MAX_NUM_ITEMS) as u16 * 2 + 1,
            ));

        key_event_handler.bind_vertical(
            (None, None),
            "Scroll".into(),
            Some(popup_area),
            move |app, data| {
                if let Some(Popup::Open(open_popup)) = app.drawer.active_popup.as_mut() {
                    match data {
                        event_handler::Data::Direction(dir, _) =>
                            if dir {
                                if open_popup.item < open_popup.model.len() - 1 {
                                    let num_visible_items =
                                        MAX_NUM_ITEMS.min(open_popup.model.len());
                                    open_popup.item += 1;
                                    if open_popup.item < open_popup.scroll_pos
                                        || open_popup.item - open_popup.scroll_pos
                                            >= num_visible_items
                                    {
                                        open_popup.scroll_pos =
                                            open_popup.item.saturating_sub(num_visible_items - 1)
                                    }
                                }
                            } else {
                                if open_popup.item > 0 {
                                    open_popup.item -= 1;
                                    if open_popup.item < open_popup.scroll_pos {
                                        open_popup.scroll_pos -= 1
                                    }
                                }
                            },
                        _ => (),
                    }
                }
            },
        );

        frame.render_widget(Clear, popup_area);
        let mut area = popup_area.resize(Size {
            width:  popup_area.width,
            height: 2,
        });
        for (i, item) in self
            .model
            .iter()
            .dropping(self.scroll_pos)
            .take(MAX_NUM_ITEMS.min(self.model.len()))
            .enumerate()
        {
            let index = i + self.scroll_pos;
            let alt = index & 1 == 1;
            let selected = index == self.item;
            let (top_color, (bg_color, fg_color)) = (
                if i == 0 {
                    tailwind::VIOLET.c950
                } else if index - 1 == self.item {
                    tailwind::SKY.c600
                } else if !alt {
                    tailwind::SLATE.c800
                } else {
                    tailwind::SLATE.c900
                },
                if selected {
                    (tailwind::SKY.c600, tailwind::SKY.c200)
                } else if alt {
                    (tailwind::SLATE.c800, tailwind::SLATE.c300)
                } else {
                    (tailwind::SLATE.c900, tailwind::SLATE.c300)
                },
            );

            {
                let buf = frame.buffer_mut();
                let mut cell = Cell::new(border::QUADRANT_BLOCK);
                cell.set_fg(tailwind::VIOLET.c950);
                *buf.cell_mut(area.as_position()).unwrap() = cell.clone();
                *buf.cell_mut(
                    area.offset(Offset {
                        x: area.width as i32 - 1,
                        y: 0,
                    })
                    .as_position(),
                )
                .unwrap() = cell.clone();
                *buf.cell_mut(area.offset(Offset { x: 0, y: 1 }).as_position())
                    .unwrap() = cell.clone();
                *buf.cell_mut(
                    area.offset(Offset {
                        x: area.width as i32 - 1,
                        y: 1,
                    })
                    .as_position(),
                )
                .unwrap() = cell.clone();

                if i == self.model.len().min(MAX_NUM_ITEMS) - 1 {
                    *buf.cell_mut(area.offset(Offset { x: 0, y: 2 }).as_position())
                        .unwrap() = cell.clone();
                    *buf.cell_mut(
                        area.offset(Offset {
                            x: area.width as i32 - 1,
                            y: 2,
                        })
                        .as_position(),
                    )
                    .unwrap() = cell;
                }
            }

            frame.render_widget(
                Fill::new(border::QUADRANT_BOTTOM_HALF)
                    .bg(top_color)
                    .fg(bg_color),
                helpers::add_padding(area, Padding::new(1, 1, 0, 1)),
            );

            frame.render_widget(
                Fill::new(" ").bg(bg_color).remove_modifier(Modifier::all()),
                helpers::add_padding(area, Padding::new(1, 1, 1, 0)),
            );
            frame.render_widget(
                item.as_str().fg(fg_color).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
                helpers::add_padding(area, Padding::new(2, 1, 1, 0)),
            );

            let input_area = if i == self.model.len().min(MAX_NUM_ITEMS) - 1 {
                frame.render_widget(
                    Fill::new(border::QUADRANT_BOTTOM_HALF)
                        .bg(bg_color)
                        .fg(tailwind::VIOLET.c950),
                    helpers::add_padding(area.offset(Offset::new(0, 2)), Padding::new(1, 1, 0, 1)),
                );

                helpers::add_padding(area, Padding::horizontal(1)).resize(Size {
                    width:  area.width - 2,
                    height: 3,
                })
            } else {
                helpers::add_padding(area, Padding::horizontal(1))
            };
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                input_area,
                move |app, _| {
                    if let Some(Popup::Open(open_popup)) = app.drawer.active_popup.as_mut() {
                        if selected {
                            if open_popup.model[open_popup.item] == "Collection" {
                                if let Some(Screen::MainScreen(main_screen)) =
                                    app.drawer.current_screen.as_mut()
                                {
                                    main_screen.try_open_temp_list(
                                        crate::types::ListID::Collection(
                                            open_popup.selected_movie.tmdb_collection.unwrap(),
                                        ),
                                        &mut app.key_event_handler,
                                    );
                                }
                            } else {
                                open_popup.open();
                            }
                            app.drawer.close_popup();
                        } else {
                            open_popup.item = index;
                        }
                    }
                },
            );

            area = area.offset(Offset { x: 0, y: 2 });
        }
        if self.model.len() > MAX_NUM_ITEMS {
            let mut cell = Cell::new(" ");
            cell.set_fg(tailwind::VIOLET.c950);
            let mut buffer = Buffer::filled(
                popup_area
                    .offset(Offset::new(popup_area.width as i32 - 1, 0))
                    .resize(Size::new(1, popup_area.height)),
                cell,
            );
            widgets::scroll_bar(
                Orientation::Vertical,
                self.model.len(),
                self.scroll_pos,
                MAX_NUM_ITEMS,
                &mut buffer,
            );
            frame.buffer_mut().merge(&buffer);
        }
    }
}
