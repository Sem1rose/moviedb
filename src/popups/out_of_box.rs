use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Alignment, Layout, Margin, Offset, Rect, Size},
    macros::{constraints, horizontal, span, text, vertical},
    style::{
        Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::{Block, Padding},
};

use crate::{
    app::App,
    helpers::{add_padding, create_popup, static_area},
    key_event_handler::{self, Data, KeyEventHandler},
    popups::{
        FetchArtworksPopup, OMDBInitPopup, PopupTrait, Popups, TMDBInitPopup, TraktInitPopup,
    },
    widgets::{self, Action, ActionTypes},
};

pub struct OutOfBoxPopup {
    tab:              usize,
    item:             usize,
    throbber_visible: bool,
    toggled_list:     [bool; 3],
}

impl OutOfBoxPopup {
    pub fn new() -> Self {
        Self {
            tab:              0,
            item:             0,
            throbber_visible: false,
            toggled_list:     [false; 3],
        }
    }
}

const COLUMNS: usize = 2;
const NUM_REQUIRED_CHOICES: usize = 2;
impl PopupTrait for OutOfBoxPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        self.throbber_visible
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_key((None, None), 'q', "Quit".into(), |app, _| {
            app.quit = true;
        });
        key_event_handler.bind_key((Some(0), None), 'a', "Toggle all".into(), |app, _| {
            if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                if out_of_box_popup.toggled_list.contains(&false) {
                    out_of_box_popup.toggled_list = [true; 3];
                } else {
                    out_of_box_popup.toggled_list = [false; 3];
                }
            }
        });

        let confirm_fn = move |app: &mut App, _: Data| {
            let mut popups = vec![];
            let config = app.config.clone();
            if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                if out_of_box_popup.toggled_list[0] {
                    popups.push(Popups::TraktInit(TraktInitPopup::new(&app.home_dir, false)));
                    config.borrow_mut().options.trakt_enabled = true;
                }
                if out_of_box_popup.toggled_list[1] {
                    popups.push(Popups::TMDBInit(TMDBInitPopup::new(&app.home_dir, false)));
                    config.borrow_mut().options.tmdb_enabled = true;
                }
                if out_of_box_popup.toggled_list[2] {
                    popups.push(Popups::OMDBInit(OMDBInitPopup::new(&app.home_dir, false)));
                    config.borrow_mut().options.omdb_enabled = true;
                }

                popups.push(Popups::FetchArtworks(FetchArtworksPopup::new(
                    &app.cache_dir,
                )));
            }
            config.borrow_mut().options.oob_done = true;

            app.drawer.popup_queue.extend(popups);
            app.drawer.close_popup();
        };

        if !self.toggled_list[..NUM_REQUIRED_CHOICES].contains(&true) {
            if self.item < NUM_REQUIRED_CHOICES {
                self.toggled_list[self.item] = true;
            } else {
                self.toggled_list[0] = true;
            }
        }

        key_event_handler.bind_enter((None, None), "Confirm".into(), confirm_fn);
        key_event_handler.bind_tab((None, None), "".into(), move |app, data| {
            if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        out_of_box_popup.tab += 1;
                        if out_of_box_popup.tab > 1 {
                            out_of_box_popup.tab = 0;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        out_of_box_popup.tab = out_of_box_popup.tab.checked_sub(1).unwrap_or(1);
                    }
                    _ => {}
                }
            }
        });
        let toggle_block = |area: Rect,
                            frame: &mut Frame,
                            key_event_handler: &mut KeyEventHandler,
                            text: &'static str,
                            i: usize| {
            let tab_selected = self.tab == 0;
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                area,
                move |app, _| {
                    if let Some(Popups::OutOfBox(out_of_box_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        out_of_box_popup.tab = 0;
                        out_of_box_popup.item = i;

                        out_of_box_popup.toggled_list[i] ^= true;
                        if i < NUM_REQUIRED_CHOICES
                            && !out_of_box_popup.toggled_list[..NUM_REQUIRED_CHOICES]
                                .contains(&true)
                        {
                            let mut x = i + 1;
                            x = (NUM_REQUIRED_CHOICES - 1)
                                - (NUM_REQUIRED_CHOICES - 1)
                                    .checked_sub(x)
                                    .unwrap_or(NUM_REQUIRED_CHOICES - 1);

                            out_of_box_popup.toggled_list[x] = true;
                        }
                    }
                },
            );
            key_event_handler.bind_key((Some(0), Some(i)), ' ', "Toggle".into(), move |app, _| {
                if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                    out_of_box_popup.toggled_list[i] ^= true;
                    if i < NUM_REQUIRED_CHOICES
                        && !out_of_box_popup.toggled_list[..NUM_REQUIRED_CHOICES].contains(&true)
                    {
                        let mut x = i + 1;
                        x = (NUM_REQUIRED_CHOICES - 1)
                            - (NUM_REQUIRED_CHOICES - 1)
                                .checked_sub(x)
                                .unwrap_or(NUM_REQUIRED_CHOICES - 1);

                        out_of_box_popup.toggled_list[x] = true;
                    }
                }
            });

            let selected = self.item == i;
            let toggled = self.toggled_list[i];
            let (bg_color, highlight_color, text_color) = match (selected, toggled) {
                (false, false) => (tailwind::SKY.c900, tailwind::RED.c500, tailwind::STONE.c300),
                (false, true) => (
                    tailwind::EMERALD.c500,
                    tailwind::RED.c500,
                    tailwind::STONE.c800,
                ),
                (true, false) => (
                    tailwind::SKY.c800,
                    tailwind::EMERALD.c400,
                    tailwind::STONE.c300,
                ),
                (true, true) => (
                    tailwind::EMERALD.c400,
                    tailwind::BLUE.c500,
                    tailwind::STONE.c800,
                ),
            };

            let a_block = Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(bg_color);
            frame.render_widget(Block::new().bg(bg_color), a_block.inner(area));
            frame.render_widget(&a_block, area);
            if selected && tab_selected {
                frame.render_widget(
                    text![span!("▐"); area.height as usize - 2]
                        .fg(highlight_color)
                        .bg(bg_color),
                    area.resize(Size::new(1, area.height - 2))
                        .offset(Offset::new(0, 1)),
                );
                frame.render_widget(
                    text![span!("▌"); area.height as usize - 2]
                        .fg(highlight_color)
                        .bg(bg_color),
                    area.resize(Size::new(1, area.height - 2))
                        .offset(Offset::new(area.width as i32 - 1, 1)),
                );
            }

            let [check_box_area, _, text_area] = horizontal![==4, ==2, >=1]
                .areas(add_padding(a_block.inner(area), Padding::horizontal(1)));
            let checkbox_block = Block::bordered()
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(tailwind::INDIGO.c950);
            frame.render_widget(&checkbox_block, check_box_area);
            frame.render_widget(
                Block::new().bg(if toggled {
                    tailwind::SKY.c500
                } else {
                    tailwind::INDIGO.c950
                }),
                checkbox_block.inner(check_box_area),
            );

            frame.render_widget(
                text.fg(text_color).bold(),
                text_area
                    .resize(Size::new(text_area.width, 1))
                    .offset(Offset::new(0, text_area.height as i32 / 2)),
            );
        };

        self.throbber_visible = false;
        let popup_area = create_popup(
            frame,
            static_area(
                (4 + NUM_REQUIRED_CHOICES.div_ceil(COLUMNS) * 5
                    + (self.toggled_list.len() - NUM_REQUIRED_CHOICES).div_ceil(COLUMNS) * 5)
                    as u16
                    + 2,
                55,
                frame.area(),
            ),
            "  Choose Backends  ",
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
        let [_, mut remaining_area] =
            vertical![==1, >=1].areas(add_padding(popup_area, Padding::horizontal(2)));

        let mut table_indices = vec![];
        let labels = ["Trakt", "TMDB", "OMDB"];
        let mut required_header = false;
        let mut optional_header = false;
        let mut i = 0;
        loop {
            if i == self.toggled_list.len() {
                break;
            }

            if i == 0 && !required_header {
                let [area, remaining] = vertical![==1, >=1].areas(remaining_area);
                frame.render_widget(span!("Required:") + "(at least one)".dim().italic(), area);
                remaining_area = remaining;
                required_header = true;
            } else if i == NUM_REQUIRED_CHOICES && !optional_header {
                let [area, remaining] = vertical![==1, >=1].areas(remaining_area);
                frame.render_widget(span!("Optional:"), area);
                remaining_area = remaining;
                optional_header = true;
            } else {
                table_indices.push([None; COLUMNS]);
                let [area, remaining] = vertical![==5, >=1].areas(remaining_area);
                let constraints = (0..COLUMNS)
                    .map(|_| constraints![>=1, ==2])
                    .flatten()
                    .dropping_back(1)
                    .collect_vec();
                let areas = Layout::horizontal(constraints)
                    .split(area)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(n, &x)| if n & 1 == 0 { Some(x) } else { None })
                    .collect_vec();

                for (col, area) in areas.into_iter().enumerate() {
                    table_indices.last_mut().unwrap()[col] = Some(i);
                    toggle_block(area, frame, key_event_handler, labels[i], i);

                    i += 1;
                    if i == NUM_REQUIRED_CHOICES || i == self.toggled_list.len() {
                        break;
                    }
                }

                remaining_area = remaining;
            }
        }

        let table_indices_cloned = table_indices.clone();
        key_event_handler.bind_vertical((Some(0), None), "Scroll".into(), move |app, data| {
            if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                let row = (out_of_box_popup.item
                    - if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES
                    } else {
                        0
                    })
                    % COLUMNS;
                let col = (out_of_box_popup.item
                    - if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES
                    } else {
                        0
                    })
                    / COLUMNS
                    + if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES.div_ceil(COLUMNS)
                    } else {
                        0
                    };

                match data {
                    key_event_handler::Data::Direction(false, _) =>
                        if col > 0 {
                            for i in 0..=row {
                                if let Some(index) = table_indices_cloned[col - 1][row - i] {
                                    out_of_box_popup.item = index;
                                    break;
                                }
                            }
                        },
                    key_event_handler::Data::Direction(true, _) =>
                        if col < table_indices_cloned.len() - 1 {
                            for i in 0..=row {
                                if let Some(index) = table_indices_cloned[col + 1][row - i] {
                                    out_of_box_popup.item = index;
                                    break;
                                }
                            }
                        },
                    _ => (),
                }
            }
        });
        key_event_handler.bind_horizontal((Some(0), None), "Scroll".into(), move |app, data| {
            if let Some(Popups::OutOfBox(out_of_box_popup)) = app.drawer.active_popup.as_mut() {
                let row = (out_of_box_popup.item
                    - if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES
                    } else {
                        0
                    })
                    % COLUMNS;
                let col = (out_of_box_popup.item
                    - if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES
                    } else {
                        0
                    })
                    / COLUMNS
                    + if out_of_box_popup.item >= NUM_REQUIRED_CHOICES {
                        NUM_REQUIRED_CHOICES.div_ceil(COLUMNS)
                    } else {
                        0
                    };

                match data {
                    key_event_handler::Data::Direction(false, _) =>
                        if row > 0 {
                            out_of_box_popup.item = table_indices[col][row - 1].unwrap();
                        },
                    key_event_handler::Data::Direction(true, _) =>
                        if row < COLUMNS - 1 {
                            if let Some(index) = table_indices[col][row + 1] {
                                out_of_box_popup.item = index;
                            }
                        },
                    _ => (),
                }
            }
        });

        let confirm_mouse_area = widgets::action(
            Action::new(" Confirm ", ActionTypes::Default, self.tab == 1, true),
            ratatui::layout::HorizontalAlignment::Right,
            remaining_area,
            frame,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            confirm_mouse_area,
            confirm_fn,
        );
    }
}
