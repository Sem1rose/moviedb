use itertools::Itertools;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Position, Rect},
};
use rustc_hash::FxHashMap;

use crate::{app::App, drawer::Drawer};

#[derive(Clone, Copy)]
pub enum Data {
    None,
    Direction(bool, KeyModifiers),
    Key(KeyEvent),
    Mouse(MouseEvent),
}

type State = (Option<usize>, Option<usize>);
type Callback = Box<dyn FnMut(&mut App, Data)>;

fn states_equal(current_state: &State, bind_state: &State) -> bool {
    bind_state
        .0
        .map(|x| current_state.0.is_some() && x == current_state.0.unwrap())
        .unwrap_or(true)
        && bind_state
            .1
            .map(|x| current_state.1.is_some() && x == current_state.1.unwrap())
            .unwrap_or(true)
}
fn state_key(s: &State) -> usize {
    ((s.0.is_some() as usize) << 1) + s.1.is_some() as usize
}

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
pub struct EventHandler {
    execute_immediate: Vec<Callback>,
    mouse_binds:       FxHashMap<(usize, Bind, Rect), Callback>,
    key_binds:         FxHashMap<(Bind, State), (String, Callback)>,

    semi_bind: Option<char>,
}

impl EventHandler {
    pub fn clear(&mut self) {
        self.key_binds.clear();
        self.mouse_binds.clear();

        self.bind_key((None, None), 'q', "Quit".into(), |app, _| app.quit = true);
    }

    pub fn bind_immediate(&mut self, callback: impl FnMut(&mut App, Data) + 'static) {
        self.execute_immediate.push(Box::new(callback));
    }

    fn add_key_bind(
        &mut self,
        state: State,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
        bind: Bind,
    ) {
        _ = self
            .key_binds
            .insert((bind, state), (description, Box::new(callback)));
    }

    pub fn bind_horizontal(
        &mut self,
        state: State,
        description: String,
        area: Option<Rect>,
        callback: impl Fn(&mut App, Data) + Clone + 'static,
    ) {
        self.add_key_bind(state, description, callback.clone(), Bind::Horizontal);
        if let Some(area) = area {
            self.mouse_binds.insert(
                (self.mouse_binds.len(), Bind::Horizontal, area),
                Box::new(callback),
            );
        }
    }

    pub fn bind_vertical(
        &mut self,
        state: State,
        description: String,
        area: Option<Rect>,
        callback: impl Fn(&mut App, Data) + Clone + 'static,
    ) {
        self.add_key_bind(state, description, callback.clone(), Bind::Vertical);
        if let Some(area) = area {
            self.mouse_binds.insert(
                (self.mouse_binds.len(), Bind::Vertical, area),
                Box::new(callback),
            );
        }
    }

    pub fn bind_tab(
        &mut self,
        state: State,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Tab)
    }

    pub fn bind_input_field(
        &mut self,
        state: State,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Input)
    }

    pub fn bind_esc(
        &mut self,
        state: State,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Esc)
    }

    pub fn bind_enter(
        &mut self,
        state: State,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Enter)
    }

    pub fn bind_key(
        &mut self,
        state: State,
        keys: impl ToString,
        description: String,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        self.add_key_bind(state, description, callback, Bind::Key(keys.to_string()))
    }

    pub fn bind_mouse_button_down(
        &mut self,
        button: MouseButton,
        area: Rect,
        callback: impl Fn(&mut App, Data) + 'static,
    ) {
        if area.is_empty() {
            return;
        }

        _ = self.mouse_binds.insert(
            (self.mouse_binds.len(), Bind::MouseButtonDown(button), area),
            Box::new(callback),
        );
    }

    // pub fn bind_mouse_button_up(
    //     &mut self,
    //     button: MouseButton,
    //     area: Rect,
    //     callback: impl Fn(&mut App, Data) + 'static,
    // ) {
    //     _ = self.mouse_binds.insert(
    //         (self.mouse_binds.len(), Bind::MouseButtonUp(button), area),
    //         Box::new(callback),
    //     );
    // }

    fn try_get_mouse_bind(&mut self, position: Position, bind: Bind) -> Option<Callback> {
        let key = self
            .mouse_binds
            .keys()
            .filter(|(_, b, rect)| b == &bind && rect.contains(position))
            .sorted_by_key(|x| x.0)
            .next_back()?;

        self.mouse_binds.remove(&key.clone())
    }

    pub fn try_get_bind(&mut self, bind: Bind, state: State) -> Option<Callback> {
        let key = self
            .key_binds
            .keys()
            .filter(|(b, s)| b == &bind && states_equal(&state, s))
            .sorted_by_key(|(_, s)| state_key(s))
            .next_back()?;

        self.key_binds
            .remove(&key.clone())
            .map(|(_, callback)| callback)
    }

    fn try_get_keys_bind(&mut self, key: char, state: State) -> Option<Callback> {
        let key = if let Some(semi_bind) = self.semi_bind {
            String::from_iter([semi_bind, key])
        } else {
            key.to_string()
        };

        if let Some(callback) = self.try_get_bind(Bind::Key(key.clone()), state) {
            self.semi_bind = None;

            return Some(callback);
        } else if self.semi_bind.is_some() {
            self.semi_bind = None;
            return None;
        }

        if self.key_binds.iter().any(|((bind, s), _)| {
            (if let Bind::Key(k) = bind {
                k.starts_with(&key.clone())
            } else {
                false
            }) && states_equal(&state, s)
        }) {
            self.semi_bind = Some(key.chars().nth(0).unwrap());
        }

        None
    }

    pub fn get_key_binds_descriptions(&self, drawer: &Drawer, max: usize) -> Vec<(Bind, String)> {
        let any_input = self
            .key_binds
            .iter()
            .any(|((b, s), _)| matches!(b, Bind::Input) && states_equal(&drawer.state, s));
        let binds = self
            .key_binds
            .iter()
            .filter(|((bind, s), _)| {
                (if let Some(semi_bind) = self.semi_bind {
                    if let Bind::Key(k) = bind {
                        k.starts_with(&semi_bind.to_string())
                    } else {
                        false
                    }
                } else {
                    if !any_input {
                        matches!(
                            bind,
                            Bind::Horizontal
                                | Bind::Vertical
                                | Bind::Enter
                                | Bind::Esc
                                | Bind::Tab
                                | Bind::Key(_)
                        )
                    } else {
                        matches!(
                            bind,
                            Bind::Horizontal | Bind::Vertical | Bind::Enter | Bind::Esc | Bind::Tab
                        )
                    }
                }) && states_equal(&drawer.state, s)
            })
            .filter_map(
                |((b, s), (d, _))| {
                    if !d.is_empty() { Some((b, *s, d)) } else { None }
                },
            )
            .sorted_by_key(|(b, _, _)| b.sort_key())
            .sorted_by_key(|(_, s, _)| state_key(s))
            .collect_vec();

        binds
            .into_iter()
            .chunk_by(|a| a.0.clone())
            .into_iter()
            .filter_map(|(_, g)| g.last())
            .take(max)
            .map(|(b, _, d)| (b.clone(), d.clone()))
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
        match event.code {
            KeyCode::Tab | KeyCode::BackTab =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_bind(Bind::Tab, drawer.state) {
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
                } else if let Some(callback) = self.try_get_bind(Bind::Enter, drawer.state) {
                    Some((callback, Data::None))
                } else {
                    None
                },
            KeyCode::Esc =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_bind(Bind::Esc, drawer.state) {
                    Some((callback, Data::None))
                } else {
                    None
                },
            KeyCode::Backspace | KeyCode::Delete =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_bind(Bind::Input, drawer.state) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                },
            KeyCode::Up | KeyCode::Down =>
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_bind(Bind::Vertical, drawer.state) {
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
                } else if let Some(callback) = self.try_get_bind(Bind::Input, drawer.state) {
                    Some((callback, Data::Key(event)))
                } else if let Some(callback) = self.try_get_bind(Bind::Horizontal, drawer.state) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Right, event.modifiers),
                    ))
                } else {
                    None
                },
            KeyCode::Char(key) => {
                if let Some(callback) = self.try_get_bind(Bind::Input, drawer.state) {
                    Some((callback, Data::Key(event)))
                } else if let Some(callback) = self.try_get_keys_bind(key, drawer.state) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        drawer: &Drawer,
    ) -> Option<(Callback, Data)> {
        let position = Position {
            x: event.column,
            y: event.row,
        };
        match event.kind {
            MouseEventKind::ScrollDown => {
                if let Some(callback) = self.try_get_mouse_bind(position, Bind::Vertical) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                // } else if let Some(callback) = self.try_get_bind(Bind::Vertical, drawer.state) {
                //     Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(callback) = self.try_get_mouse_bind(position, Bind::Vertical) {
                    Some((callback, Data::Direction(false, event.modifiers)))
                // } else if let Some(callback) = self.try_get_bind(Bind::Vertical, drawer.state) {
                //     Some((callback, Data::Direction(false, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollRight => {
                if let Some(callback) = self.try_get_mouse_bind(position, Bind::Horizontal) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                // } else if let Some(callback) = self.try_get_bind(Bind::Horizontal, drawer.state) {
                //     Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollLeft => {
                if let Some(callback) = self.try_get_mouse_bind(position, Bind::Horizontal) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                // } else if let Some(callback) = self.try_get_bind(Bind::Horizontal, drawer.state) {
                //     Some((callback, Data::Direction(false, event.modifiers)))
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
