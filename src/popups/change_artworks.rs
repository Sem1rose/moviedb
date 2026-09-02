use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use log::error;
use ratatui::{
    Frame,
    layout::{HorizontalAlignment, Margin, Offset, Rect, Size},
    macros::{constraint, line, vertical},
    style::{Style, Stylize, palette::tailwind},
    symbols::border,
    widgets::{Block, Borders, Fill, Padding},
};
use ratatui_image::sliced::SignedPosition;
use throbber_widgets_tui::{Throbber, ThrobberState, symbols::throbber};
use tmdb::smo::MovieDetails as TMDBMovieDetails;

use crate::{
    helpers,
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{Data, KeyEventHandler},
    popups::{Popup, PopupTrait},
    types::Movie,
    widgets::{self, Action, ActionType, Direction, ScrollGallery},
};

const POSTER_SIZE: Size = Size {
    width:  16,
    height: 11,
};
const BACKDROP_SIZE: Size = Size {
    width:  25,
    height: 8,
};
const POSTER_WINDOW_SIZE: Size = Size {
    width:  67,
    height: 33,
};
const BACKDROP_WINDOW_SIZE: Size = Size {
    width:  78,
    height: 27,
};
#[derive(Default)]
pub struct ChangeArtworksPopup {
    backdrops:      bool,
    tick:           u64,
    throbber_state: ThrobberState,
    images_drawn:   bool,

    pub chosen_poster:   usize,
    pub chosen_backdrop: usize,

    movie_override_poster:   Option<String>,
    movie_override_backdrop: Option<String>,

    gallery: ScrollGallery,

    pub movie_images:    Option<TMDBMovieDetails>,
    rx_details_response: Option<Receiver<anyhow::Result<TMDBMovieDetails>>>,

    pub movie_id:      u32,
    tmdb_access_token: String,
}

impl ChangeArtworksPopup {
    pub fn new(movie: &Movie, tmdb_access_token: &str) -> Self {
        Self {
            movie_override_backdrop: movie.override_backdrop.clone(),
            movie_override_poster: movie.override_poster.clone(),
            movie_id: movie.id,

            gallery: ScrollGallery::new(POSTER_SIZE),

            ..Default::default()
        }
        .start_thread(tmdb_access_token)
    }

    fn start_thread(mut self, tmdb_access_token: &str) -> Self {
        self.tmdb_access_token = tmdb_access_token.to_string();

        let (tx_details_response, rx_details_response) = mpsc::channel();
        let movie_id = self.movie_id;
        let access_token = self.tmdb_access_token.clone();
        thread::spawn(move || {
            tx_details_response.send(tmdb::movie::get_movie_images(&access_token, movie_id))
        });
        self.rx_details_response = Some(rx_details_response);

        self
    }
}

impl PopupTrait for ChangeArtworksPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (None, None)
    }

    fn update_next_frame(&self) -> bool {
        !self.images_drawn || self.movie_images.is_none()
    }

    fn update(&mut self) {
        self.tick += 1;
        if self.tick & 7 == 0 {
            self.throbber_state.calc_next();
        }

        if self.movie_images.is_none() {
            if let Some(rx_details_response) = self.rx_details_response.as_ref() {
                if let Ok(result) = rx_details_response.try_recv() {
                    match result {
                        Ok(images) => {
                            if let Some(backdrop) = self.movie_override_backdrop.take() {
                                if let Some(position) = images
                                    .images
                                    .as_ref()
                                    .map(|images| {
                                        images
                                            .backdrops
                                            .iter()
                                            .position(|x| x.file_path == backdrop)
                                    })
                                    .flatten()
                                {
                                    self.chosen_backdrop = position + 1;
                                }
                            }
                            if let Some(poster) = self.movie_override_poster.take() {
                                if let Some(position) = images
                                    .images
                                    .as_ref()
                                    .map(|images| {
                                        images.posters.iter().position(|x| x.file_path == poster)
                                    })
                                    .flatten()
                                {
                                    self.chosen_poster = position + 1;
                                }
                            }

                            self.movie_images = Some(images);
                            _ = self.rx_details_response.take();
                        }
                        Err(error) => {
                            error!("Error while getting movie images: {error:?}");

                            let (tx_details_response, rx_details_response) = mpsc::channel();
                            let movie_id = self.movie_id;
                            let access_token = self.tmdb_access_token.clone();
                            thread::spawn(move || {
                                thread::sleep(Duration::from_secs(2));
                                tx_details_response
                                    .send(tmdb::movie::get_movie_images(&access_token, movie_id))
                            });
                            self.rx_details_response = Some(rx_details_response);
                        }
                    }
                }
            }
        }
    }

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
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        key_event_handler.bind_tab((None, None), "Change tab".into(), |app, _| {
            if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                app.drawer.active_popup.as_mut()
            {
                change_artworks_popup.backdrops ^= true;
                change_artworks_popup.gallery =
                    ScrollGallery::new(if change_artworks_popup.backdrops {
                        BACKDROP_SIZE
                    } else {
                        POSTER_SIZE
                    });
                change_artworks_popup.gallery.selected_index = if change_artworks_popup.backdrops {
                    change_artworks_popup.chosen_backdrop
                } else {
                    change_artworks_popup.chosen_poster
                };
            }
        });

        let popup_area = widgets::window(
            frame,
            if self.backdrops {
                helpers::centered_area(
                    BACKDROP_WINDOW_SIZE.height,
                    BACKDROP_WINDOW_SIZE.width,
                    frame.area(),
                )
            } else {
                helpers::centered_area(
                    POSTER_WINDOW_SIZE.height,
                    POSTER_WINDOW_SIZE.width,
                    frame.area(),
                )
            },
            " Change Artworks ",
            false,
        );
        image_renderer.add_overlay(popup_area.outer(Margin::new(1, 1)));
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );

        let [tabs_area, main_area] = vertical![==4, >=1].areas(popup_area);

        frame.render_widget(Block::new().bg(tailwind::GRAY.c900), tabs_area);
        frame.render_widget(
            Block::bordered()
                .borders(Borders::TOP)
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(tailwind::VIOLET.c950),
            helpers::add_padding(tabs_area, Padding::top(3)),
        );

        let mut area = Rect::new(tabs_area.x, tabs_area.y + 1, tabs_area.width, 3)
            .centered_horizontally(constraint!(==22));

        let tabs = ["Poster", "Backdrop"];
        for tab in [false, true] {
            if tab == self.backdrops {
                area = area.resize(Size::new(12, 3));

                let block = Block::bordered()
                    .borders(!Borders::BOTTOM)
                    .border_set(border::PROPORTIONAL_TALL)
                    .fg(tailwind::VIOLET.c950)
                    .bg(tailwind::BLUE.c950);
                frame.render_widget(&block, area);
                frame.render_widget(
                    line!(tabs[tab as usize])
                        .centered()
                        .bold()
                        .fg(tailwind::ORANGE.c500),
                    block.inner(area),
                );

                frame.render_widget(
                    Fill::new(" ").bg(tailwind::BLUE.c950),
                    block
                        .inner(area)
                        .offset(Offset::new(0, 1))
                        .resize(Size::new(area.width - 2, 1)),
                );
            } else {
                area = area.resize(Size::new(10, 3));

                let block = Block::bordered()
                    .borders(!Borders::BOTTOM)
                    .border_set(border::PROPORTIONAL_WIDE)
                    .fg(tailwind::GRAY.c800);
                frame.render_widget(Block::new().bg(tailwind::GRAY.c800), block.inner(area));
                frame.render_widget(&block, area);
                frame.render_widget(
                    line!(tabs[tab as usize]).centered().fg(tailwind::GRAY.c400),
                    block.inner(area),
                );

                frame.render_widget(
                    Fill::new(border::QUADRANT_BOTTOM_HALF)
                        .fg(tailwind::VIOLET.c950)
                        .bg(tailwind::GRAY.c800),
                    area.offset(Offset::new(0, 2))
                        .resize(Size::new(area.width, 1)),
                );
            }

            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                area,
                move |app, _| {
                    if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if change_artworks_popup.backdrops != tab {
                            change_artworks_popup.backdrops = tab;
                            change_artworks_popup.gallery =
                                ScrollGallery::new(if change_artworks_popup.backdrops {
                                    BACKDROP_SIZE
                                } else {
                                    POSTER_SIZE
                                });
                            change_artworks_popup.gallery.selected_index =
                                if change_artworks_popup.backdrops {
                                    change_artworks_popup.chosen_backdrop
                                } else {
                                    change_artworks_popup.chosen_poster
                                };
                        }
                    }
                },
            );
            area = area
                .offset(Offset::new(area.width as i32, 0))
                .resize(Size::new(1, 2));
        }

        let actions_mouse_areas = widgets::actions(
            [
                Action::new("  ", ActionType::Normal, true, true),
                Action::new("  ", ActionType::Critical, true, true),
            ],
            HorizontalAlignment::Right,
            true,
            1,
            helpers::add_padding(tabs_area, Padding::uniform(1)),
            frame,
        );
        for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                mouse_area,
                move |app, _| {
                    if i == 0 {
                        app.change_movie_artworks();
                    }

                    app.drawer.close_popup();
                },
            );
        }

        self.images_drawn = true;
        if let Some(TMDBMovieDetails {
            poster_path,
            backdrop_path,
            images,
            ..
        }) = self.movie_images.as_ref()
        {
            let num_items = if self.backdrops {
                images
                    .as_ref()
                    .map(|x| x.backdrops.len())
                    .unwrap_or_default()
                    + backdrop_path.as_ref().map(|_| 1).unwrap_or_default()
            } else {
                images.as_ref().map(|x| x.posters.len()).unwrap_or_default()
                    + poster_path.as_ref().map(|_| 1).unwrap_or_default()
            };

            key_event_handler.bind_horizontal((None, None), "Scroll".into(), move |app, data| {
                if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    change_artworks_popup.gallery.scroll(
                        match data {
                            Data::Direction(b, _) =>
                                if b {
                                    Direction::Right
                                } else {
                                    Direction::Left
                                },
                            _ => unreachable!(),
                        },
                        num_items,
                    );
                }
            });
            key_event_handler.bind_vertical((None, None), "Scroll".into(), move |app, data| {
                if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    change_artworks_popup.gallery.scroll(
                        match data {
                            Data::Direction(b, _) =>
                                if b {
                                    Direction::Down
                                } else {
                                    Direction::Up
                                },
                            _ => unreachable!(),
                        },
                        num_items,
                    );
                }
            });
            key_event_handler.bind_key((None, None), ' ', "Select".into(), |app, _| {
                if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    let selected_index = change_artworks_popup.gallery.selected_index;
                    if change_artworks_popup.backdrops {
                        change_artworks_popup.chosen_backdrop = selected_index;
                    } else {
                        change_artworks_popup.chosen_poster = selected_index;
                    }
                }
            });
            key_event_handler.bind_enter((None, None), "Select & Confirm".into(), |app, _| {
                if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    let selected_index = change_artworks_popup.gallery.selected_index;
                    if change_artworks_popup.backdrops {
                        change_artworks_popup.chosen_backdrop = selected_index;
                    } else {
                        change_artworks_popup.chosen_poster = selected_index;
                    }

                    app.change_movie_artworks();
                }

                app.drawer.close_popup();
            });

            let scrollbar_area = main_area
                .offset(Offset::new(main_area.width as i32 - 1, 0))
                .resize(Size::new(1, main_area.height));

            self.gallery.render(
                num_items,
                main_area,
                scrollbar_area,
                frame,
                key_event_handler,
                |gallery,
                 area,
                 index,
                 hidden_height,
                 selected,
                 alternate,
                 frame,
                 key_event_handler| {
                    let active = if self.backdrops {
                        index == self.chosen_backdrop
                    } else {
                        index == self.chosen_poster
                    };

                    frame.render_widget(
                        Block::new().bg(if active {
                            tailwind::TEAL.c700
                        } else if alternate {
                            tailwind::SLATE.c950
                        } else {
                            tailwind::GRAY.c900
                        }),
                        area,
                    );
                    if selected {
                        // frame.render_widget(
                        //     Block::bordered().border_set(border::PROPORTIONAL_WIDE).fg(
                        //         tailwind::SKY.c700
                        //     ),
                        //     helpers::add_padding(area, Padding::horizontal(1)),
                        // );
                        // frame.render_widget(
                        //     Block::new().bg(
                        //         tailwind::SKY.c700
                        //     ),
                        //     helpers::add_padding(area, Padding::proportional(1)),
                        // );
                        frame.render_widget(
                            Block::bordered()
                                .border_set(border::PROPORTIONAL_TALL)
                                .fg(if active {
                                    tailwind::SKY.c600
                                } else {
                                    tailwind::TEAL.c600
                                }),
                            area,
                        );
                    }
                    self.images_drawn &= image_renderer.draw_image(
                        ImageID::Custom(
                            if self.backdrops {
                                if index == 0 {
                                    backdrop_path.clone().unwrap()
                                } else if let Some(images) = images {
                                    images.backdrops[index - 1].file_path.clone()
                                } else {
                                    unreachable!()
                                }
                            } else {
                                if index == 0 {
                                    poster_path.clone().unwrap()
                                } else if let Some(images) = images {
                                    images.posters[index - 1].file_path.clone()
                                } else {
                                    unreachable!()
                                }
                            },
                            self.backdrops,
                        ),
                        helpers::add_padding(
                            area,
                            Padding::new(
                                2,
                                2,
                                if hidden_height > 0 && gallery.alignment_bottom {
                                    0
                                } else {
                                    1
                                },
                                if hidden_height > 0 && !gallery.alignment_bottom {
                                    0
                                } else {
                                    1
                                },
                            ),
                        ),
                        true,
                        if hidden_height > 0 {
                            Some(SignedPosition {
                                x: 0,
                                y: if gallery.alignment_bottom {
                                    -(hidden_height as i16 - 1)
                                } else {
                                    0
                                },
                            })
                        } else {
                            None
                        },
                        &mut self.throbber_state,
                        frame,
                    );

                    key_event_handler.bind_mouse_button_down(
                        ratatui::crossterm::event::MouseButton::Left,
                        area,
                        move |app, _| {
                            if let Some(Popup::ChangeArtworks(change_artworks_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if change_artworks_popup.gallery.selected_index != index {
                                    change_artworks_popup.gallery.goto_index(index, false, 18);
                                } else {
                                    if change_artworks_popup.backdrops {
                                        change_artworks_popup.chosen_backdrop = index;
                                    } else {
                                        change_artworks_popup.chosen_poster = index;
                                    }
                                }
                            }
                        },
                    );
                },
            );
        } else {
            frame.render_widget(Block::new().bg(tailwind::GRAY.c950), main_area);
            frame.render_stateful_widget(
                Throbber::default()
                    .throbber_set(throbber::BRAILLE_SIX_DOUBLE)
                    .style(Style::new().fg(tailwind::CYAN.c600).bold()),
                main_area.centered(constraint!(==1), constraint!(==1)),
                &mut self.throbber_state,
            );
        }
    }
}
