use itertools::Itertools;
use log::error;
use ratatui::{
    Frame,
    buffer::Cell,
    layout::{Offset, Rect, Size},
    style::{Modifier, Stylize, palette::tailwind},
    symbols::border,
    widgets::{Clear, Fill, Padding},
};
use webbrowser;

use crate::{
    helpers,
    image_backend::RatatuiImage,
    key_event_handler::{self, KeyEventHandler},
    popups::{Popup, PopupTrait},
    screens::Screen,
    types::Movie,
};

const MAX_NUM_ITEMS: usize = usize::MAX;
const POPUP_OFFSET: Offset = Offset {
    x: crate::screens::main_screen::LIST_POSTER_WIDTH as i32 + 3,
    y: 1,
};

#[derive(Default)]
pub struct OpenPopup {
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
        key_event_handler: &mut KeyEventHandler,
        _image_renderer: &mut RatatuiImage,
    ) {
        key_event_handler.clear();
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        let model_len = self.model.len();
        key_event_handler.bind_vertical((None, None), "Scroll".into(), move |app, data| {
            if let Some(Popup::Open(open_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    key_event_handler::Data::Direction(dir, _) =>
                        if dir {
                            open_popup.item += 1;
                            if open_popup.item >= model_len {
                                open_popup.item = model_len - 1;
                            }
                        } else {
                            open_popup.item = open_popup.item.saturating_sub(1);
                        },
                    _ => (),
                }
            }
        });
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
            .resize(Size::new(23, self.model.len().min(MAX_NUM_ITEMS) as u16));
        frame.render_widget(Clear, popup_area);
        let mut area = popup_area.resize(Size {
            width:  popup_area.width,
            height: 2,
        });
        for (i, item) in self.model.iter().enumerate() {
            let alt = i & 1 == 1;
            let selected = i == self.item;
            let (top_color, (bg_color, fg_color)) = (
                if i == 0 {
                    tailwind::VIOLET.c950
                } else if i - 1 == self.item {
                    tailwind::INDIGO.c500
                } else if !alt {
                    tailwind::SLATE.c800
                } else {
                    tailwind::SLATE.c900
                },
                if selected {
                    (tailwind::INDIGO.c500, tailwind::INDIGO.c200)
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

                if i == self.model.len() - 1 {
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

            if i == self.model.len() - 1 {
                frame.render_widget(
                    Fill::new(border::QUADRANT_BOTTOM_HALF)
                        .bg(bg_color)
                        .fg(tailwind::VIOLET.c950),
                    helpers::add_padding(area.offset(Offset::new(0, 2)), Padding::new(1, 1, 0, 1)),
                );
            }

            let input_area = if i == self.model.len() - 1 {
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
                            open_popup.item = i;
                        }
                    }
                },
            );

            area = area.offset(Offset { x: 0, y: 2 });
        }
    }
}
