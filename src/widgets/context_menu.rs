use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Offset, Position, Rect, Size},
    macros::line,
    style::{
        Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    text::Text,
    widgets::{Block, Clear, Fill},
};
use rustc_hash::FxHashMap;

use crate::{key_event_handler::KeyEventHandler, types::FxIndexMap};

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
            top_colors.push(
                frame
                    .buffer_mut()
                    .cell((area.x + x, area.y))
                    .map(|x| x.bg)
                    .unwrap_or_default(),
            );
        }
        let mut bottom_colors = vec![];
        for x in 0..area.width {
            bottom_colors.push(
                frame
                    .buffer_mut()
                    .cell((area.x + x, area.y + area.height - 1))
                    .map(|x| x.bg)
                    .unwrap_or_default(),
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
        frame.render_widget(Fill::new(" ").bg(material::INDIGO.c900), inner_area);
        frame.render_widget(Text::from_iter(visible_items).left_aligned(), inner_area);

        if model_len > self.num_visible_items {
            super::scroll_bar(
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
            if let Some(cell) = frame.buffer_mut().cell_mut((area.x + x, area.y)) {
                cell.bg = top_colors[x as usize];
            }
        }
        for x in 0..area.width {
            if let Some(cell) = frame
                .buffer_mut()
                .cell_mut((area.x + x, area.y + area.height - 1))
            {
                cell.bg = bottom_colors[x as usize];
            }
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
