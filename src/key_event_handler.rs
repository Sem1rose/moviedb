use crossterm::event::KeyModifiers;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

use crate::{App, Drawer};

pub enum Data {
    None,
    Direction(bool, KeyModifiers),
    Key(KeyEvent),
}

type State = (Option<usize>, Option<usize>);
type Callback = Box<dyn FnOnce(&mut App, Data)>;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Bind {
    Horizontal,
    Vertical,
    Enter,
    Esc,
    Tab,
    Key(char),
    Input,
}

#[derive(Default)]
pub struct KeyEventHandler {
    binds: HashMap<(Bind, State), Callback>,
    execute_immediate: Vec<Callback>,
}

impl KeyEventHandler {
    pub fn clear(&mut self) {
        self.binds.clear();

        self.bind_key((None, None), 'q', |app, _| app.quit = true);
    }

    pub fn bind_immediate(&mut self, callback: impl FnOnce(&mut App, Data) + 'static) {
        self.execute_immediate.push(Box::new(callback));
    }

    pub fn bind_horizontal(
        &mut self,
        state: State,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .binds
            .insert((Bind::Horizontal, state), Box::new(callback));
    }
    pub fn bind_vertical(&mut self, state: State, callback: impl FnOnce(&mut App, Data) + 'static) {
        _ = self
            .binds
            .insert((Bind::Vertical, state), Box::new(callback));
    }
    pub fn bind_tab(&mut self, state: State, callback: impl FnOnce(&mut App, Data) + 'static) {
        _ = self.binds.insert((Bind::Tab, state), Box::new(callback));
    }
    pub fn bind_input_field(
        &mut self,
        state: State,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self.binds.insert((Bind::Input, state), Box::new(callback));
    }
    pub fn bind_esc(&mut self, state: State, callback: impl FnOnce(&mut App, Data) + 'static) {
        _ = self.binds.insert((Bind::Esc, state), Box::new(callback));
    }
    pub fn bind_enter(&mut self, state: State, callback: impl FnOnce(&mut App, Data) + 'static) {
        _ = self.binds.insert((Bind::Enter, state), Box::new(callback));
    }
    pub fn bind_key(
        &mut self,
        state: State,
        key: char,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .binds
            .insert((Bind::Key(key), state), Box::new(callback));
    }

    fn try_get_bind(&mut self, state: State, bind: Bind) -> Option<Callback> {
        if let Some(callback) = self.binds.remove(&(bind, state)) {
            Some(callback)
        } else if let Some(callback) = self.binds.remove(&(bind, (state.0, None))) {
            Some(callback)
        } else if let Some(callback) = self.binds.remove(&(bind, (None, None))) {
            Some(callback)
        } else {
            None
        }
    }

    pub fn execute_immediates(&mut self) -> Vec<Callback> {
        self.execute_immediate.drain(..).collect()
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
        drawer: &Drawer,
    ) -> anyhow::Result<Option<(Callback, Data)>> {
        let state;
        if let Some(popup) = drawer.active_popup.as_ref() {
            match popup {
                crate::popups::Popups::EditMovie(edit_movie_popup) => {
                    state = edit_movie_popup.get_state();
                }
                crate::popups::Popups::RemoveMovie(remove_movie_popup) => {
                    state = remove_movie_popup.get_state();
                }
                crate::popups::Popups::AddMovie(add_movie_popup) => {
                    state = add_movie_popup.get_state();
                }
            }
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => {
                    state = main_screen.get_state();
                }
            }
        } else {
            return Ok(None);
        }

        match event.code {
            KeyCode::Up | KeyCode::Down => {
                if let Some(callback) = self.try_get_bind(state, Bind::Vertical) {
                    Ok(Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Down, event.modifiers),
                    )))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if let Some(callback) = self.try_get_bind(state, Bind::Tab) {
                    Ok(Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Tab, KeyModifiers::NONE),
                    )))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Enter => {
                if let Some(callback) = self.try_get_bind(state, Bind::Enter) {
                    Ok(Some((callback, Data::None)))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Esc => {
                if let Some(callback) = self.try_get_bind(state, Bind::Esc) {
                    Ok(Some((callback, Data::None)))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Some(callback) = self.try_get_bind(state, Bind::Input) {
                    Ok(Some((callback, Data::Key(event))))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Left | KeyCode::Right => {
                if let Some(callback) = self.try_get_bind(state, Bind::Input) {
                    Ok(Some((callback, Data::Key(event))))
                } else if let Some(callback) = self.try_get_bind(state, Bind::Horizontal) {
                    Ok(Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Right, event.modifiers),
                    )))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Char(key) => {
                if let Some(callback) = self.try_get_bind(state, Bind::Input) {
                    Ok(Some((callback, Data::Key(event))))
                } else if let Some(callback) = self.try_get_bind(state, Bind::Key(key)) {
                    Ok(Some((callback, Data::Key(event))))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}
