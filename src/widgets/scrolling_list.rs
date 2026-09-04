use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    macros::vertical,
    style::{Stylize, palette::tailwind},
    widgets::Fill,
};

use crate::key_event_handler::KeyEventHandler;

#[derive(Default)]
pub struct ScrolledList {
    pub item_height:       u16,
    pub selected_index:    usize,
    pub scroll_pos:        usize,
    pub alignment_bottom:  bool,
    pub num_visible_items: usize,
    pub partially_visible: bool,
}

impl ScrolledList {
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

    pub fn update_for_area(&mut self, area: Rect, num_items: usize) {
        let num_visible_items = area.height as usize / self.item_height as usize;
        self.partially_visible =
            area.height as usize > num_visible_items * self.item_height as usize;

        let num_visible_items = num_visible_items + if self.partially_visible { 1 } else { 0 };
        if self.num_visible_items > num_visible_items {
            if self.alignment_bottom {
                self.scroll_pos += self.num_visible_items - num_visible_items;
            }
        } else if self.num_visible_items < num_visible_items {
            if self.alignment_bottom {
                if self.scroll_pos == 0 {
                    self.alignment_bottom = false
                } else {
                    self.scroll_pos = self
                        .scroll_pos
                        .saturating_sub(num_visible_items - self.num_visible_items);
                }
            }
        }
        self.num_visible_items = num_visible_items;

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
        let partially_visible_item_height = area.height as usize
            - (self.num_visible_items - if self.partially_visible { 1 } else { 0 })
                * self.item_height as usize;

        let mut remaining_area = area;
        for i in 0..self.num_visible_items {
            let item_is_partially_visible = self.partially_visible
                && i == (!self.alignment_bottom as usize * (self.num_visible_items - 1));

            let [area, remaining] = if item_is_partially_visible {
                vertical![==partially_visible_item_height as u16, >= 0]
            } else {
                vertical![==self.item_height, >= 0]
            }
            .areas(remaining_area);

            let index = self.scroll_pos + i;
            if index < num_items {
                let selected = self.selected_index == i + self.scroll_pos;
                let num_hidden_lines = self.item_height - area.height as u16;
                let buffer_y_negative_offset =
                    if item_is_partially_visible && area.y < num_hidden_lines {
                        -((num_hidden_lines - area.y) as i32)
                    } else {
                        0
                    };

                let mut buffer = Buffer::empty(Rect::new(
                    area.x,
                    area.y
                        .saturating_sub(if item_is_partially_visible && self.alignment_bottom {
                            num_hidden_lines
                        } else {
                            0
                        }),
                    area.width,
                    self.item_height,
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

                if item_is_partially_visible {
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
                frame.render_widget(
                    Fill::new(" ").bg(if i & 1 == 0 {
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
            super::scroll_bar(
                num_items + self.partially_visible as usize,
                self.scroll_pos + (self.partially_visible && self.alignment_bottom) as usize,
                self.num_visible_items,
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
