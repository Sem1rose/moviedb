use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use log::{error, info};
use ratatui::{
    Frame,
    layout::{Margin, Offset, Rect, Size},
    macros::{constraint, horizontal, line, vertical},
    style::{Stylize, palette::tailwind},
    symbols::{block, border},
    widgets::{Block, Borders, Fill, Padding},
};
use ratatui_image::sliced::SignedPosition;
use throbber_widgets_tui::ThrobberState;
use tmdb::smo::MovieDetails as TMDBMovieDetails;

use crate::{
    helpers,
    image_backend::{ImageID, RatatuiImage},
    key_event_handler::{Data, KeyEventHandler},
    popups::{Popup, PopupTrait},
    types::Movie,
    widgets::{self, Direction, ScrollGallery},
};

const POSTER_SIZE: Size = Size {
    width:  13,
    height: 9,
};
const BACKDROP_SIZE: Size = Size {
    width:  21,
    height: 7,
};
const POSTER_WINDOW_SIZE: Size = Size {
    width:  55,
    height: 29,
};
const BACKDROP_WINDOW_SIZE: Size = Size {
    width:  66,
    height: 24,
};
#[derive(Default)]
pub struct ChangeArtworksPopup {
    item:           usize,
    backdrops:      bool,
    tick:           u64,
    throbber_state: ThrobberState,
    drawing_images: bool,

    chosen_poster:   Option<String>,
    chosen_backdrop: Option<String>,

    gallery: ScrollGallery,

    movie_images:        Option<TMDBMovieDetails>,
    rx_details_response: Option<Receiver<anyhow::Result<TMDBMovieDetails>>>,

    movie_id:          u32,
    tmdb_access_token: String,
}

impl ChangeArtworksPopup {
    pub fn new(movie: &Movie, tmdb_access_token: &str) -> Self {
        Self {
            chosen_poster: movie.override_poster.clone(),
            chosen_backdrop: movie.override_backdrop.clone(),

            gallery: ScrollGallery::new(POSTER_SIZE),

            ..Default::default()
        }
        // .start_thread(movie, tmdb_access_token)
    }

    fn start_thread(mut self, movie: &Movie, tmdb_access_token: &str) -> Self {
        self.movie_id = movie.id;
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
        (None, Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.drawing_images || self.movie_images.is_none()
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
                    if let Some(Popup::ChangeArtworks(change_artwork_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        if change_artwork_popup.backdrops != tab {
                            change_artwork_popup.gallery =
                                ScrollGallery::new(if tab { BACKDROP_SIZE } else { POSTER_SIZE });
                            change_artwork_popup.backdrops = tab;
                        }
                    }
                },
            );
            area = area
                .offset(Offset::new(area.width as i32, 0))
                .resize(Size::new(1, 2));
        }

        key_event_handler.bind_tab((None, None), "Change tab".into(), |app, _| {
            if let Some(Popup::ChangeArtworks(change_artwork_popup)) =
                app.drawer.active_popup.as_mut()
            {
                change_artwork_popup.backdrops ^= true;
                change_artwork_popup.gallery =
                    ScrollGallery::new(if change_artwork_popup.backdrops {
                        BACKDROP_SIZE
                    } else {
                        POSTER_SIZE
                    });
            }
        });

        key_event_handler.bind_horizontal((None, None), "Scroll".into(), |app, data| {
            if let Some(Popup::ChangeArtworks(change_artwork_popup)) =
                app.drawer.active_popup.as_mut()
            {
                change_artwork_popup.gallery.scroll(
                    match data {
                        Data::Direction(b, _) =>
                            if b {
                                Direction::Right
                            } else {
                                Direction::Left
                            },
                        _ => unreachable!(),
                    },
                    18,
                );
            }
        });

        key_event_handler.bind_vertical((None, None), "Scroll".into(), |app, data| {
            if let Some(Popup::ChangeArtworks(change_artwork_popup)) =
                app.drawer.active_popup.as_mut()
            {
                change_artwork_popup.gallery.scroll(
                    match data {
                        Data::Direction(b, _) =>
                            if b {
                                Direction::Down
                            } else {
                                Direction::Up
                            },
                        _ => unreachable!(),
                    },
                    18,
                );
            }
        });

        let scrollbar_area = main_area
            .offset(Offset::new(main_area.width as i32 - 1, 0))
            .resize(Size::new(1, main_area.height));
        // if let Some(TMDBMovieDetails {
        //     poster_path,
        //     backdrop_path,
        //     images: Some(images),
        //     ..
        // }) = self.movie_images.as_ref()
        // {
        self.drawing_images = false;

        self.gallery.render(
            18,
            main_area,
            scrollbar_area,
            frame,
            key_event_handler,
            |gallery, area, index, hidden_height, selected, alternate, frame, key_event_handler| {
                frame.render_widget(
                    Block::new().bg(if selected {
                        tailwind::RED.c500
                    } else if alternate {
                        tailwind::SLATE.c950
                    } else {
                        tailwind::BLACK
                    }),
                    area,
                );
                self.drawing_images |= image_renderer.draw_image(
                    ImageID::Custom(
                        None,
                        if self.backdrops {
                            if alternate {
                                "/iImUFYvuwDHildoANGXpvA30ROO.jpg".to_string()
                            } else {
                                "/qTdCfGyDisY9e8BLycszlyTsPWx.jpg".to_string()
                            }
                        } else {
                            if alternate {
                                "/r9utEhMKiaXUj0Bi6iAa3Yr5hrL.jpg".to_string()
                            } else {
                                "/vQoAq0etXKgEB67iFYVYZoTlKTR.jpg".to_string()
                            }
                        },
                        self.backdrops,
                    ),
                    helpers::add_padding(area, Padding::uniform(1)),
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
            },
        );
        // if self.backdrops {

        // for (row, &vert) in vertical![==5; 2].split(main_area).into_iter().enumerate() {
        //     for (col, &area) in horizontal![==17; 3].split(vert).into_iter().enumerate() {
        //         // if row * 7 + col > images.backdrops.len() {
        //         //     break;
        //         // }

        //         image_renderer.draw_image(
        //             ImageID::Custom(
        //                 None,
        //                 "/qTdCfGyDisY9e8BLycszlyTsPWx.jpg".to_string(),
        //                 // images.backdrops[row * 7 + col].file_path.clone(),
        //                 true,
        //             ),
        //             area,
        //             true,
        //             None,
        //             &mut self.throbber_state,
        //             frame,
        //         );
        //     }
        // }
        // } else {
        // for (row, &vert) in vertical![==7; 2].split(main_area).into_iter().enumerate() {
        //     for (col, &area) in horizontal![==9; 6].split(vert).into_iter().enumerate() {
        //         // if row * 7 + col > images.posters.len() {
        //         //     break;
        //         // }

        //         image_renderer.draw_image(
        //             ImageID::Custom(
        //                 None,
        //                 "/vQoAq0etXKgEB67iFYVYZoTlKTR.jpg".to_string(),
        //                 // images.posters[row * 7 + col].file_path.clone(),
        //                 false,
        //             ),
        //             area,
        //             true,
        //             None,
        //             &mut self.throbber_state,
        //             frame,
        //         );
        //     }
        // }
        // }
        // }
    }
}
