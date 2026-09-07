use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    macros::{horizontal, vertical},
    style::{Stylize, palette::tailwind},
    widgets::{Fill, Widget},
};

use crate::key_event_handler::KeyEventHandler;

pub enum ListDirection {
    Vertical(bool),
    Horizontal(bool),
}
impl Default for ListDirection {
    fn default() -> Self {
        Self::Vertical(false)
    }
}

#[derive(Default)]
pub struct ScrolledList {
    pub item_dimension:     u16,
    pub selected_index:     usize,
    pub scroll_pos:         usize,
    pub alignment_opposite: bool,
    pub num_visible_items:  usize,
    pub partially_visible:  bool,
    direction:              ListDirection,
}

impl ScrolledList {
    pub fn new(direction: ListDirection, item_dimension: u16) -> Self {
        Self {
            direction,
            item_dimension,

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

        match self.direction {
            ListDirection::Vertical(_) =>
                if self.scroll_pos > num_items.saturating_sub(self.num_visible_items) {
                    self.scroll_pos = num_items.saturating_sub(self.num_visible_items);
                },
            ListDirection::Horizontal(_) =>
                if self.scroll_pos > num_items.saturating_sub(self.num_visible_items) + 1 {
                    self.scroll_pos = num_items.saturating_sub(self.num_visible_items) + 1;
                },
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
            self.alignment_opposite = false;
        } else if self.selected_index.saturating_sub(self.scroll_pos) == self.num_visible_items - 1
        {
            if matches!(self.direction, ListDirection::Horizontal(_)) {
                if self.partially_visible {
                    self.scroll_pos += 1;
                }
            } else {
                self.alignment_opposite = true;
            }
        }
    }

    pub fn goto_index(&mut self, index: usize, centered: bool, num_items: usize) {
        self.selected_index = index;
        if centered {
            if self.scroll_pos > index || index >= self.scroll_pos + self.num_visible_items {
                self.scroll_pos = index
                    .saturating_sub(self.num_visible_items / 2)
                    .min(num_items.saturating_sub(self.num_visible_items));
                self.alignment_opposite = false;
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
        let num_visible_items = match self.direction {
            ListDirection::Vertical(_) => {
                let num_visible_items = area.height as usize / self.item_dimension as usize;
                self.partially_visible =
                    area.height as usize > num_visible_items * self.item_dimension as usize;

                num_visible_items
            }
            ListDirection::Horizontal(_) => {
                let num_visible_items = area.width as usize / self.item_dimension as usize;
                self.partially_visible =
                    area.width as usize > num_visible_items * self.item_dimension as usize;

                num_visible_items
            }
        } + if self.partially_visible { 1 } else { 0 };

        if self.num_visible_items > num_visible_items {
            if self.alignment_opposite {
                self.scroll_pos += self.num_visible_items - num_visible_items;
            }
        } else if self.num_visible_items < num_visible_items {
            if self.alignment_opposite {
                if self.scroll_pos == 0 {
                    self.alignment_opposite = false
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
        scrollbar_buffer: Option<&mut Buffer>,
        render_placeholder: bool,
        buffer: &mut Buffer,
        key_event_handler: &mut KeyEventHandler,
        mut render_callback: impl FnMut(&mut Buffer, u16, i32, bool, usize, bool, &mut KeyEventHandler),
    ) {
        let area = buffer.area;
        let partially_visible_item_dimension = match self.direction {
            ListDirection::Vertical(_) => area.height,
            ListDirection::Horizontal(_) => area.width,
        } as usize
            - (self.num_visible_items - if self.partially_visible { 1 } else { 0 })
                * self.item_dimension as usize;

        let mut remaining_area = area;
        for i in 0..self.num_visible_items {
            let item_is_partially_visible = self.partially_visible
                && i == (!self.alignment_opposite as usize * (self.num_visible_items - 1));

            let [area, remaining] = match self.direction {
                ListDirection::Vertical(reversed) =>
                    if reversed {
                        let [remaining, area] = vertical![>=0, ==if item_is_partially_visible {partially_visible_item_dimension as u16} else {self.item_dimension}]
                    .areas(remaining_area);

                        [area, remaining]
                    } else {
                        vertical![==if item_is_partially_visible {partially_visible_item_dimension as u16} else {self.item_dimension}, >= 0]
                        .areas(remaining_area)
                    },
                ListDirection::Horizontal(reversed) =>
                    if reversed {
                        let [remaining, area] = horizontal![>=0, ==if item_is_partially_visible {partially_visible_item_dimension as u16} else {self.item_dimension}]
                        .areas(remaining_area);

                        [area, remaining]
                    } else {
                        horizontal![==if item_is_partially_visible {partially_visible_item_dimension as u16} else {self.item_dimension}, >= 0]
                        .areas(remaining_area)
                    },
            };

            let index = self.scroll_pos + i;
            if index < num_items {
                let selected = self.selected_index == i + self.scroll_pos;
                let num_hidden_lines = self.item_dimension
                    - match self.direction {
                        ListDirection::Vertical(_) => area.height,
                        ListDirection::Horizontal(_) => area.width,
                    } as u16;
                let buffer_negative_offset = if item_is_partially_visible {
                    match self.direction {
                        ListDirection::Vertical(false) =>
                            if area.y < num_hidden_lines {
                                -((num_hidden_lines - area.y) as i32)
                            } else {
                                0
                            },
                        ListDirection::Horizontal(false) =>
                            if area.x < num_hidden_lines {
                                -((num_hidden_lines - area.x) as i32)
                            } else {
                                0
                            },
                        _ => 0,
                    }
                } else {
                    0
                };

                let mut buf = Buffer::empty(Rect::new(
                    area.x.saturating_sub(
                        if item_is_partially_visible
                            && matches!(self.direction, ListDirection::Horizontal(false))
                            && self.alignment_opposite
                        {
                            num_hidden_lines
                        } else {
                            0
                        },
                    ),
                    area.y.saturating_sub(
                        if item_is_partially_visible
                            && matches!(self.direction, ListDirection::Vertical(false))
                            && self.alignment_opposite
                        {
                            num_hidden_lines
                        } else {
                            0
                        },
                    ),
                    if matches!(self.direction, ListDirection::Vertical(_)) {
                        area.width
                    } else {
                        self.item_dimension
                    },
                    if matches!(self.direction, ListDirection::Vertical(_)) {
                        self.item_dimension
                    } else {
                        area.height
                    },
                ));

                render_callback(
                    &mut buf,
                    num_hidden_lines,
                    buffer_negative_offset,
                    self.alignment_opposite,
                    index,
                    selected,
                    key_event_handler,
                );

                if item_is_partially_visible {
                    match self.direction {
                        ListDirection::Vertical(reversed) =>
                            if reversed {
                                if self.alignment_opposite {
                                    buf.resize(area);
                                } else {
                                    buf.content = buf.content
                                        [(num_hidden_lines * area.width) as usize..]
                                        .to_vec();
                                    buf.area = area;
                                }
                            } else {
                                if self.alignment_opposite {
                                    buf.content = buf.content
                                        [(num_hidden_lines * area.width) as usize..]
                                        .to_vec();
                                    buf.area = area;
                                } else {
                                    buf.resize(area);
                                }
                            },
                        ListDirection::Horizontal(reversed) => {
                            if reversed {
                                if self.alignment_opposite {
                                    buf.content = buf
                                        .content
                                        .into_iter()
                                        .chunks(self.item_dimension as usize)
                                        .into_iter()
                                        .map(|x| {
                                            x.take(
                                                (self.item_dimension - num_hidden_lines) as usize,
                                            )
                                        })
                                        .flatten()
                                        .collect_vec();
                                } else {
                                    buf.content = buf
                                        .content
                                        .into_iter()
                                        .chunks(self.item_dimension as usize)
                                        .into_iter()
                                        .map(|x| x.dropping(num_hidden_lines as usize))
                                        .flatten()
                                        .collect_vec();
                                }
                            } else {
                                if self.alignment_opposite {
                                    buf.content = buf
                                        .content
                                        .into_iter()
                                        .chunks(self.item_dimension as usize)
                                        .into_iter()
                                        .map(|x| x.dropping(num_hidden_lines as usize))
                                        .flatten()
                                        .collect_vec();
                                } else {
                                    buf.content = buf
                                        .content
                                        .into_iter()
                                        .chunks(self.item_dimension as usize)
                                        .into_iter()
                                        .map(|x| {
                                            x.take(
                                                (self.item_dimension - num_hidden_lines) as usize,
                                            )
                                        })
                                        .flatten()
                                        .collect_vec();
                                }
                            }

                            buf.area = area;
                        }
                    }
                }

                buffer.merge(&buf);
            } else if render_placeholder {
                Fill::new(" ")
                    .bg(if i & 1 == 0 {
                        tailwind::SLATE.c950
                    } else {
                        tailwind::BLACK
                    })
                    .render(area, buffer);
            }

            remaining_area = remaining;
        }

        if num_items + self.partially_visible as usize > self.num_visible_items {
            if let Some(scrollbar_buffer) = scrollbar_buffer {
                super::scroll_bar(
                    num_items + self.partially_visible as usize,
                    self.scroll_pos + (self.partially_visible && self.alignment_opposite) as usize,
                    self.num_visible_items,
                    scrollbar_buffer,
                );
            }
        }
    }

    pub fn render(
        &mut self,
        num_items: usize,
        scrollbar_buffer: Option<&mut Buffer>,
        render_placeholder: bool,
        buffer: &mut Buffer,
        key_event_handler: &mut KeyEventHandler,
        render_callback: impl FnMut(&mut Buffer, u16, i32, bool, usize, bool, &mut KeyEventHandler),
    ) {
        self.update_for_area(buffer.area, num_items);
        self.render_without_area_update(
            num_items,
            scrollbar_buffer,
            render_placeholder,
            buffer,
            key_event_handler,
            render_callback,
        );
    }
}
