use itertools::Itertools;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Position, Rect},
};
use std::collections::HashMap;

use crate::{App, Drawer};

pub enum Data {
    None,
    Direction(bool, KeyModifiers),
    Key(KeyEvent),
    Mouse(MouseEvent),
}

type State = (Option<usize>, Option<usize>);
type Callback = Box<dyn FnOnce(&mut App, Data)>;

#[derive(PartialEq, Eq, Hash, Clone)]
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

#[derive(Default)]
pub struct KeyEventHandler {
    key_binds: HashMap<(Bind, State), (String, Callback)>,
    execute_immediate: Vec<Callback>,
    mouse_binds: HashMap<(usize, Bind, Rect), Callback>,

    semi_bind: Option<char>,
}

impl KeyEventHandler {
    pub fn clear(&mut self) {
        self.key_binds.clear();
        self.mouse_binds.clear();

        self.bind_key((None, None), 'q', "Quit".into(), |app, _| app.quit = true);
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
        _ = self
            .key_binds
            .insert((Bind::Horizontal, state), (description, Box::new(callback)));
    }
    pub fn bind_vertical(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .key_binds
            .insert((Bind::Vertical, state), (description, Box::new(callback)));
    }
    pub fn bind_tab(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .key_binds
            .insert((Bind::Tab, state), (description, Box::new(callback)));
    }
    pub fn bind_input_field(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .key_binds
            .insert((Bind::Input, state), (description, Box::new(callback)));
    }
    pub fn bind_esc(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .key_binds
            .insert((Bind::Esc, state), (description, Box::new(callback)));
    }
    pub fn bind_enter(
        &mut self,
        state: State,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self
            .key_binds
            .insert((Bind::Enter, state), (description, Box::new(callback)));
    }
    pub fn bind_key(
        &mut self,
        state: State,
        keys: impl ToString,
        description: String,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self.key_binds.insert(
            (Bind::Key(keys.to_string()), state),
            (description, Box::new(callback)),
        );
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
    pub fn bind_mouse_button_up(
        &mut self,
        button: MouseButton,
        area: Rect,
        callback: impl FnOnce(&mut App, Data) + 'static,
    ) {
        _ = self.mouse_binds.insert(
            (self.mouse_binds.len(), Bind::MouseButtonUp(button), area),
            Box::new(callback),
        );
    }

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

    fn try_get_key_bind(&mut self, state: State, bind: Bind) -> Option<Callback> {
        for s in [state, (state.0, None), (None, state.1), (None, None)] {
            if let Some((_, bind)) = self.key_binds.remove(&(bind.clone(), s)) {
                return Some(bind);
            }
        }

        None
    }

    fn try_get_keys_bind(&mut self, state: State, key: char) -> Option<Callback> {
        let key = if let Some(semi_bind) = self.semi_bind {
            String::from_iter([semi_bind, key])
        } else {
            key.to_string()
        };

        if let Some(callback) = self.try_get_key_bind(state, Bind::Key(key.clone())) {
            self.semi_bind = None;

            return Some(callback);
        } else if self.semi_bind.is_some() {
            self.semi_bind = None;
            return None;
        }

        for s in [state, (state.0, None), (None, state.1), (None, None)] {
            if self
                .key_binds
                .iter()
                .filter(|((bind, state), _)| {
                    state == &s
                        && if let Bind::Key(k) = bind {
                            k.starts_with(&key.clone())
                        } else {
                            false
                        }
                })
                .count()
                > 0
            {
                self.semi_bind = Some(key.chars().nth(0).unwrap());

                return None;
            }
        }

        None
    }

    pub fn get_key_binds_descriptions(&self, state: State, max: usize) -> Vec<(Bind, String)> {
        let mut binds = vec![];

        if let Some(semi_bind) = self.semi_bind {
            for s in [state, (state.0, None), (None, state.1), (None, None)] {
                let matches: Vec<(
                    &(Bind, (Option<usize>, Option<usize>)),
                    &(String, Box<dyn FnOnce(&mut App, Data)>),
                )> = self
                    .key_binds
                    .iter()
                    .filter(|((bind, state), _)| {
                        state == &s
                            && if let Bind::Key(k) = bind {
                                k.starts_with(&semi_bind.to_string())
                            } else {
                                false
                            }
                    })
                    .collect_vec();
                if !matches.is_empty() {
                    binds.extend(matches.iter().map(|&(k, v)| (k.0.clone(), v.0.clone())));

                    break;
                }
            }
        } else {
            for bind in [
                Bind::Horizontal,
                Bind::Vertical,
                Bind::Enter,
                Bind::Esc,
                Bind::Tab,
            ] {
                for state in [state, (state.0, None), (None, state.1), (None, None)] {
                    if self.key_binds.contains_key(&(bind.clone(), state)) {
                        binds.push((
                            bind.clone(),
                            self.key_binds[&(bind.clone(), state)].0.clone(),
                        ));
                        break;
                    }
                }
            }

            let mut input = false;
            for s in [state, (state.0, None), (None, state.1), (None, None)] {
                if self.key_binds.contains_key(&(Bind::Input, s)) {
                    input = true;
                    break;
                }
            }
            if !input {
                for s in [state, (state.0, None), (None, state.1), (None, None)] {
                    let matches: Vec<(
                        &(Bind, (Option<usize>, Option<usize>)),
                        &(String, Box<dyn FnOnce(&mut App, Data)>),
                    )> = self
                        .key_binds
                        .iter()
                        .filter(|((bind, state), _)| {
                            state == &s && if let Bind::Key(_) = bind { true } else { false }
                        })
                        .collect_vec();
                    if !matches.is_empty() {
                        binds.extend(matches.iter().map(|&(k, v)| (k.0.clone(), v.0.clone())));

                        break;
                    }
                }
            }
        }

        binds
            .into_iter()
            .filter(|(_, x)| !x.is_empty())
            .take(max)
            .collect()
    }

    pub fn execute_immediates(&mut self) -> Vec<Callback> {
        self.execute_immediate.drain(..).collect()
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
        drawer: &Drawer,
    ) -> Option<(Callback, Data)> {
        let state;
        if let Some(popup) = drawer.active_popup.as_ref() {
            match popup {
                crate::popups::Popups::EditMovie(edit_movie_popup) => {
                    state = edit_movie_popup.get_state();
                }
                crate::popups::Popups::DeleteMovie(delete_movie_popup) => {
                    state = delete_movie_popup.get_state();
                }
                crate::popups::Popups::AddMovie(add_movie_popup) => {
                    state = add_movie_popup.get_state();
                }
                crate::popups::Popups::TMDBInit(tmdb_init) => {
                    state = tmdb_init.get_state();
                }
                crate::popups::Popups::OMDBInit(omdb_init) => {
                    state = omdb_init.get_state();
                }
                crate::popups::Popups::TraktInit(trakt_init) => {
                    state = trakt_init.get_state();
                }
                crate::popups::Popups::FetchArtworks(_) => {
                    return None;
                }
            }
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => {
                    state = main_screen.get_state();
                }
            }
        } else {
            return None;
        }

        match event.code {
            KeyCode::Up | KeyCode::Down => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Vertical) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Down, event.modifiers),
                    ))
                } else {
                    None
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Tab) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Tab, KeyModifiers::NONE),
                    ))
                } else {
                    None
                }
            }
            KeyCode::Enter => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Enter) {
                    Some((callback, Data::None))
                } else {
                    None
                }
            }
            KeyCode::Esc => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Esc) {
                    Some((callback, Data::None))
                } else {
                    None
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Input) {
                    Some((callback, Data::Key(event)))
                } else {
                    None
                }
            }
            KeyCode::Left | KeyCode::Right => {
                if self.semi_bind.is_some() {
                    self.semi_bind = None;
                    None
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Input) {
                    Some((callback, Data::Key(event)))
                } else if let Some(callback) = self.try_get_key_bind(state, Bind::Horizontal) {
                    Some((
                        callback,
                        Data::Direction(event.code == KeyCode::Right, event.modifiers),
                    ))
                } else {
                    None
                }
            }
            KeyCode::Char(key) => {
                if let Some(callback) = self.try_get_key_bind(state, Bind::Input) {
                    Some((callback, Data::Key(event)))
                } else if let Some(callback) = self.try_get_keys_bind(state, key) {
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
        let state;
        if let Some(popup) = drawer.active_popup.as_ref() {
            match popup {
                crate::popups::Popups::EditMovie(edit_movie_popup) => {
                    state = edit_movie_popup.get_state();
                }
                crate::popups::Popups::DeleteMovie(delete_movie_popup) => {
                    state = delete_movie_popup.get_state();
                }
                crate::popups::Popups::AddMovie(add_movie_popup) => {
                    state = add_movie_popup.get_state();
                }
                crate::popups::Popups::TMDBInit(tmdb_init) => {
                    state = tmdb_init.get_state();
                }
                crate::popups::Popups::OMDBInit(omdb_init) => {
                    state = omdb_init.get_state();
                }
                crate::popups::Popups::TraktInit(trakt_init) => {
                    state = trakt_init.get_state();
                }
                crate::popups::Popups::FetchArtworks(_) => {
                    return None;
                }
            }
        } else if let Some(screen) = drawer.current_screen.as_ref() {
            match screen {
                crate::screens::Screens::MainScreen(main_screen) => {
                    state = main_screen.get_state();
                }
            }
        } else {
            return None;
        }

        let position = Position {
            x: event.column,
            y: event.row,
        };
        match event.kind {
            MouseEventKind::ScrollDown => {
                if let Some(callback) = self.try_get_key_bind(state, Bind::Vertical) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(callback) = self.try_get_key_bind(state, Bind::Vertical) {
                    Some((callback, Data::Direction(false, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollRight => {
                if let Some(callback) = self.try_get_key_bind(state, Bind::Horizontal) {
                    Some((callback, Data::Direction(true, event.modifiers)))
                } else {
                    None
                }
            }
            MouseEventKind::ScrollLeft => {
                if let Some(callback) = self.try_get_key_bind(state, Bind::Horizontal) {
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
