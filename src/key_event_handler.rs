use std::collections::HashMap;

use itertools::Itertools;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Position, Rect},
};

use crate::{app::App, drawer::Drawer};

pub enum Data {
    None,
    Direction(bool, KeyModifiers),
    Key(KeyEvent),
    Mouse(MouseEvent),
}

type State = (Option<usize>, Option<usize>);
type Callback = Box<dyn FnOnce(&mut App, Data)>;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Bind {
    Horizontal,
    Vertical,
    Enter,
    Esc,
    Tab,
    Input,
    Key(String),
    MouseButtonDown(MouseButton),
    MouseButtonUp(MouseButton),
}
impl Bind {
    pub fn sort_key(&self) -> String {
        match self {
            Bind::Esc => (0 as char).to_string(),
            Bind::Tab => (1 as char).to_string(),
            Bind::Enter => (2 as char).to_string(),
            Bind::Horizontal => (3 as char).to_string(),
            Bind::Vertical => (4 as char).to_string(),
            Bind::Key(key) => (5 as char).to_string() + key,
            Bind::MouseButtonDown(_) => "~".into(),
            Bind::MouseButtonUp(_) => "~".into(),
            Bind::Input => "~".into(),
        }
    }
}

#[derive(Default)]
pub struct KeyEventHandler {
    key_binds:         HashMap<(Bind, State), (String, Callback)>,
    execute_immediate: Vec<Callback>,
    mouse_binds:       HashMap<(usize, Bind, Rect), Callback>,

    semi_bind: Option<char>,
}

impl KeyEventHandler {
    pub fn clear(&mut self) {
        self.key_binds.clear();
        self.mouse_binds.clear();

        self.bind_key((None, None), 'q', "Quit".into(), |app, _| app.quit = true);
    }

    fn add_key_bind(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
        bind: Bind,
    ) {
        _ = self
            .key_binds
            .insert((bind, state), (description, Box::new(callback)));
    }

    pub fn bind_immediate(&mut self, callback: impl FnOnce(&mut App, Data) + 'static) {
        self.execute_immediate.push(Box::new(callback));
    }

    pub fn bind_horizontal(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Horizontal)
    }

    pub fn bind_vertical(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Vertical)
    }

    pub fn bind_tab(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Tab)
    }

    pub fn bind_input_field(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Input)
    }

    pub fn bind_esc(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Esc)
    }

    pub fn bind_enter(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Enter)
    }

    pub fn bind_key(
        &mut self,
        state: State,
        keys: impl ToString,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Key(keys.to_string()))
    }

    pub fn bind_mouse_button_down(
        &mut self,
        button: MouseButton,
        area: Rect,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self.mouse_binds.insert(
            (self.mouse_binds.len(), Bind::MouseButtonDown(button), area),
            Box::new(callback),
        );
    }

    // pub fn bind_mouse_button_up(
    //     &mut self,
    //     button: MouseButton,
    //     area: Rect,
    //     callback: impl FnOnce(&mut App, Data) + 'static,
    // ) {
    //     _ = self.mouse_binds.insert(
    //         (self.mouse_binds.len(), Bind::MouseButtonUp(button), area),
    //         Box::new(callback),
    //     );
    // }

    fn try_get_mouse_bind(&mut self, position: Position, bind: Bind) -> Option<Callback> {
        let mut matches = self
            .mouse_binds
            .keys()
            .cloned()
            .filter(|(_, b, rect)| b == &bind && rect.contains(position))
            .collect_vec();
        matches.sort_by(|a, b| a.0.cmp(&b.0));
        matches.reverse();
        if !matches.is_empty() {
            return Some(self.mouse_binds.remove(&matches[0]).unwrap());
        }

        None
    }

    fn try_get_key_bind(&mut self, bind: Bind, state: State) -> Option<Callback> {
        let Some((key, _)) = self
            .key_binds
            .iter()
            .filter(|((b, s), _)| {
                b == &bind
                    && s.0
                        .map(|x| state.0.is_some() && x == state.0.unwrap())
                        .unwrap_or(true)
                    && s.1
                        .map(|x| state.1.is_some() && x == state.1.unwrap())
                        .unwrap_or(true)
            })
            .sorted_by_key(|((_, s), _)| s.0.is_some() as usize * 2 + s.1.is_some() as usize)
            .last()
        else {
            return None;
        };

        if let Some((_, callback)) = self.key_binds.remove(&key.clone()) {
            return Some(callback);
        }

        None
    }

    fn try_get_keys_bind(&mut self, key: char, state: State) -> Option<Callback> {
        let key = if let Some(semi_bind) = self.semi_bind {
            String::from_iter([semi_bind, key])
        } else {
            key.to_string()
        };

        if let Some(callback) = self.try_get_key_bind(Bind::Key(key.clone()), state) {
            self.semi_bind = None;

            return Some(callback);
        } else if self.semi_bind.is_some() {
            self.semi_bind = None;
            return None;
        }

        if self
            .key_binds
            .iter()
            .filter(|((bind, s), _)| {
                (if let Bind::Key(k) = bind {
                    k.starts_with(&key.clone())
                } else {
                    false
                }) && s
                    .0
                    .map(|x| state.0.is_some() && x == state.0.unwrap())
                    .unwrap_or(true)
                    && s.1
                        .map(|x| state.1.is_some() && x == state.1.unwrap())
                        .unwrap_or(true)
            })
            .count()
            > 0
        {
            self.semi_bind = Some(key.chars().nth(0).unwrap());

            return None;
        }

        None
    }

    pub fn get_key_binds_descriptions(&self, drawer: &Drawer, max: usize) -> Vec<(Bind, String)> {
        let state = if let Some(popup) = drawer.active_popup.as_ref() {
            popup.get_state()
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => main_screen.get_state(),
            }
        } else {
            return vec![];
        };

        let mut binds = vec![];

        if let Some(semi_bind) = self.semi_bind {
            let matches = self
                .key_binds
                .iter()
                .filter(|((bind, s), _)| {
                    (if let Bind::Key(k) = bind {
                        k.starts_with(&semi_bind.to_string())
                    } else {
                        false
                    }) && s
                        .0
                        .map(|x| state.0.is_some() && x == state.0.unwrap())
                        .unwrap_or(true)
                        && s.1
                            .map(|x| state.1.is_some() && x == state.1.unwrap())
                            .unwrap_or(true)
                })
                .sorted_by_key(|((_, s), _)| s.0.is_some() as usize * 2 + s.1.is_some() as usize)
                .collect_vec();
            if !matches.is_empty() {
                binds.extend(
                    matches
                        .iter()
                        .map(|&((b, k), (d, _))| (b.clone(), *k, d.clone())),
                );
            }
        } else {
            for bind in [
                Bind::Horizontal,
                Bind::Vertical,
                Bind::Enter,
                Bind::Esc,
                Bind::Tab,
            ] {
                let matches = self
                    .key_binds
                    .iter()
                    .filter(|((b, s), _)| {
                        b == &bind
                            && s.0
                                .map(|x| state.0.is_some() && x == state.0.unwrap())
                                .unwrap_or(true)
                            && s.1
                                .map(|x| state.1.is_some() && x == state.1.unwrap())
                                .unwrap_or(true)
                    })
                    .sorted_by_key(|((_, s), _)| {
                        s.0.is_some() as usize * 2 + s.1.is_some() as usize
                    })
                    .collect_vec();
                if !matches.is_empty() {
                    binds.extend(
                        matches
                            .iter()
                            .map(|&((_, k), (d, _))| (bind.clone(), *k, d.clone())),
                    );
                }
            }

            // let input = self
            //     .key_binds
            //     .iter()
            //     .filter(|((b, s), _)| {
            //         matches!(b, Bind::Input)
            //             && s.0
            //                 .map(|x| state.0.is_some() && x == state.0.unwrap())
            //                 .unwrap_or(true)
            //             && s.1
            //                 .map(|x| state.1.is_some() && x == state.1.unwrap())
            //                 .unwrap_or(true)
            //     })
            //     .count()
            //     > 0;
            // if !input {
            let matches = self
                .key_binds
                .iter()
                .filter(|((bind, s), _)| {
                    matches!(bind, Bind::Key(_))
                        && s.0
                            .map(|x| state.0.is_some() && x == state.0.unwrap())
                            .unwrap_or(true)
                        && s.1
                            .map(|x| state.1.is_some() && x == state.1.unwrap())
                            .unwrap_or(true)
                })
                .sorted_by_key(|((_, s), _)| s.0.is_some() as usize * 2 + s.1.is_some() as usize)
                .collect_vec();

            if !matches.is_empty() {
                binds.extend(
                    matches
                        .iter()
                        .map(|&((b, k), (d, _))| (b.clone(), *k, d.clone())),
                );
            }
            // }
        }

        binds
            .into_iter()
            .filter(|(_, _, d)| !d.is_empty())
            .sorted_by_key(|(b, _, _)| b.sort_key())
            .chunk_by(|a| a.0.clone())
            .into_iter()
            .filter_map(|(_, g)| {
                g.sorted_by_key(|x| x.1.0.is_some() as usize * 2 + x.1.1.is_some() as usize)
                    .last()
            })
            .take(max)
            .map(|(b, _, d)| (b, d))
            .collect()
    }

    pub fn get_execute_immediates(&mut self) -> Vec<Callback> {
        self.execute_immediate.drain(..).collect()
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
        drawer: &Drawer,
    ) -> Option<(Callback, Data)> {
        let state = if let Some(popup) = drawer.active_popup.as_ref() {
            popup.get_state()
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => main_screen.get_state(),
            }
        } else {
            (None, None)
        };

        match event.code {
            KeyCode::Tab | KeyCode::BackTab =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Tab, state) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Tab, KeyModifiers::NONE),
                    ))
                } else {
                    None
                },
            KeyCode::Enter =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Enter, state) {
                    Some((callback, Data::None))
                } else {
                    None
                },
            KeyCode::Esc =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Esc, state) {
                    Some((callback, Data::None))
                } else {
                    None
                },
            KeyCode::Backspace | KeyCode::Delete =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Input, state) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                },
            KeyCode::Up | KeyCode::Down =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Vertical, state) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Down, event.modifiers),
                    ))
                } else {
                    None
                },
            KeyCode::Left | KeyCode::Right =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(Bind::Horizontal, state) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Right, event.modifiers),
                    ))
                } else if let Some(callback) = self.try_get_key_bind(Bind::Input, state) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                },
            KeyCode::Char(key) =>
                if let Some(callback) = self.try_get_keys_bind(key, state) {
                    Some((callback, Data::Key(event)))
                } else if let Some(callback) = self.try_get_key_bind(Bind::Input, state) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                },
            _ => None,
        }
    }

    pub fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        drawer: &Drawer,
    ) -> Option<(Callback, Data)> {
        let state = if let Some(popup) = drawer.active_popup.as_ref() {
            popup.get_state()
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => main_screen.get_state(),
            }
        } else {
            (None, None)
        };

        let position = Position {
            x: event.column,
            y: event.row,
        };
        match event.kind {
            MouseEventKind::ScrollDown => {
                if let Some(callback) = self.try_get_key_bind(Bind::Vertical, state) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(callback) = self.try_get_key_bind(Bind::Vertical, state) {
                    Some((callback, Data::Direction(false, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollRight => {
                if let Some(callback) = self.try_get_key_bind(Bind::Horizontal, state) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollLeft => {
                if let Some(callback) = self.try_get_key_bind(Bind::Horizontal, state) {
                    Some((callback, Data::Direction(false, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::Down(button) => {
                if let Some(callback) =
                    self.try_get_mouse_bind(position, Bind::MouseButtonDown(button))
                {
                    Some((callback, Data::Mouse(event)))
                } else {
                    None
                }
            }
            // MouseEventKind::Drag(MouseButton::Left) => {
            //     if let Some(callback) =
            //         self.try_get_mouse_bind(position, Bind::MouseButtonDown(MouseButton::Left))
            //     {
            //         Some((callback, Data::Mouse(event)))
            //     } else {
            //         None
            //     }
            // }
            MouseEventKind::Up(button) => {
                if let Some(callback) =
                    self.try_get_mouse_bind(position, Bind::MouseButtonUp(button))
                {
                    Some((callback, Data::Mouse(event)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
