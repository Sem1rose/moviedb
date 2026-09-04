use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Rect, Size},
    macros::{horizontal, vertical},
    style::{Stylize, palette::tailwind},
    widgets::Fill,
};

use crate::key_event_handler::KeyEventHandler;

pub enum Direction {
    Up,
    Down,
    Right,
    Left,
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

    pub fn update_for_area(&mut self, area: Rect, num_items: usize) {
        let num_visible_rows = (area.height / self.item_size.height) as usize;
        self.partially_visible =
            area.height as usize > num_visible_rows * self.item_size.height as usize;
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
    }

    pub fn render_without_area_update(
        &self,
        num_items: usize,
        area: Rect,
        scrollbar_area: Rect,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        mut render_callback: impl FnMut(&mut Buffer, u16, i32, bool, usize, bool, &mut KeyEventHandler),
    ) {
        let num_rows = if self.items_per_row != 0 {
            num_items.div_ceil(self.items_per_row)
        } else {
            0
        };
        let num_visible_rows = (area.height / self.item_size.height) as usize;
        let partially_visible_row_height =
            area.height as usize - num_visible_rows * self.item_size.height as usize;

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
                let row_is_partially_visible = self.partially_visible
                    && i == (!self.alignment_bottom as usize * (self.num_visible_rows - 1));
                let [area, remaining] =
                    horizontal![==self.item_size.width, >=0].areas(remaining_horiz_area);

                let index = row * self.items_per_row + j;
                if index < num_items {
                    let selected = self.selected_index == index;
                    let num_hidden_lines = self.item_size.height - area.height as u16;
                    let buffer_y_negative_offset =
                        if row_is_partially_visible && area.y < num_hidden_lines {
                            -((num_hidden_lines - area.y) as i32)
                        } else {
                            0
                        };

                    let mut buffer = Buffer::empty(Rect::new(
                        area.x,
                        area.y.saturating_sub(
                            if row_is_partially_visible && self.alignment_bottom {
                                num_hidden_lines
                            } else {
                                0
                            },
                        ),
                        area.width,
                        self.item_size.height,
                    ));
                    render_callback(
                        &mut buffer,
                        num_hidden_lines,
                        buffer_y_negative_offset,
                        self.alignment_bottom,
                        index,
                        selected,
                        key_event_handler,
                    );

                    if row_is_partially_visible {
                        if self.alignment_bottom {
                            buffer.content =
                                buffer.content[(num_hidden_lines * area.width) as usize..].to_vec();
                            buffer.area = area;
                        } else {
                            buffer.resize(area);
                        }
                    }

                    frame.buffer_mut().merge(&buffer);
                } else {
                    frame.render_widget(Fill::new(" ").bg(tailwind::SLATE.c900), area);
                }

                remaining_horiz_area = remaining;
            }

            remaining_vert_area = remaining;
        }

        if num_rows + self.partially_visible as usize > self.num_visible_rows {
            super::scroll_bar(
                num_rows + self.partially_visible as usize,
                self.scroll_pos + (self.partially_visible && self.alignment_bottom) as usize,
                self.num_visible_rows,
                frame,
                scrollbar_area,
            );
        }
    }

    pub fn render(
        &mut self,
        num_items: usize,
        area: Rect,
        scrollbar_area: Rect,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        render_callback: impl FnMut(&mut Buffer, u16, i32, bool, usize, bool, &mut KeyEventHandler),
    ) {
        self.update_for_area(area, num_items);
        self.render_without_area_update(
            num_items,
            area,
            scrollbar_area,
            frame,
            key_event_handler,
            render_callback,
        );
    }
}
