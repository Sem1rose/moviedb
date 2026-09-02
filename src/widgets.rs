use itertools::Itertools;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, HorizontalAlignment, Offset, Position, Rect, Size},
    macros::{horizontal, line, span, text, vertical},
    style::{
        Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Widget},
};
use ratatui_textarea::{TextArea, WrapMode};
use rustc_hash::FxHashMap;

use crate::{helpers::add_padding, key_event_handler::KeyEventHandler, types::FxIndexMap};

#[allow(clippy::too_many_arguments)]
pub fn input_field(
    tab_selected: bool,
    selected: bool,
    valid: bool,
    input: &mut TextArea<'static>,
    wrap_mode: WrapMode,
    frame: &mut Frame,
    area: Rect,
    title: &str,
    placeholder_text: &str,
    custom_padding: Option<Padding>,
) {
    input.set_style(Style::new().fg(if tab_selected && selected {
        tailwind::SLATE.c200
    } else {
        tailwind::STONE.c400
    }));
    input.set_cursor_style(
        Style::new()
            .fg(if tab_selected && selected {
                tailwind::SLATE.c300
            } else {
                tailwind::STONE.c400
            })
            .add_modifier(if tab_selected && selected {
                Modifier::REVERSED
            } else {
                Modifier::default()
            }),
    );
    input.set_block(
        Block::bordered()
            .border_type(ratatui::widgets::BorderType::Thick)
            .fg(if tab_selected {
                if selected {
                    if valid { material::BLUE.c500 } else { material::RED.c600 }
                } else {
                    tailwind::STONE.c500
                }
            } else {
                tailwind::STONE.c600
            })
            .title(title.to_string())
            .title_style(Style::new().fg(if tab_selected {
                if selected {
                    material::BLUE.c400
                } else {
                    if valid { material::BLUE.c600 } else { material::RED.c600 }
                }
            } else {
                tailwind::STONE.c400
            }))
            .padding(custom_padding.unwrap_or(Padding::symmetric(1, 0))),
    );
    input.set_placeholder_text(placeholder_text);
    input.set_placeholder_style(Style::new().fg(material::GRAY.c700));
    input.set_wrap_mode(wrap_mode);

    frame.render_widget(&*input, area);
}

pub fn dropdown(tab_selected: bool, selected: bool, frame: &mut Frame, area: Rect, text: String) {
    // "▼⬇⬆⏷▲▴▼▾◥◤◣◢⥡⥝⥜⥠🡙🢓🢑"
    let dropdown_block =
        Block::bordered()
            .border_set(border::PROPORTIONAL_WIDE)
            .fg(if tab_selected {
                if selected {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            });
    frame.render_widget(&dropdown_block, area);
    frame.render_widget(
        span!(text)
            .bold()
            .fg(if tab_selected {
                if selected {
                    material::TEAL.c100
                } else {
                    material::INDIGO.c200
                }
            } else {
                material::GRAY.c400
            })
            .bg(if tab_selected {
                if selected {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            }),
        dropdown_block.inner(area),
    );
    frame.render_widget(
        line!(" ▼")
            .right_aligned()
            .bold()
            .fg(if tab_selected {
                if selected {
                    material::TEAL.c100
                } else {
                    material::INDIGO.c200
                }
            } else {
                material::GRAY.c400
            })
            .bg(if tab_selected {
                if selected {
                    material::BLUE.c600
                } else {
                    material::INDIGO.c800
                }
            } else {
                tailwind::SLATE.c700
            }),
        dropdown_block.inner(area),
    );
}

#[derive(Default)]
pub struct ContextMenu {
    pub model:             FxIndexMap<usize, String>,
    pub selected_index:    usize,
    pub scroll_pos:        usize,
    pub num_visible_items: usize,
    pub opened_submenu:    Option<usize>,
    pub submenu_right:     bool,
    pub width:             u16,
    pub submenus:          FxIndexMap<usize, ContextMenu>,
}

impl ContextMenu {
    pub fn new(
        model: Vec<(usize, String)>,
        num_visible_items: usize,
        max_width: Option<u16>,
        submenu_right: bool,
    ) -> Self {
        Self {
            width: max_width
                .unwrap_or(model.iter().map(|(_, x)| x.len()).max().unwrap_or(0) as u16 + 4),
            model: model.into_iter().collect(),
            num_visible_items,
            submenu_right,

            ..Default::default()
        }
    }

    pub fn with_submenu(
        mut self,
        index: usize,
        model: Vec<(usize, String)>,
        num_visible_items: usize,
        max_width: Option<u16>,
    ) -> Self {
        self.add_submenu(index, model, num_visible_items, max_width);

        self
    }

    pub fn change_model(&mut self, new_model: Vec<(usize, String)>, max_width: Option<u16>) {
        self.width = max_width
            .unwrap_or(new_model.iter().map(|(_, x)| x.len()).max().unwrap_or(0) as u16 + 4);
        self.model = new_model.into_iter().collect();
        self.submenus.clear();
        self.selected_index = 0;
        self.scroll_pos = 0;
        self.opened_submenu = None;
    }

    pub fn add_submenu(
        &mut self,
        id: usize,
        model: Vec<(usize, String)>,
        num_visible_items: usize,
        max_width: Option<u16>,
    ) {
        self.submenus.insert(
            id,
            Self::new(model, num_visible_items, max_width, self.submenu_right),
        );
    }

    pub fn reset_state(&mut self) {
        self.scroll_pos = 0;
        self.selected_index = 0;
        self.opened_submenu = None;

        for submenu in self.submenus.values_mut() {
            submenu.reset_state();
        }
    }

    pub fn id_from_index(&self, index: usize) -> usize {
        let (&x, _) = self.model.get_index(index).unwrap();
        x
    }

    pub fn index_from_id(&self, id: usize) -> Option<usize> {
        self.model.keys().position(|x| *x == id)
    }

    pub fn open_submenu(&mut self, reset: bool) {
        if let Some(submenu_id) = self.opened_submenu.as_ref() {
            self.submenus
                .get_mut(submenu_id)
                .unwrap()
                .open_submenu(reset);
        } else {
            let x = self.id_from_index(self.selected_index);
            if self.submenus.contains_key(&x) {
                self.opened_submenu = Some(x);

                if reset {
                    self.submenus.get_mut(&x).unwrap().reset_state();
                }
            }
        }
    }

    pub fn close_submenu(&mut self) {
        if let Some(submenu_id) = self.opened_submenu.as_ref() {
            self.submenus.get_mut(submenu_id).unwrap().close_submenu();
            self.opened_submenu = None;
        }
    }

    pub fn scroll(&mut self, direction: bool) {
        if let Some(submenu_id) = self.opened_submenu.as_ref() {
            self.submenus.get_mut(submenu_id).unwrap().scroll(direction);
        } else if direction {
            if self.selected_index < self.model.len() - 1 {
                self.selected_index += 1;
                if self.selected_index < self.scroll_pos
                    || self.selected_index - self.scroll_pos >= self.num_visible_items
                {
                    self.scroll_pos = self
                        .selected_index
                        .saturating_sub(self.num_visible_items - 1)
                }
            }
        } else {
            if self.selected_index > 0 {
                self.selected_index -= 1;
                if self.selected_index < self.scroll_pos {
                    self.scroll_pos -= 1
                }
            }
        }
    }

    pub fn choose(&self) -> Vec<usize> {
        if let Some(submenu_id) = self.opened_submenu.as_ref() {
            [*submenu_id]
                .into_iter()
                .chain(self.submenus[submenu_id].choose())
                .collect()
        } else {
            vec![self.id_from_index(self.selected_index)]
        }
    }

    pub fn render(
        &mut self,
        position: Position,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> FxHashMap<Vec<usize>, (Rect, usize)> {
        let model_len = self.model.len();
        if self.selected_index < self.scroll_pos
            || self.selected_index - self.scroll_pos >= self.num_visible_items
        {
            self.scroll_pos = self
                .selected_index
                .saturating_sub(self.num_visible_items - 1)
        }
        if self.opened_submenu.is_some()
            && *self.opened_submenu.as_ref().unwrap() != self.id_from_index(self.selected_index)
        {
            _ = self.opened_submenu.take();
        }

        let mut visible_items = self
            .model
            .iter()
            .enumerate()
            .dropping(self.scroll_pos)
            .take(self.num_visible_items)
            .map(|(i, (_, x))| {
                line!(
                    if !self.submenu_right && self.submenus.contains_key(&self.id_from_index(i)) {
                        "<"
                    } else {
                        " "
                    },
                    x,
                    if self.submenu_right && self.submenus.contains_key(&self.id_from_index(i)) {
                        ">"
                    } else {
                        " "
                    },
                )
                .fg(material::INDIGO.c200)
                .bg(material::INDIGO.c900)
            })
            .collect_vec();
        let visible_items_len = visible_items.len();

        if let Some(x) = visible_items.get_mut(self.selected_index.saturating_sub(self.scroll_pos))
        {
            *x = x
                    .clone() // why clone!!! :(
                    .fg(material::BLUE.c100)
                    .bg(material::LIGHT_BLUE.c900)
        }

        let area = Rect {
            x:      position.x,
            y:      position.y,
            width:  self.width,
            height: visible_items_len as u16 + 2,
        };
        let mut top_colors = vec![];
        for x in 0..area.width {
            top_colors.push(frame.buffer_mut().cell((area.x + x, area.y)).unwrap().bg);
        }
        let mut bottom_colors = vec![];
        for x in 0..area.width {
            bottom_colors.push(
                frame
                    .buffer_mut()
                    .cell((area.x + x, area.y + area.height - 1))
                    .unwrap()
                    .bg,
            );
        }
        frame.render_widget(Clear, area);
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            area,
            |_, _| {},
        );

        let sort_popup_block = Block::bordered()
            .border_set(border::PROPORTIONAL_WIDE)
            .fg(tailwind::INDIGO.c900);
        let inner_area = sort_popup_block.inner(area);
        frame.render_widget(&sort_popup_block, area);
        frame.render_widget(Block::new().bg(material::INDIGO.c900), inner_area);
        frame.render_widget(Text::from_iter(visible_items).left_aligned(), inner_area);

        if model_len > self.num_visible_items {
            scroll_bar(
                model_len,
                self.scroll_pos,
                self.num_visible_items,
                frame,
                inner_area
                    .offset(Offset::new(inner_area.width as i32, 0))
                    .resize(Size::new(1, inner_area.height)),
            );
        }

        for x in 0..area.width {
            frame
                .buffer_mut()
                .cell_mut((area.x + x, area.y))
                .unwrap()
                .bg = top_colors[x as usize];
        }
        for x in 0..area.width {
            frame
                .buffer_mut()
                .cell_mut((area.x + x, area.y + area.height - 1))
                .unwrap()
                .bg = bottom_colors[x as usize];
        }

        let mut result = FxHashMap::from_iter([(
            vec![],
            (
                inner_area.resize(Size {
                    width:  inner_area.width,
                    height: 1,
                }),
                visible_items_len,
            ),
        )]);

        if let Some(submenu_id) = self.opened_submenu.as_ref() {
            let submenu_width = self.submenus[submenu_id].width;
            let new_pos = position.offset(Offset::new(
                if self.submenu_right {
                    self.width as i32
                } else {
                    -(submenu_width as i32)
                },
                self.index_from_id(*submenu_id).unwrap() as i32 + 1 - self.scroll_pos as i32,
            ));

            result.extend(
                self.submenus
                    .get_mut(submenu_id)
                    .unwrap()
                    .render(new_pos, frame, key_event_handler)
                    .into_iter()
                    .map(|(mut k, v)| {
                        k.insert(0, *submenu_id);
                        (k, v)
                    }),
            );
        }

        result
    }

    pub fn render_dropdown(
        &mut self,
        dropdown_widget_area: Rect,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
    ) -> FxHashMap<Vec<usize>, (Rect, usize)> {
        self.width = dropdown_widget_area.width;
        let result = self.render(
            dropdown_widget_area.offset(Offset::new(0, 2)).as_position(),
            frame,
            key_event_handler,
        );

        frame.render_widget(
            Block::new().bg(material::BLUE.c600),
            dropdown_widget_area
                .offset(Offset::new(0, 2))
                .resize(Size::new(dropdown_widget_area.width, 1)),
        );

        result
    }
}

pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

#[derive(Default)]
pub struct ScrollList {
    pub item_height:       u16,
    pub selected_index:    usize,
    pub scroll_pos:        usize,
    pub alignment_bottom:  bool,
    pub num_visible_items: usize,
    pub partially_visible: bool,
}

impl ScrollList {
    pub fn new(item_height: u16) -> Self {
        Self {
            item_height,

            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.selected_index = 0;
        self.scroll_pos = 0;
    }

    fn ensure_view_in_bounds(&mut self, num_items: usize) {
        if self.selected_index >= num_items {
            self.selected_index = num_items.saturating_sub(1);
            self.scroll_pos = self
                .selected_index
                .saturating_sub(self.num_visible_items + 1);
        }
        if self.scroll_pos > num_items.saturating_sub(self.num_visible_items) {
            self.scroll_pos = num_items.saturating_sub(self.num_visible_items);
        }

        if self.selected_index < self.scroll_pos {
            self.scroll_pos = self.selected_index;
        } else if self.selected_index.saturating_sub(self.scroll_pos) >= self.num_visible_items - 1
        {
            self.scroll_pos = self
                .selected_index
                .saturating_sub(self.num_visible_items - 1);
        }

        if num_items < self.num_visible_items
            || self.selected_index.saturating_sub(self.scroll_pos) == 0
        {
            self.alignment_bottom = false;
        } else if self.selected_index.saturating_sub(self.scroll_pos) == self.num_visible_items - 1
        {
            self.alignment_bottom = true;
        }
    }

    pub fn goto_index(&mut self, index: usize, centered: bool, num_items: usize) {
        self.selected_index = index;
        if centered {
            if self.scroll_pos > index || index >= self.scroll_pos + self.num_visible_items {
                self.scroll_pos = index
                    .saturating_sub(self.num_visible_items / 2)
                    .min(num_items.saturating_sub(self.num_visible_items));
                self.alignment_bottom = false;
            }
        } else {
            self.scroll_pos = self.scroll_pos.min(self.selected_index);
            if self.selected_index - self.scroll_pos >= self.num_visible_items {
                self.scroll_pos = self.selected_index - self.num_visible_items + 1;
            }
        }
        self.ensure_view_in_bounds(num_items);
    }

    pub fn scroll(&mut self, direction: bool, num_items: usize) {
        if direction {
            self.selected_index = (self.selected_index + 1).min(num_items.saturating_sub(1));
            if self.selected_index.saturating_sub(self.scroll_pos) >= self.num_visible_items {
                self.scroll_pos = self
                    .selected_index
                    .saturating_sub(self.num_visible_items - 1);
            }
        } else {
            self.selected_index = self.selected_index.saturating_sub(1);
            if self.selected_index < self.scroll_pos {
                self.scroll_pos = self.selected_index;
            }
        }
        self.ensure_view_in_bounds(num_items);
    }

    pub fn render(
        &mut self,
        num_items: usize,
        area: Rect,
        scrollbar_area: Rect,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        mut render_callback: impl FnMut(
            &Self,
            Rect,
            usize,
            bool,
            bool,
            &mut Frame,
            &mut KeyEventHandler,
        ),
    ) {
        let num_visible_items = area.height as usize / self.item_height as usize;
        let partially_visible_item_height =
            area.height as usize - num_visible_items * self.item_height as usize;
        self.partially_visible = partially_visible_item_height > 0;

        let num_visible_items = num_visible_items + if self.partially_visible { 1 } else { 0 };
        if self.num_visible_items > num_visible_items {
            if self.alignment_bottom {
                self.scroll_pos += self.num_visible_items - num_visible_items;
            }
        } else if self.num_visible_items < num_visible_items {
            if self.alignment_bottom {
                self.scroll_pos = self
                    .scroll_pos
                    .saturating_sub(num_visible_items - self.num_visible_items);
            }
        }
        self.num_visible_items = num_visible_items;

        self.ensure_view_in_bounds(num_items);

        let mut remaining_area = area;
        for i in 0..self.num_visible_items {
            let [area, remaining] = if self.partially_visible
                && i == (!self.alignment_bottom as usize * (self.num_visible_items - 1))
            {
                vertical![==partially_visible_item_height as u16, >= 0]
            } else {
                vertical![==self.item_height, >= 0]
            }
            .areas(remaining_area);

            let index = self.scroll_pos + i;
            if index < num_items {
                let alternate = i & 1 == 1;
                let selected = self.selected_index == i + self.scroll_pos;

                render_callback(
                    self,
                    area,
                    index,
                    selected,
                    alternate,
                    frame,
                    key_event_handler,
                );
            } else {
                frame.render_widget(
                    Block::new().bg(if i & 1 == 0 {
                        tailwind::SLATE.c950
                    } else {
                        tailwind::BLACK
                    }),
                    area,
                );
            }

            remaining_area = remaining;
        }

        if num_items + self.partially_visible as usize > self.num_visible_items {
            scroll_bar(
                num_items + self.partially_visible as usize,
                self.scroll_pos + (self.partially_visible && self.alignment_bottom) as usize,
                self.num_visible_items,
                frame,
                scrollbar_area,
            );
        }
    }
}

#[derive(Default)]
pub struct ScrollGallery {
    item_size:            Size,
    pub selected_index:   usize,
    items_per_row:        usize,
    scroll_pos:           usize,
    pub alignment_bottom: bool,
    num_visible_rows:     usize,
    partially_visible:    bool,
}

impl ScrollGallery {
    pub fn new(item_size: Size) -> Self {
        Self {
            item_size,

            ..Default::default()
        }
    }

    // pub fn reset(&mut self) {
    //     self.selected_index = 0;
    //     self.scroll_pos = 0;
    // }

    fn ensure_view_in_bounds(&mut self, num_items: usize) {
        let num_rows = if self.items_per_row != 0 {
            num_items.div_ceil(self.items_per_row)
        } else {
            0
        };
        let selected_row = if self.items_per_row != 0 {
            self.selected_index / self.items_per_row
        } else {
            0
        };
        if self.selected_index >= num_items {
            self.selected_index = num_items.saturating_sub(1);
            self.scroll_pos = if self.items_per_row != 0 {
                (self.selected_index / self.items_per_row).saturating_sub(self.num_visible_rows + 1)
            } else {
                0
            };
        }
        if self.scroll_pos > num_rows.saturating_sub(self.num_visible_rows) {
            self.scroll_pos = num_rows.saturating_sub(self.num_visible_rows);
        }

        if selected_row < self.scroll_pos {
            self.scroll_pos = selected_row;
        } else if selected_row.saturating_sub(self.scroll_pos)
            >= self.num_visible_rows.saturating_sub(1)
        {
            self.scroll_pos = selected_row.saturating_sub(self.num_visible_rows.saturating_sub(1));
        }

        if num_rows < self.num_visible_rows || selected_row.saturating_sub(self.scroll_pos) == 0 {
            self.alignment_bottom = false;
        } else if selected_row.saturating_sub(self.scroll_pos) == self.num_visible_rows - 1 {
            self.alignment_bottom = true;
        }
    }

    pub fn goto_index(&mut self, index: usize, centered: bool, num_items: usize) {
        let num_rows = if self.items_per_row != 0 {
            num_items.div_ceil(self.items_per_row)
        } else {
            0
        };
        let row = if self.items_per_row != 0 {
            index / self.items_per_row
        } else {
            0
        };
        self.selected_index = index;
        if centered {
            if self.scroll_pos > row || row >= self.scroll_pos + self.num_visible_rows {
                self.scroll_pos = row
                    .saturating_sub(self.num_visible_rows / 2)
                    .min(num_rows.saturating_sub(self.num_visible_rows));
                self.alignment_bottom = false;
            }
        } else {
            self.scroll_pos = self.scroll_pos.min(row);
            if row - self.scroll_pos >= self.num_visible_rows {
                self.scroll_pos = row - self.num_visible_rows + 1;
            }
        }
        self.ensure_view_in_bounds(num_items);
    }

    pub fn scroll(&mut self, direction: Direction, num_items: usize) {
        match direction {
            Direction::Up => {
                self.selected_index = self.selected_index.saturating_sub(self.items_per_row);
                let selected_row = if self.items_per_row != 0 {
                    self.selected_index / self.items_per_row
                } else {
                    0
                };

                if selected_row < self.scroll_pos {
                    self.scroll_pos = selected_row;
                }
            }
            Direction::Down => {
                self.selected_index =
                    (self.selected_index + self.items_per_row).min(num_items.saturating_sub(1));
                let selected_row = if self.items_per_row != 0 {
                    self.selected_index / self.items_per_row
                } else {
                    0
                };

                if selected_row.saturating_sub(self.scroll_pos) >= self.num_visible_rows {
                    self.scroll_pos = selected_row.saturating_sub(self.num_visible_rows - 1);
                }
            }
            Direction::Right => {
                self.selected_index = (self.selected_index + 1).min(num_items.saturating_sub(1));
                let selected_row = if self.items_per_row != 0 {
                    self.selected_index / self.items_per_row
                } else {
                    0
                };

                if selected_row.saturating_sub(self.scroll_pos) >= self.num_visible_rows {
                    self.scroll_pos = selected_row.saturating_sub(self.num_visible_rows - 1);
                }
            }
            Direction::Left => {
                self.selected_index = self.selected_index.saturating_sub(1);
                let selected_row = if self.items_per_row != 0 {
                    self.selected_index / self.items_per_row
                } else {
                    0
                };

                if selected_row < self.scroll_pos {
                    self.scroll_pos = selected_row;
                }
            }
        }
        self.ensure_view_in_bounds(num_items);
    }

    pub fn render(
        &mut self,
        num_items: usize,
        area: Rect,
        scrollbar_area: Rect,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        mut render_callback: impl FnMut(
            &Self,
            Rect,
            usize,
            u16,
            bool,
            bool,
            &mut Frame,
            &mut KeyEventHandler,
        ),
    ) {
        let num_visible_rows = (area.height / self.item_size.height) as usize;
        let partially_visible_row_height =
            area.height as usize - num_visible_rows * self.item_size.height as usize;
        self.partially_visible = partially_visible_row_height > 0;
        self.items_per_row = (area.width / self.item_size.width) as usize;

        let num_visible_rows = num_visible_rows + if self.partially_visible { 1 } else { 0 };
        if self.num_visible_rows > num_visible_rows {
            if self.alignment_bottom {
                self.scroll_pos += self.num_visible_rows - num_visible_rows;
            }
        } else if self.num_visible_rows < num_visible_rows {
            if self.alignment_bottom {
                self.scroll_pos = self
                    .scroll_pos
                    .saturating_sub(num_visible_rows - self.num_visible_rows);
            }
        }
        self.num_visible_rows = num_visible_rows;

        self.ensure_view_in_bounds(num_items);

        let num_rows = if self.items_per_row != 0 {
            num_items.div_ceil(self.items_per_row)
        } else {
            0
        };

        let mut remaining_vert_area = area;
        for i in 0..self.num_visible_rows {
            let [vert_area, remaining] = if self.partially_visible
                && i == (!self.alignment_bottom as usize * (self.num_visible_rows - 1))
            {
                vertical![==partially_visible_row_height as u16, >= 0]
            } else {
                vertical![==self.item_size.height, >= 0]
            }
            .areas(remaining_vert_area);

            let row = self.scroll_pos + i;
            let mut remaining_horiz_area = vert_area;
            for j in 0..self.items_per_row {
                let [area, remaining] =
                    horizontal![==self.item_size.width, >=0].areas(remaining_horiz_area);

                let index = row * self.items_per_row + j;
                let alternate = (j + i) & 1 == 1;
                if index < num_items {
                    let selected = self.selected_index == index;

                    render_callback(
                        self,
                        area,
                        index,
                        self.item_size.height - area.height,
                        selected,
                        alternate,
                        frame,
                        key_event_handler,
                    );
                } else {
                    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), area);
                }

                remaining_horiz_area = remaining;
            }

            remaining_vert_area = remaining;
        }

        if num_rows + self.partially_visible as usize > self.num_visible_rows {
            scroll_bar(
                num_rows + self.partially_visible as usize,
                self.scroll_pos + (self.partially_visible && self.alignment_bottom) as usize,
                self.num_visible_rows,
                frame,
                scrollbar_area,
            );
        }
    }
}

pub fn window(frame: &mut Frame, area: Rect, title: &str, and_a_half: bool) -> Rect {
    let popup = Block::bordered()
        .border_set(border::PROPORTIONAL_WIDE)
        .border_style(Style::new().fg(tailwind::VIOLET.c950))
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(Style::new().fg(material::YELLOW.c800));

    let mut top_colors = vec![];
    for x in 0..area.width {
        top_colors.push(frame.buffer_mut().cell((area.x + x, area.y)).unwrap().bg);
    }
    let mut bottom_colors = vec![];
    for x in 0..area.width {
        bottom_colors.push(
            frame
                .buffer_mut()
                .cell((area.x + x, area.y + area.height - 1))
                .unwrap()
                .bg,
        );
    }

    let popup_area = popup.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    for x in 0..area.width {
        frame
            .buffer_mut()
            .cell_mut((area.x + x, area.y))
            .unwrap()
            .bg = top_colors[x as usize];
    }
    if and_a_half {
        frame.render_widget(
            Block::new()
                .borders(!Borders::TOP)
                .border_set(border::PROPORTIONAL_TALL)
                .border_style(Style::new().fg(tailwind::VIOLET.c950))
                .bg(tailwind::BLUE.c950),
            add_padding(area, Padding::top(1)),
        );
    } else {
        for x in 0..area.width {
            frame
                .buffer_mut()
                .cell_mut((area.x + x, area.y + area.height - 1))
                .unwrap()
                .bg = bottom_colors[x as usize];
        }
    }
    frame.render_widget(Block::new().bg(tailwind::BLUE.c950), popup_area);

    popup_area
}

pub fn scroll_bar(
    items_count: usize,
    scroll_pos: usize,
    num_visible_items: usize,
    frame: &mut Frame,
    area: Rect,
) {
    const BLOCKS: [char; 9] = ['█', '▇', '▆', '▅', '▄', '▃', '▂', '▁', ' '];

    frame.render_widget(
        text!["█".repeat(area.height as usize),].fg(tailwind::INDIGO.c950),
        area,
    );
    frame.render_widget("▲".bg(tailwind::INDIGO.c700).fg(material::BLUE.c300), area);

    let num_pixels = (area.height as usize - 2) * 8;
    let max_scroll_amount = items_count.saturating_sub(num_visible_items);

    let mut handle_size = num_pixels.saturating_sub(max_scroll_amount);
    let mut scroll_pixels = handle_size.div_ceil(max_scroll_amount)
        // - if handle_size % max_scroll_amount == 0 { 1 } else { 0 })
    .min(3);
    handle_size -= scroll_pixels.saturating_sub(1) * max_scroll_amount;
    while handle_size < 8 && scroll_pixels > 1 {
        handle_size += max_scroll_amount;
        scroll_pixels -= 1;
    }

    if handle_size >= 8 {
        let mut top_margin = scroll_pos * scroll_pixels;
        let mut lines = Text::default();

        while top_margin >= 8 {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
            top_margin -= 8;
        }
        if top_margin > 0 {
            lines.push_line(
                BLOCKS[top_margin]
                    .bg(tailwind::INDIGO.c950)
                    .fg(material::BLUE.c300),
            );
            handle_size -= 8 - top_margin;
        }
        while handle_size >= 8 {
            lines.push_line(" ".bg(material::BLUE.c300));
            handle_size -= 8;
        }
        if handle_size > 0 {
            lines.push_line(
                BLOCKS[handle_size]
                    .fg(tailwind::INDIGO.c950)
                    .bg(material::BLUE.c300),
            );
        }
        while lines.lines.len() < area.height as usize - 2 {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }

        frame.render_widget(lines, area.offset(Offset::new(0, 1)));
    } else {
        let cycle_every = area.height as usize - 3;
        let scroll_fraction =
            scroll_pos as f32 / items_count.saturating_sub(num_visible_items) as f32;
        let phase = (scroll_fraction * cycle_every as f32) as usize;
        let block = BLOCKS[(scroll_fraction * 9.0 * cycle_every as f32) as usize % 9]
            .bg(tailwind::INDIGO.c950)
            .fg(material::BLUE.c300);

        let mut lines = Text::default();
        for _ in 0..phase {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }
        lines.push_line(block.clone());
        lines.push_line(block.reversed());
        for _ in 0..(area.height as usize - lines.lines.len()) {
            lines.push_line(" ".bg(tailwind::INDIGO.c950));
        }

        frame.render_widget(&lines, area);
    }

    frame.render_widget(
        "▼"
            .bg(tailwind::INDIGO.c700)
            .fg(material::BLUE.c300)
            .not_reversed(),
        area.offset(Offset::new(0, area.height as i32 - 1)),
    );
}

pub enum ActionType {
    Default,
    Normal,
    Critical,
}

pub struct Action {
    action:      &'static str,
    action_type: ActionType,
    selected:    bool,
    valid:       bool,
}
impl Action {
    pub fn new(action: &'static str, action_type: ActionType, selected: bool, valid: bool) -> Self {
        Self {
            action,
            action_type,
            selected,
            valid,
        }
    }
}
impl<'a> From<Action> for Span<'a> {
    fn from(value: Action) -> Span<'a> {
        span!(value.action)
            .fg(if value.valid {
                if value.selected {
                    tailwind::SLATE.c300
                } else {
                    match value.action_type {
                        ActionType::Default => tailwind::SLATE.c300,
                        ActionType::Normal => material::BLUE.c500,
                        ActionType::Critical => tailwind::RED.c500,
                    }
                }
            } else {
                tailwind::SLATE.c500
            })
            .bg(if value.valid {
                if value.selected {
                    match value.action_type {
                        ActionType::Default => material::BLUE.c600,
                        ActionType::Normal => material::BLUE.c800,
                        ActionType::Critical => tailwind::RED.c800,
                    }
                } else {
                    if matches!(value.action_type, ActionType::Default) {
                        material::BLUE.c900
                    } else {
                        tailwind::SLATE.c950
                    }
                }
            } else {
                if value.selected {
                    tailwind::SLATE.c700
                } else {
                    tailwind::SLATE.c800
                }
            })
    }
}

pub fn action(
    action: Action,
    alignment: HorizontalAlignment,
    bottom: bool,
    area: Rect,
    frame: &mut Frame,
) -> Rect {
    let span: Span<'_> = action.into();

    let area = if bottom {
        vertical![>=1, ==1].split(area)[1]
    } else {
        area
    };

    let mouse_area = match alignment {
        HorizontalAlignment::Left => area,
        HorizontalAlignment::Center => area.offset(Offset::new(
            (area.width as i32 - span.width() as i32) / 2,
            0,
        )),
        HorizontalAlignment::Right =>
            area.offset(Offset::new(area.width as i32 - span.width() as i32, 0)),
    }
    .resize(Size::new(span.width() as u16, 1));
    let line = match alignment {
        HorizontalAlignment::Left => line!(span),
        HorizontalAlignment::Center => line!(span).centered(),
        HorizontalAlignment::Right => line!(span).right_aligned(),
    };

    frame.render_widget(line, area);

    mouse_area
}

pub fn actions<const N: usize>(
    actions: [Action; N],
    alignment: HorizontalAlignment,
    bottom: bool,
    spacing: u16,
    area: Rect,
    frame: &mut Frame,
) -> [Rect; N] {
    let spans: Vec<Span<'_>> = actions.into_iter().map(|x| x.into()).collect_vec();
    let actions_count = spans.len();
    let actions_width =
        spans.iter().fold(0, |a, x| a + x.width()) + spacing as usize * (actions_count - 1);

    let area = if bottom {
        vertical![>=1, ==1].split(area)[1]
    } else {
        area
    };

    let mut mouse_areas = [Rect::default(); N];
    let mut mouse_area = match alignment {
        HorizontalAlignment::Left => area,
        HorizontalAlignment::Center => area.offset(Offset::new(
            (area.width as i32 - actions_width as i32) / 2,
            0,
        )),
        HorizontalAlignment::Right =>
            area.offset(Offset::new(area.width as i32 - actions_width as i32, 0)),
    };
    for (i, span) in spans.iter().enumerate() {
        mouse_area = mouse_area.resize(Size::new(span.width() as u16, 1));
        mouse_areas[i] = mouse_area;

        mouse_area = mouse_area.offset(Offset::new(span.width() as i32 + spacing as i32, 0));
    }

    let mut line = Line::from_iter(
        spans
            .into_iter()
            .flat_map(|x| [x, span!(" ".repeat(spacing as usize))])
            .take(actions_count * 2 - 1),
    );
    line = match alignment {
        HorizontalAlignment::Left => line,
        HorizontalAlignment::Center => line.centered(),
        HorizontalAlignment::Right => line.right_aligned(),
    };

    frame.render_widget(line, area);

    mouse_areas
}

pub struct Hyperlink<'content> {
    pub text: Text<'content>,
    pub url:  String,
}

impl Widget for Hyperlink<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        (&self.text).render(area, buffer);

        if !self.text.lines.is_empty() {
            for (j, line) in self.text.lines.iter().enumerate() {
                if line.width() > 0 {
                    for (i, char) in line.to_string().chars().enumerate() {
                        let hyperlink = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, char);

                        buffer[(area.x + i as u16, area.y + j as u16)]
                            .set_symbol(hyperlink.as_str());
                        buffer[(area.x + i as u16, area.y + j as u16)].set_diff_option(
                            ratatui::buffer::CellDiffOption::ForcedWidth(
                                core::num::NonZero::new(1).unwrap(),
                            ),
                        );
                    }
                }
            }
        }
    }
}
