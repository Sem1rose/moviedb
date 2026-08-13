use std::{cmp::Ordering, rc::Rc};

use itertools::Itertools;
use nucleo_matcher::{Config, Matcher, pattern::Atom};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, HorizontalAlignment, Layout, Margin, Offset, Rect, Size},
    macros::{constraint, constraints, horizontal, line, span},
    style::{
        Modifier, Style, Stylize,
        palette::{material, tailwind},
    },
    symbols::border,
    widgets::{Block, Borders, Padding},
};
use ratatui_textarea::TextArea;
use strum::IntoEnumIterator;

use crate::{
    app::App,
    helpers,
    key_event_handler::{self, KeyEventHandler},
    pop_criterion,
    popups::{PopupTrait, Popups},
    screens::Screens,
    types::{
        BoxedFn, BoxedMutFn, FilterCriterion, FilterCriterionDiscriminants, FxIndexMap, Movie,
        Person,
    },
    widgets::{self, Action, ActionType, ContextMenu},
};

#[derive(Clone, Copy)]
pub enum DropdownType {
    Normal,
    MultipleChoice,
}
#[derive(Clone, Copy)]
pub enum TextInputType {
    Normal,
    Number,
    Rating,
}
pub enum Widget {
    Dropdown {
        item:               usize,
        dropdown_type:      DropdownType,
        text_input:         TextArea<'static>,
        constraint:         Constraint,
        items:              Vec<String>,
        filtered_items:     Vec<usize>,
        current_selected:   usize,
        scroll_pos:         usize,
        selected_items:     Vec<usize>,
        num_visible_items:  usize,
        placeholder_text:   String,
        search_placeholder: Option<String>,
        visible_if:         Option<BoxedFn<AdvancedFilterPopup, bool>>,
    },
    TextInput {
        item:            usize,
        text_input_type: TextInputType,
        constraint:      Constraint,
        placeholder:     String,
        title:           String,
        text_input:      TextArea<'static>,
        visible_if:      Option<BoxedFn<AdvancedFilterPopup, bool>>,
    },
    StaticText {
        text:       String,
        constraint: Constraint,
        visible_if: Option<BoxedFn<AdvancedFilterPopup, bool>>,
    },
}
impl Widget {
    fn new_normal_dropdown(
        item: usize,
        constraint: Constraint,
        items: Vec<String>,
        num_visible_items: usize,
        placeholder: String,
        search_placeholder: Option<String>,
    ) -> Self {
        Self::Dropdown {
            item,
            dropdown_type: DropdownType::Normal,
            text_input: TextArea::default(),
            constraint,
            filtered_items: (0..items.len()).collect(),
            items,
            current_selected: 0,
            scroll_pos: 0,
            selected_items: vec![],
            num_visible_items,
            placeholder_text: placeholder,
            search_placeholder,
            visible_if: None,
        }
    }

    fn new_multiple_choice_dropdown(
        item: usize,
        constraint: Constraint,
        items: Vec<String>,
        num_visible_items: usize,
        placeholder: String,
        search_placeholder: Option<String>,
    ) -> Self {
        Self::Dropdown {
            item,
            dropdown_type: DropdownType::MultipleChoice,
            text_input: TextArea::default(),
            constraint,
            filtered_items: (0..items.len()).collect(),
            items,
            current_selected: 0,
            scroll_pos: 0,
            selected_items: vec![],
            num_visible_items,
            placeholder_text: placeholder,
            search_placeholder,
            visible_if: None,
        }
    }

    fn new_text_input(
        item: usize,
        constraint: Constraint,
        title: String,
        placeholder: String,
    ) -> Self {
        Self::TextInput {
            item,
            text_input_type: TextInputType::Normal,
            constraint,
            title,
            placeholder,
            text_input: TextArea::default(),
            visible_if: None,
        }
    }

    fn new_number_input(
        item: usize,
        constraint: Constraint,
        title: String,
        placeholder: String,
    ) -> Self {
        Self::TextInput {
            item,
            text_input_type: TextInputType::Number,
            constraint,
            title,
            placeholder,
            text_input: TextArea::default(),
            visible_if: None,
        }
    }

    fn new_rating_input(
        item: usize,
        constraint: Constraint,
        title: String,
        placeholder: String,
    ) -> Self {
        Self::TextInput {
            item,
            text_input_type: TextInputType::Rating,
            constraint,
            title,
            placeholder,
            text_input: TextArea::default(),
            visible_if: None,
        }
    }

    fn new_static_text(text: String, width: Option<Constraint>) -> Self {
        Self::StaticText {
            constraint: width.unwrap_or(constraint!(==text.len() as u16)),
            text,
            visible_if: None,
        }
    }

    fn and_visible_if(mut self, condition: BoxedFn<AdvancedFilterPopup, bool>) -> Self {
        match &mut self {
            Widget::Dropdown { visible_if, .. }
            | Widget::TextInput { visible_if, .. }
            | Widget::StaticText { visible_if, .. } => *visible_if = Some(condition),
        }

        self
    }

    fn get_constraint(&self) -> Constraint {
        match self {
            Widget::Dropdown { constraint, .. }
            | Widget::TextInput { constraint, .. }
            | Widget::StaticText { constraint, .. } => *constraint,
        }
    }

    fn bind(
        &mut self,
        key_event_handler: &mut KeyEventHandler,
        valid: bool,
        area: Rect,
        selected_item: usize,
        tab_selected: bool,
        last_item: bool,
    ) {
        match self {
            Widget::StaticText { .. } => (),
            Widget::Dropdown {
                item,
                text_input,
                dropdown_type,
                items,
                filtered_items,
                current_selected,
                scroll_pos,
                selected_items,
                num_visible_items,
                search_placeholder,
                ..
            } => {
                let item = *item;
                let selected = item == selected_item;
                let dropdown_type = *dropdown_type;
                let can_search = search_placeholder.is_some();
                let input_empty = text_input.is_empty();
                key_event_handler.bind_enter(
                    (Some(2), Some(item)),
                    if matches!(dropdown_type, DropdownType::MultipleChoice)
                        && can_search
                        && !input_empty
                    {
                        "Toggle"
                    } else if last_item && valid {
                        "Confirm"
                    } else {
                        "Select"
                    }
                    .into(),
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match advanced_filter_popup.get_widget_at_mut(item).unwrap() {
                                Widget::Dropdown {
                                    filtered_items,
                                    current_selected,
                                    selected_items,
                                    ..
                                } =>
                                    if matches!(dropdown_type, DropdownType::Normal) {
                                        *selected_items = vec![filtered_items[*current_selected]];
                                    } else if can_search && !input_empty {
                                        if let Some(index) = selected_items
                                            .iter()
                                            .position(|x| *x == filtered_items[*current_selected])
                                        {
                                            selected_items.remove(index);
                                        } else {
                                            selected_items.push(filtered_items[*current_selected]);
                                        }
                                    },
                                _ => unreachable!(),
                            }

                            if !matches!(dropdown_type, DropdownType::MultipleChoice)
                                || !can_search
                                || input_empty
                            {
                                // possible bug if `validate` doesn't only check if `selected_items` is empty
                                if last_item
                                    && (valid
                                        || (can_search
                                            && matches!(dropdown_type, DropdownType::Normal)))
                                {
                                    advanced_filter_popup.confirm.as_ref().unwrap().clone()(
                                        advanced_filter_popup,
                                    );
                                } else {
                                    advanced_filter_popup.item += 1;
                                }
                            }
                            // }
                        }
                    },
                );

                key_event_handler.bind_vertical(
                    (Some(2), Some(item)),
                    "Choose".into(),
                    move |app, data| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match advanced_filter_popup.get_widget_at_mut(item).unwrap() {
                                Widget::Dropdown {
                                    filtered_items,
                                    current_selected,
                                    scroll_pos,
                                    num_visible_items,
                                    ..
                                } => match data {
                                    crate::key_event_handler::Data::Direction(true, _) =>
                                        if *current_selected < filtered_items.len() - 1 {
                                            *current_selected += 1;
                                            if *current_selected < *scroll_pos
                                                || *current_selected - *scroll_pos
                                                    >= *num_visible_items
                                            {
                                                *scroll_pos = current_selected
                                                    .saturating_sub(*num_visible_items - 1)
                                            }
                                        },
                                    crate::key_event_handler::Data::Direction(false, _) => {
                                        *current_selected = current_selected.saturating_sub(1);
                                        if *current_selected < *scroll_pos {
                                            *scroll_pos -= 1
                                        }
                                    }
                                    _ => {}
                                },
                                _ => unreachable!(),
                            }
                        }
                    },
                );

                if matches!(dropdown_type, DropdownType::MultipleChoice) && !can_search {
                    key_event_handler.bind_key(
                        (Some(2), Some(item)),
                        ' ',
                        "Toggle".into(),
                        move |app, _| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                match advanced_filter_popup.get_widget_at_mut(item).unwrap() {
                                    Widget::Dropdown {
                                        filtered_items,
                                        current_selected,
                                        selected_items,
                                        ..
                                    } => {
                                        if let Some(index) = selected_items
                                            .iter()
                                            .position(|x| *x == filtered_items[*current_selected])
                                        {
                                            selected_items.remove(index);
                                        } else {
                                            selected_items.push(filtered_items[*current_selected]);
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        },
                    );
                }

                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    area,
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.tab = 2;
                            advanced_filter_popup.item = item;
                        }
                    },
                );

                if can_search {
                    key_event_handler.bind_input_field(
                        (Some(2), Some(item)),
                        "".into(),
                        move |app, data| {
                            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                app.drawer.active_popup.as_mut()
                            {
                                if let key_event_handler::Data::Key(key_event) = data {
                                    if let Widget::Dropdown {
                                        text_input,
                                        items,
                                        filtered_items,
                                        current_selected,
                                        scroll_pos,
                                        selected_items,
                                        ..
                                    } = advanced_filter_popup.get_widget_at_mut(item).unwrap()
                                    {
                                        if text_input.is_empty()
                                            && matches!(dropdown_type, DropdownType::MultipleChoice)
                                            && matches!(
                                                key_event,
                                                KeyEvent {
                                                    code: KeyCode::Char(' '),
                                                    ..
                                                }
                                            )
                                        {
                                            if let Some(index) =
                                                selected_items.iter().position(|x| {
                                                    *x == filtered_items[*current_selected]
                                                })
                                            {
                                                selected_items.remove(index);
                                            } else {
                                                selected_items
                                                    .push(filtered_items[*current_selected]);
                                            }
                                            return;
                                        }
                                        // if matches!(key_event, KeyEvent { code: KeyCode::Right, .. }) {
                                        //     if advanced_filter_popup.get_widget_at(item + 1).is_some() {
                                        //         advanced_filter_popup.item += 1;
                                        //         return;
                                        //     }
                                        // } else if matches!(key_event, KeyEvent { code: KeyCode::Left, .. }) {
                                        //     if item > 0 && advanced_filter_popup.get_widget_at(item - 1).is_some() {
                                        //         advanced_filter_popup.item -= 1;
                                        //         return;
                                        //     }
                                        // }

                                        text_input.input(key_event);
                                        let search_text = text_input.lines()[0].trim();

                                        if !search_text.is_empty() {
                                            let mut conf = Config::DEFAULT;
                                            conf.prefer_prefix = true;
                                            let mut matcher = Matcher::new(conf);
                                            let pattern = Atom::parse(
                                                search_text,
                                                nucleo_matcher::pattern::CaseMatching::Ignore,
                                                nucleo_matcher::pattern::Normalization::Never,
                                            );
                                            let mut scores = vec![];
                                            for item in items.iter().enumerate() {
                                                if let Some(score) = pattern.score(
                                                    nucleo_matcher::Utf32Str::Ascii(
                                                        item.1.as_bytes(),
                                                    ),
                                                    &mut matcher,
                                                ) {
                                                    scores.push((score, item));
                                                }
                                            }

                                            *filtered_items =
                                                scores.iter().map(|&(_, (x, _))| x).collect();
                                        } else {
                                            *filtered_items = (0..items.len()).collect();
                                        }
                                        *scroll_pos = 0;
                                        *current_selected = 0;
                                    }
                                }
                            }
                        },
                    );
                }

                if selected && tab_selected {
                    let num_visible_items =
                        (*num_visible_items).min(filtered_items.len() - *scroll_pos);
                    let mut mouse_area = area
                        .offset(Offset::new(1, 3))
                        .resize(Size::new(area.width - 2, 1));
                    for i in 0..num_visible_items {
                        let index = i + *scroll_pos;
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    match advanced_filter_popup.get_widget_at_mut(item).unwrap() {
                                        Widget::Dropdown {
                                            dropdown_type,
                                            filtered_items,
                                            current_selected,
                                            selected_items,
                                            ..
                                        } => {
                                            *current_selected = index;
                                            if matches!(dropdown_type, DropdownType::MultipleChoice)
                                            {
                                                if let Some(index) = selected_items
                                                    .iter()
                                                    .position(|x| *x == filtered_items[index])
                                                {
                                                    selected_items.remove(index);
                                                } else {
                                                    selected_items.push(filtered_items[index]);
                                                }
                                            } else {
                                                *selected_items = vec![filtered_items[index]];
                                                if last_item
                                                    && (valid
                                                        || (can_search
                                                            && matches!(
                                                                dropdown_type,
                                                                DropdownType::Normal
                                                            )))
                                                {
                                                    // possible bug if `validate` doesn't only check if `selected_items` is empty
                                                    advanced_filter_popup
                                                        .confirm
                                                        .as_ref()
                                                        .unwrap()
                                                        .clone()(
                                                        advanced_filter_popup
                                                    );
                                                } else {
                                                    advanced_filter_popup.item += 1;
                                                }
                                            }
                                        }
                                        _ => unreachable!(),
                                    };
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                } else {
                    text_input.clear();
                    *filtered_items = (0..items.len()).collect();

                    if matches!(dropdown_type, DropdownType::MultipleChoice) {
                        *scroll_pos = 0;
                        *current_selected = 0;
                    } else if can_search {
                        *current_selected = selected_items.first().copied().unwrap_or(0);
                        *scroll_pos = current_selected.saturating_sub(*num_visible_items - 1)
                    }
                }
            }
            Widget::TextInput {
                item,
                text_input_type,
                ..
            } => {
                let item = *item;
                let text_input_type = *text_input_type;

                key_event_handler.bind_enter(
                    (Some(2), Some(item)),
                    if last_item && valid { "Confirm" } else { "Select" }.into(),
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if last_item && valid {
                                advanced_filter_popup.confirm.as_ref().unwrap().clone()(
                                    advanced_filter_popup,
                                );
                            } else {
                                advanced_filter_popup.item += 1;
                            }
                        }
                    },
                );

                key_event_handler.bind_input_field(
                    (Some(2), Some(item)),
                    "".into(),
                    move |app, data| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if let key_event_handler::Data::Key(key_event) = data {
                                if let Widget::TextInput { text_input, .. } =
                                    advanced_filter_popup.get_widget_at_mut(item).unwrap()
                                {
                                    match text_input_type {
                                        TextInputType::Normal => {
                                            text_input.input(key_event);
                                        }
                                        TextInputType::Number => {
                                            if let KeyCode::Char(x) = &key_event.code {
                                                if text_input.lines()[0].len() >= 4 {
                                                    return;
                                                }

                                                if !x.is_ascii_digit() {
                                                    return;
                                                }
                                            }

                                            text_input.input(key_event);
                                            // if text_input.lines()[0].len() == 4 {
                                            //     text_input.scroll((0, -1));
                                            // }
                                        }
                                        TextInputType::Rating => {
                                            let parsed =
                                                text_input.lines()[0].parse::<f64>().unwrap_or(0.0);
                                            if let KeyCode::Char(x) = &key_event.code {
                                                if text_input.lines()[0].len() >= 3
                                                    || parsed >= 10.0
                                                {
                                                    return;
                                                }

                                                if !x.is_ascii_digit() && *x != '.' {
                                                    return;
                                                }
                                            }

                                            text_input.input(key_event);
                                            // if text_input.lines()[0].len() == 3 {
                                            //     text_input.scroll((0, -1));
                                            // }
                                        }
                                    }
                                }
                            }
                        }
                    },
                );
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    area,
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.tab = 2;
                            advanced_filter_popup.item = item;
                        }
                    },
                );
            }
        }
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        key_event_handler: &mut KeyEventHandler,
        area: Rect,
        selected_item: usize,
        tab_selected: bool,
    ) {
        match self {
            Widget::Dropdown {
                item,
                dropdown_type,
                text_input,
                items,
                filtered_items,
                current_selected,
                scroll_pos,
                selected_items,
                num_visible_items,
                placeholder_text,
                search_placeholder,
                ..
            } => {
                let selected = selected_item == *item;
                if let Some(placeholder) = search_placeholder {
                    widgets::dropdown(
                        tab_selected,
                        selected,
                        frame,
                        area,
                        if selected && tab_selected {
                            "".into()
                        } else {
                            match dropdown_type {
                                DropdownType::Normal => selected_items
                                    .first()
                                    .map(|x| items[*x].clone())
                                    .unwrap_or(placeholder_text.clone()),
                                DropdownType::MultipleChoice => placeholder_text.clone(),
                            }
                        },
                    );

                    if selected && tab_selected {
                        text_input.set_style(Style::new().fg(if tab_selected && selected {
                            tailwind::SLATE.c200
                        } else {
                            tailwind::STONE.c500
                        }));
                        text_input.set_cursor_style(
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
                        text_input.set_placeholder_text(placeholder.as_str());
                        text_input.set_placeholder_style(Style::new().fg(tailwind::STONE.c800));
                        text_input.set_block(
                            Block::bordered().border_set(border::PROPORTIONAL_WIDE).fg(
                                if tab_selected {
                                    if selected {
                                        material::BLUE.c600
                                    } else {
                                        material::INDIGO.c800
                                    }
                                } else {
                                    tailwind::SLATE.c700
                                },
                            ),
                        );
                        text_input.set_wrap_mode(ratatui_textarea::WrapMode::None);

                        frame.render_widget(&*text_input, area);
                    }
                } else {
                    widgets::dropdown(
                        tab_selected,
                        selected,
                        frame,
                        area,
                        match dropdown_type {
                            DropdownType::Normal =>
                                items[filtered_items[*current_selected]].clone(),
                            DropdownType::MultipleChoice => placeholder_text.clone(),
                        },
                    );
                }
                if tab_selected && selected {
                    let (mut mouse_area, len) = ContextMenu {
                        model: filtered_items
                            .iter()
                            .map(|x| helpers::ellipsize_string(&items[*x], area.width as usize - 4))
                            .collect_vec(),
                        selected_index: *current_selected,
                        scroll_pos: *scroll_pos,
                        num_visible_items: *num_visible_items,
                        ..Default::default()
                    }
                    .render_dropdown(area, frame, key_event_handler)
                    .into_iter()
                    .nth(0)
                    .unwrap()
                    .1;
                    if matches!(dropdown_type, DropdownType::MultipleChoice)
                        && !selected_items.is_empty()
                    {
                        for i in 0..len {
                            if selected_items.contains(&(filtered_items[i + *scroll_pos])) {
                                frame.render_widget(
                                    "".bg(tailwind::GREEN.c500).fg(tailwind::WHITE),
                                    mouse_area.offset(Offset { x: -1, y: 0 }),
                                );
                            }

                            mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                        }
                    }
                }
            }
            Widget::TextInput {
                item,
                text_input_type,
                placeholder,
                title,
                text_input,
                ..
            } => {
                let selected = selected_item == *item;
                widgets::input_field(
                    tab_selected,
                    selected,
                    !text_input.is_empty()
                        && match text_input_type {
                            TextInputType::Normal => true,
                            TextInputType::Number => text_input.lines()[0]
                                .trim()
                                .parse::<usize>()
                                .map(|x| x > 1800)
                                .unwrap_or(false),
                            TextInputType::Rating => text_input.lines()[0]
                                .trim()
                                .parse::<f64>()
                                .map(|x| x <= 10.0)
                                .unwrap_or(false),
                        },
                    text_input,
                    ratatui_textarea::WrapMode::None,
                    frame,
                    area,
                    title,
                    placeholder,
                    None,
                );
            }
            Widget::StaticText { text, .. } => {
                frame.render_widget(
                    span!(text).fg(tailwind::WHITE).into_right_aligned_line(),
                    helpers::add_padding(area, Padding::vertical(1)),
                );
            }
        }
    }
}

#[derive(Default)]
pub struct AdvancedFilterPopup {
    tab:             usize,
    item:            usize,
    filter_criteria: Vec<FilterCriterion>,

    available_criteria: Vec<FilterCriterionDiscriminants>,
    active_criterion:   Option<FilterCriterionDiscriminants>,

    dropdown_selected_item:     Option<usize>,
    dropdown_scroll_pos:        usize,
    dropdown_num_visible_items: usize,

    available_genres:    Vec<String>,
    available_languages: Vec<String>,
    available_countries: Vec<String>,
    available_actors:    Vec<Person>,
    available_directors: Vec<Person>,

    widgets:  Option<Vec<Vec<Widget>>>,
    validate: Option<BoxedFn<Self, bool>>,
    confirm:  Option<Rc<BoxedMutFn<Self, ()>>>,
}

impl AdvancedFilterPopup {
    pub fn new(filter_criteria: &[FilterCriterion]) -> Self {
        Self {
            tab: 1,
            item: 0,
            available_criteria: FilterCriterionDiscriminants::iter()
                .filter(|x| {
                    !filter_criteria
                        .iter()
                        .any(|y| FilterCriterionDiscriminants::from(y) == *x)
                })
                .collect_vec(),
            filter_criteria: filter_criteria.to_vec(),
            dropdown_selected_item: Some(0),
            dropdown_num_visible_items: 5,
            ..Default::default()
        }
    }

    pub fn initialize(&mut self, movies: &[&Movie], persons: &FxIndexMap<u32, Person>) {
        self.available_genres = movies
            .iter()
            .flat_map(|x| x.genres.clone())
            .unique()
            .sorted()
            .collect_vec();
        self.available_languages = movies
            .iter()
            .map(|x| x.language.clone())
            .unique()
            .sorted()
            .collect_vec();
        self.available_actors = movies
            .iter()
            .flat_map(|x| x.credits.cast.iter())
            .unique_by(|x| x.id)
            .filter_map(|y| persons.get(&y.id).cloned())
            .sorted_by_key(|x| x.name.clone())
            .collect();
        self.available_directors = movies
            .iter()
            .flat_map(|x| {
                x.credits
                    .crew
                    .iter()
                    .filter(|x| x.job_or_character == "Director")
            })
            .unique_by(|x| x.id)
            .filter_map(|y| persons.get(&y.id).cloned())
            .sorted_by_key(|x| x.name.clone())
            .collect();
        self.available_countries = movies
            .iter()
            .map(|x| x.origin_country.clone())
            .unique()
            .sorted()
            .collect();
    }

    fn init_criterion_options(&mut self) {
        let Some(criterion_discriminant) = self.active_criterion.as_ref() else {
            return;
        };
        match criterion_discriminant {
            FilterCriterionDiscriminants::Title => {
                self.widgets = Some(vec![vec![Widget::new_text_input(
                    0,
                    constraint!(*=1),
                    " Filter ".into(),
                    "Search".into(),
                )]]);

                self.validate = Some(Box::new(
                    |advanced_filter_popup| match advanced_filter_popup.get_widget_at(0).unwrap() {
                        Widget::TextInput { text_input, .. } => !text_input.is_empty(),
                        _ => unreachable!(),
                    },
                ));

                self.confirm = Some(Rc::new(Box::new(|advanced_filter_popup| {
                    let title = match advanced_filter_popup.get_widget_at_mut(0).unwrap() {
                        Widget::TextInput { text_input, .. } =>
                            text_input.lines()[0].trim().to_string(),
                        _ => unreachable!(),
                    };
                    if !title.is_empty() {
                        advanced_filter_popup
                            .filter_criteria
                            .push(FilterCriterion::Title(title.to_string(), true));
                        advanced_filter_popup.recalculate_available_criteria();

                        advanced_filter_popup.tab = 1;
                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.dropdown_selected_item = Some(0);
                        advanced_filter_popup.active_criterion = None;
                    }
                })));
            }
            FilterCriterionDiscriminants::Director => {
                self.widgets = Some(vec![vec![
                    Widget::new_normal_dropdown(
                        0,
                        constraint!(==19),
                        vec!["Directed by".into(), "Not directed by".into()],
                        2,
                        "".into(),
                        None,
                    ),
                    Widget::new_normal_dropdown(
                        1,
                        constraint!(*=1),
                        self.available_directors
                            .iter()
                            .map(|x| x.name.clone())
                            .collect(),
                        5,
                        "--Directors--".into(),
                        Some("Search".into()),
                    ),
                ]]);

                self.validate = Some(Box::new(
                    |advanced_filter_popup| match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::Dropdown { selected_items, .. } => !selected_items.is_empty(),
                        _ => unreachable!(),
                    },
                ));

                self.confirm = Some(Rc::new(Box::new(|advanced_filter_popup| {
                    let (director, inverted) = (
                        match advanced_filter_popup.get_widget_at(1).unwrap() {
                            Widget::Dropdown {
                                filtered_items,
                                current_selected,
                                ..
                            } =>
                                advanced_filter_popup.available_directors
                                    [filtered_items[*current_selected]]
                                    .id,
                            _ => unreachable!(),
                        },
                        match advanced_filter_popup.get_widget_at(0).unwrap() {
                            Widget::Dropdown {
                                current_selected, ..
                            } => *current_selected == 1,
                            _ => unreachable!(),
                        },
                    );
                    advanced_filter_popup
                        .filter_criteria
                        .push(FilterCriterion::Director(director, inverted));

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })));
            }
            FilterCriterionDiscriminants::Actors | FilterCriterionDiscriminants::Genres => {
                self.widgets = Some(vec![
                    vec![
                        Widget::new_static_text("Contains".into(), Some(constraint!(==15))),
                        Widget::new_normal_dropdown(
                            0,
                            constraint!(==10),
                            vec!["Any of".into(), "All of".into()],
                            2,
                            "".into(),
                            None,
                        ),
                        Widget::new_multiple_choice_dropdown(
                            1,
                            constraint!(*=1),
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                self.available_actors
                                    .iter()
                                    .map(|x| x.name.clone())
                                    .collect()
                            } else {
                                self.available_genres.clone()
                            },
                            5,
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                "--Actors--".into()
                            } else {
                                "--Genres--".into()
                            },
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                Some("Search".into())
                            } else {
                                None
                            },
                        ),
                    ],
                    vec![
                        Widget::new_static_text("Doesn't Contain".into(), None),
                        Widget::new_normal_dropdown(
                            2,
                            constraint!(==10),
                            vec!["Any of".into(), "All of".into()],
                            2,
                            "".into(),
                            None,
                        ),
                        Widget::new_multiple_choice_dropdown(
                            3,
                            constraint!(*=1),
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                self.available_actors
                                    .iter()
                                    .map(|x| x.name.clone())
                                    .collect()
                            } else {
                                self.available_genres.clone()
                            },
                            5,
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                "--Actors--".into()
                            } else {
                                "--Genres--".into()
                            },
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                Some("Search".into())
                            } else {
                                None
                            },
                        ),
                    ],
                ]);

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    let positive_empty = match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::Dropdown { selected_items, .. } => selected_items.is_empty(),
                        _ => unreachable!(),
                    };
                    let negative_empty = match advanced_filter_popup.get_widget_at(3).unwrap() {
                        Widget::Dropdown { selected_items, .. } => selected_items.is_empty(),
                        _ => unreachable!(),
                    };

                    !(positive_empty && negative_empty)
                }));

                let criterion_discriminant = *criterion_discriminant;
                self.confirm = Some(Rc::new(Box::new(move |advanced_filter_popup| {
                    let (positive, positive_contains_all) = (
                        match advanced_filter_popup.get_widget_at_mut(1).unwrap() {
                            Widget::Dropdown { selected_items, .. } =>
                                selected_items.drain(..).collect_vec(),
                            _ => unreachable!(),
                        },
                        match advanced_filter_popup.get_widget_at_mut(0).unwrap() {
                            Widget::Dropdown {
                                current_selected, ..
                            } => *current_selected == 1,
                            _ => unreachable!(),
                        },
                    );
                    let (negative, negative_contains_all) = (
                        match advanced_filter_popup.get_widget_at_mut(3).unwrap() {
                            Widget::Dropdown { selected_items, .. } =>
                                selected_items.drain(..).collect_vec(),
                            _ => unreachable!(),
                        },
                        match advanced_filter_popup.get_widget_at_mut(2).unwrap() {
                            Widget::Dropdown {
                                current_selected, ..
                            } => *current_selected == 1,
                            _ => unreachable!(),
                        },
                    );
                    if !positive.is_empty() {
                        advanced_filter_popup.filter_criteria.push(
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                FilterCriterion::Actors(
                                    positive
                                        .iter()
                                        .map(|x| advanced_filter_popup.available_actors[*x].id)
                                        .collect(),
                                    positive_contains_all,
                                    false,
                                )
                            } else {
                                FilterCriterion::Genres(
                                    positive
                                        .iter()
                                        .map(|x| advanced_filter_popup.available_genres[*x].clone())
                                        .collect(),
                                    positive_contains_all,
                                    false,
                                )
                            },
                        );
                    }
                    if !negative.is_empty() {
                        advanced_filter_popup.filter_criteria.push(
                            if matches!(
                                criterion_discriminant,
                                FilterCriterionDiscriminants::Actors
                            ) {
                                FilterCriterion::Actors(
                                    negative
                                        .iter()
                                        .map(|x| advanced_filter_popup.available_actors[*x].id)
                                        .collect(),
                                    negative_contains_all,
                                    true,
                                )
                            } else {
                                FilterCriterion::Genres(
                                    negative
                                        .iter()
                                        .map(|x| advanced_filter_popup.available_genres[*x].clone())
                                        .collect(),
                                    negative_contains_all,
                                    true,
                                )
                            },
                        );
                    }

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })));
            }
            FilterCriterionDiscriminants::Certification => {
                self.widgets = Some(vec![vec![
                    Widget::new_normal_dropdown(
                        0,
                        constraint!(==17),
                        vec!["Certified".into(), "Not certified".into()],
                        2,
                        "".into(),
                        None,
                    ),
                    Widget::new_multiple_choice_dropdown(
                        1,
                        constraint!(==22),
                        vec![
                            "NR".into(),
                            "G".into(),
                            "PG".into(),
                            "PG-13".into(),
                            "R".into(),
                        ],
                        5,
                        "--Certifications--".into(),
                        None,
                    ),
                ]]);

                self.validate = Some(Box::new(
                    |advanced_filter_popup| match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::Dropdown { selected_items, .. } => !selected_items.is_empty(),
                        _ => unreachable!(),
                    },
                ));

                self.confirm = Some(Rc::new(Box::new(|advanced_filter_popup| {
                    let (certificaions, inverted) = (
                        match advanced_filter_popup.get_widget_at(1).unwrap() {
                            Widget::Dropdown {
                                items,
                                selected_items,
                                ..
                            } => selected_items.iter().map(|x| items[*x].clone()).collect(),
                            _ => unreachable!(),
                        },
                        match advanced_filter_popup.get_widget_at_mut(0).unwrap() {
                            Widget::Dropdown {
                                current_selected, ..
                            } => *current_selected == 1,
                            _ => unreachable!(),
                        },
                    );
                    advanced_filter_popup
                        .filter_criteria
                        .push(FilterCriterion::Certification(certificaions, inverted));

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })));
            }
            FilterCriterionDiscriminants::Released
            | FilterCriterionDiscriminants::FirstWatched
            | FilterCriterionDiscriminants::LastWatched => {
                self.widgets = Some(vec![vec![
                    Widget::new_static_text(<&str>::from(criterion_discriminant).to_string(), None),
                    Widget::new_normal_dropdown(
                        0,
                        constraint!(==11),
                        vec![
                            "In".into(),
                            "After".into(),
                            "Before".into(),
                            "Between".into(),
                        ],
                        4,
                        "".into(),
                        None,
                    ),
                    Widget::new_number_input(1, constraint!(==9), " Year ".into(), "".into()),
                    Widget::new_static_text("and".into(), None).and_visible_if(Box::new(
                        |advanced_filter_popup: &AdvancedFilterPopup| -> bool {
                            match advanced_filter_popup.get_widget_at(0).unwrap() {
                                Widget::Dropdown {
                                    current_selected, ..
                                } => *current_selected == 3,
                                _ => unreachable!(),
                            }
                        },
                    )),
                    Widget::new_number_input(2, constraint!(==9), " Year ".into(), "".into())
                        .and_visible_if(Box::new(
                            |advanced_filter_popup: &AdvancedFilterPopup| -> bool {
                                match advanced_filter_popup.get_widget_at(0).unwrap() {
                                    Widget::Dropdown {
                                        current_selected, ..
                                    } => *current_selected == 3,
                                    _ => unreachable!(),
                                }
                            },
                        )),
                ]]);

                self.validate = Some(Box::new(|advanced_filter_popup| {
                    (match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0]
                            .parse::<usize>()
                            .map(|x| x > 1800)
                            .unwrap_or(false),
                        _ => unreachable!(),
                    }) && (match advanced_filter_popup.get_widget_at(0).unwrap() {
                        Widget::Dropdown {
                            current_selected, ..
                        } => *current_selected != 3,
                        _ => unreachable!(),
                    } || match advanced_filter_popup.get_widget_at(2).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0]
                            .parse::<usize>()
                            .map(|x| x > 1800)
                            .unwrap_or(false),
                        _ => unreachable!(),
                    })
                }));

                let criterion_discriminant = *criterion_discriminant;
                self.confirm = Some(Rc::new(Box::new(move |advanced_filter_popup| {
                    let ordering = match advanced_filter_popup.get_widget_at(0).unwrap() {
                        Widget::Dropdown {
                            items,
                            filtered_items,
                            current_selected,
                            ..
                        } => items[filtered_items[*current_selected]].as_str(),
                        _ => unreachable!(),
                    };
                    let lower_bound = match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0].clone(),
                        _ => unreachable!(),
                    };
                    let upper_bound = match advanced_filter_popup.get_widget_at(2).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0].clone(),
                        _ => unreachable!(),
                    };
                    let (lower_bound, upper_bound, inverted) = match ordering {
                        "In" => {
                            let input0 = lower_bound.parse().unwrap_or(u32::MIN);

                            (input0, input0, false)
                        }
                        "After" => {
                            let input0 = lower_bound.parse().unwrap_or(u32::MIN);

                            (input0, u32::MAX - 1, false)
                        }
                        "Before" => {
                            let input0 = lower_bound.parse().unwrap_or(u32::MIN);

                            (input0, u32::MAX - 1, true)
                        }
                        "Between" => {
                            let input0 = lower_bound.parse().unwrap_or(1);
                            let input1 = upper_bound.parse().unwrap_or(u32::MAX - 1);

                            (input0, input1, false)
                        }
                        _ => unreachable!(),
                    };
                    advanced_filter_popup
                        .filter_criteria
                        .push(match criterion_discriminant {
                            FilterCriterionDiscriminants::Released =>
                                FilterCriterion::Released(lower_bound, upper_bound, inverted),
                            FilterCriterionDiscriminants::FirstWatched =>
                                FilterCriterion::FirstWatched(lower_bound, upper_bound, inverted),
                            FilterCriterionDiscriminants::LastWatched =>
                                FilterCriterion::LastWatched(lower_bound, upper_bound, inverted),
                            _ => unreachable!(),
                        });

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })))
            }
            FilterCriterionDiscriminants::Rating | FilterCriterionDiscriminants::UserRating => {
                self.widgets = Some(vec![vec![
                    Widget::new_static_text(<&str>::from(criterion_discriminant).to_string(), None),
                    Widget::new_normal_dropdown(
                        0,
                        constraint!(==6),
                        vec!["=".into(), ">=".into(), ">".into(), "<=".into(), "<".into()],
                        4,
                        "".into(),
                        None,
                    ),
                    Widget::new_rating_input(1, constraint!(==10), " Rating ".into(), "".into()),
                ]]);

                self.validate = Some(Box::new(
                    |advanced_filter_popup| match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0]
                            .parse::<f64>()
                            .map(|x| x <= 10.0)
                            .unwrap_or(false),
                        _ => unreachable!(),
                    },
                ));

                let criterion_discriminant = *criterion_discriminant;
                self.confirm = Some(Rc::new(Box::new(move |advanced_filter_popup| {
                    let ordering = match advanced_filter_popup.get_widget_at(0).unwrap() {
                        Widget::Dropdown {
                            items,
                            filtered_items,
                            current_selected,
                            ..
                        } => items[filtered_items[*current_selected]].clone(),
                        _ => unreachable!(),
                    };
                    let line = match advanced_filter_popup.get_widget_at(1).unwrap() {
                        Widget::TextInput { text_input, .. } => text_input.lines()[0].clone(),
                        _ => unreachable!(),
                    };
                    let (rating, ordering, inverted) = match ordering.as_str() {
                        "<" => {
                            let rating = line.parse().unwrap();

                            (rating, Ordering::Less, false)
                        }
                        "<=" => {
                            let rating = line.parse().unwrap();

                            (rating, Ordering::Greater, true)
                        }
                        ">" => {
                            let rating = line.parse().unwrap();

                            (rating, Ordering::Greater, false)
                        }
                        ">=" => {
                            let rating = line.parse().unwrap();

                            (rating, Ordering::Less, true)
                        }
                        "=" => {
                            let rating = line.parse().unwrap();

                            (rating, Ordering::Equal, false)
                        }
                        _ => unreachable!(),
                    };
                    advanced_filter_popup
                        .filter_criteria
                        .push(match criterion_discriminant {
                            FilterCriterionDiscriminants::Rating =>
                                FilterCriterion::Rating(rating, ordering, inverted),
                            FilterCriterionDiscriminants::UserRating =>
                                FilterCriterion::UserRating(rating, ordering, inverted),
                            _ => unreachable!(),
                        });

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })));
            }
            FilterCriterionDiscriminants::Language | FilterCriterionDiscriminants::Country => {
                self.widgets = Some(vec![vec![
                    Widget::new_normal_dropdown(
                        0,
                        if matches!(
                            criterion_discriminant,
                            FilterCriterionDiscriminants::Language
                        ) {
                            constraint!(==10)
                        } else {
                            constraint!(==12)
                        },
                        if matches!(
                            criterion_discriminant,
                            FilterCriterionDiscriminants::Language
                        ) {
                            vec!["In".into(), "Not in".into()]
                        } else {
                            vec!["From".into(), "Not from".into()]
                        },
                        2,
                        "".into(),
                        None,
                    ),
                    Widget::new_normal_dropdown(
                        1,
                        constraint!(==10),
                        if matches!(
                            criterion_discriminant,
                            FilterCriterionDiscriminants::Language
                        ) {
                            self.available_languages.clone()
                        } else {
                            self.available_countries.clone()
                        },
                        5,
                        "".into(),
                        None,
                    ),
                ]]);

                self.validate = Some(Box::new(|_| true));

                let criterion_discriminant = *criterion_discriminant;
                self.confirm = Some(Rc::new(Box::new(move |advanced_filter_popup| {
                    let (values, inverted) = (
                        match advanced_filter_popup.get_widget_at(1).unwrap() {
                            Widget::Dropdown {
                                filtered_items,
                                current_selected,
                                ..
                            } => {
                                if matches!(
                                    criterion_discriminant,
                                    FilterCriterionDiscriminants::Language
                                ) {
                                    advanced_filter_popup.available_languages
                                        [filtered_items[*current_selected]]
                                        .clone()
                                } else {
                                    advanced_filter_popup.available_countries
                                        [filtered_items[*current_selected]]
                                        .clone()
                                }
                            }
                            _ => unreachable!(),
                        },
                        match advanced_filter_popup.get_widget_at(0).unwrap() {
                            Widget::Dropdown {
                                current_selected, ..
                            } => *current_selected == 1,
                            _ => unreachable!(),
                        },
                    );
                    advanced_filter_popup.filter_criteria.push(
                        if matches!(
                            criterion_discriminant,
                            FilterCriterionDiscriminants::Language
                        ) {
                            FilterCriterion::Language(values, inverted)
                        } else {
                            FilterCriterion::Country(values, inverted)
                        },
                    );

                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = Some(0);
                    advanced_filter_popup.active_criterion = None;
                })));
            }
        }
    }

    fn get_widget_at(&self, item: usize) -> Option<&Widget> {
        self.widgets
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|x| {
                x.iter().find(|x| match x {
                    Widget::StaticText { .. } => false,
                    Widget::Dropdown {
                        item: widget_item, ..
                    }
                    | Widget::TextInput {
                        item: widget_item, ..
                    } => *widget_item == item,
                })
            })
            .nth(0)
    }

    fn get_widget_at_mut(&mut self, item: usize) -> Option<&mut Widget> {
        self.widgets
            .as_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|x| {
                x.iter_mut().find(|x| match x {
                    Widget::StaticText { .. } => false,
                    Widget::Dropdown {
                        item: widget_item, ..
                    }
                    | Widget::TextInput {
                        item: widget_item, ..
                    } => *widget_item == item,
                })
            })
            .nth(0)
    }

    fn recalculate_available_criteria(&mut self) {
        self.available_criteria = FilterCriterionDiscriminants::iter()
            .filter(|x| {
                !self
                    .filter_criteria
                    .iter()
                    .any(|y| FilterCriterionDiscriminants::from(y) == *x)
            })
            .collect_vec();
    }

    fn finish(app: &mut App) {
        let filter_criteria = if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
            app.drawer.active_popup.as_mut()
        {
            advanced_filter_popup
                .filter_criteria
                .drain(..)
                .collect_vec()
        } else {
            unreachable!()
        };

        if let Some(Screens::MainScreen(main_screen)) = app.drawer.current_screen.as_mut() {
            main_screen.filter_criteria = filter_criteria;

            if let Some(FilterCriterion::Title(name, filter)) =
                pop_criterion!(main_screen.filter_criteria, FilterCriterion::Title(_, _))
            {
                if !name.is_empty() {
                    main_screen.search_input = TextArea::from([&name]);
                    main_screen
                        .search_input
                        .move_cursor(ratatui_textarea::CursorMove::End);
                    main_screen.sort = crate::types::Sort::Relevance;
                }

                main_screen
                    .filter_criteria
                    .push(FilterCriterion::Title(name, filter));
            }

            main_screen.filter_sort_movies(true);
        }
    }
}

impl PopupTrait for AdvancedFilterPopup {
    fn get_state(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.tab), Some(self.item))
    }

    fn update_next_frame(&self) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, frame: &mut Frame, key_event_handler: &mut KeyEventHandler) {
        key_event_handler.clear();
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            frame.area(),
            |app, _| {
                app.drawer.close_popup();
            },
        );
        key_event_handler.bind_esc((None, None), "Close".into(), |app, _| {
            app.drawer.close_popup();
        });
        key_event_handler.bind_key((None, None), 'q', "Close".into(), |app, _| {
            app.drawer.close_popup();
        });

        let criterion_options_lines_count =
            |criterion: &FilterCriterionDiscriminants| match criterion {
                FilterCriterionDiscriminants::Actors | FilterCriterionDiscriminants::Genres => 6,
                _ => 3,
            };

        // key_event_handler.bind_enter((Some(0), None), "Edit Criterion".into(), );
        // key_event_handler.bind_key((Some(0), None), ' ', "Delete Criterion".into(), );

        key_event_handler.bind_tab((None, None), "Change focus".into(), move |app, data| {
            if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                app.drawer.active_popup.as_mut()
            {
                match data {
                    crate::key_event_handler::Data::Direction(true, _) => {
                        advanced_filter_popup.dropdown_selected_item = None;

                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.tab += 1;
                        if advanced_filter_popup.tab > 3 {
                            advanced_filter_popup.tab = 0;
                        }
                        if advanced_filter_popup.tab == 2
                            && advanced_filter_popup.active_criterion.is_none()
                        {
                            advanced_filter_popup.tab = 3;
                        } else if advanced_filter_popup.tab == 0
                            && advanced_filter_popup.filter_criteria.is_empty()
                        {
                            advanced_filter_popup.tab = 1;
                        }
                    }
                    crate::key_event_handler::Data::Direction(false, _) => {
                        advanced_filter_popup.dropdown_selected_item = None;

                        advanced_filter_popup.item = 0;
                        advanced_filter_popup.tab =
                            advanced_filter_popup.tab.checked_sub(1).unwrap_or(3);

                        if advanced_filter_popup.tab == 2
                            && advanced_filter_popup.active_criterion.is_none()
                        {
                            advanced_filter_popup.tab = 1;
                        } else if advanced_filter_popup.tab == 0
                            && advanced_filter_popup.filter_criteria.is_empty()
                        {
                            advanced_filter_popup.tab = 3;
                        }
                    }
                    _ => {}
                }
            }
        });

        let applied_height = 1;
        let options_height = if let Some(active) = self.active_criterion.as_ref() {
            criterion_options_lines_count(active) + 1
        } else {
            0
        };
        let constraints = constraints![==(applied_height + 2), ==3, ==(options_height + 2), ==2];
        let popup_height = applied_height + 2 + 3 + options_height + 2 + 2 + 2;
        let popup_area = widgets::window(
            frame,
            helpers::centered_area(popup_height, 55, frame.area()),
            " Advanced Filter ",
            true,
        );
        key_event_handler.bind_mouse_button_down(
            ratatui::crossterm::event::MouseButton::Left,
            popup_area.outer(Margin::new(1, 1)),
            |_, _| {},
        );
        let [applied_area, dropdown_area, options_area, _] = Layout::vertical(constraints)
            .areas(helpers::add_padding(popup_area, Padding::horizontal(1)));

        {
            // let this_tab = 0;
            // let tab_selected = self.tab == this_tab;

            let applied_block = Block::new()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_set(border::PROPORTIONAL_WIDE)
                .fg(tailwind::SLATE.c900);
            let inner_applied_block = applied_block.inner(applied_area);
            frame.render_widget(Block::new().bg(tailwind::SLATE.c900), inner_applied_block);
            frame.render_widget(&applied_block, applied_area);

            if self.filter_criteria.is_empty() {
                frame.render_widget(
                    line!("None").fg(tailwind::WHITE).centered(),
                    inner_applied_block,
                );
            } else {
                frame.render_widget(
                    format!("{:?}", self.filter_criteria).fg(tailwind::WHITE),
                    inner_applied_block,
                );
            }
        }

        {
            let this_tab = 3;
            let tab_selected = self.tab == this_tab;

            key_event_handler.bind_enter(
                (Some(this_tab), Some(0)),
                "Confirm".into(),
                move |app, _| {
                    Self::finish(app);

                    app.drawer.close_popup();
                },
            );
            key_event_handler.bind_enter(
                (Some(this_tab), Some(1)),
                "Cancel".into(),
                move |app, _| {
                    app.drawer.close_popup();
                },
            );

            key_event_handler.bind_horizontal(
                (Some(this_tab), None),
                "Navigate".into(),
                move |app, data| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(true, _) => {
                                advanced_filter_popup.item += 1;
                                if advanced_filter_popup.item > 1 {
                                    advanced_filter_popup.item = 1;
                                }
                            }
                            crate::key_event_handler::Data::Direction(false, _) => {
                                advanced_filter_popup.item =
                                    advanced_filter_popup.item.saturating_sub(1);
                            }
                            _ => {}
                        }
                    }
                },
            );

            let actions_mouse_areas = widgets::actions(
                [
                    Action::new(
                        " Confirm ",
                        ActionType::Default,
                        tab_selected && self.item == 0,
                        true,
                    ),
                    Action::new(
                        " Cancel ",
                        ActionType::Critical,
                        tab_selected && self.item == 1,
                        true,
                    ),
                ],
                HorizontalAlignment::Center,
                true,
                1,
                helpers::add_padding(popup_area, Padding::right(1)),
                frame,
            );
            for (i, mouse_area) in actions_mouse_areas.into_iter().enumerate() {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if i == 0 {
                            Self::finish(app);
                        }
                        app.drawer.close_popup();
                    },
                );
            }
        }

        if self.active_criterion.is_some() {
            let this_tab = 2;
            let tab_selected = self.tab == this_tab;

            key_event_handler.bind_esc((Some(this_tab), None), "Back".into(), |app, _| {
                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                    app.drawer.active_popup.as_mut()
                {
                    advanced_filter_popup.tab = 1;
                    advanced_filter_popup.item = 0;
                    advanced_filter_popup.dropdown_selected_item = None;
                }
            });

            let valid = self
                .validate
                .as_ref()
                .map(|validate| validate(self))
                .unwrap_or(false);
            let widget_visible = self
                .widgets
                .as_ref()
                .unwrap()
                .iter()
                .map(|x| {
                    x.iter()
                        .map(|x| match x {
                            Widget::Dropdown { visible_if, .. }
                            | Widget::TextInput { visible_if, .. }
                            | Widget::StaticText { visible_if, .. } =>
                                visible_if.as_ref().map(|x| x(self)).unwrap_or(true),
                        })
                        .collect_vec()
                })
                .filter(|x| !x.is_empty())
                .collect_vec();
            let widgets = self
                .widgets
                .as_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
                .map(|(i, x)| {
                    x.iter_mut()
                        .enumerate()
                        .filter_map(|(j, y)| widget_visible[i][j].then_some(y))
                        .collect_vec()
                })
                .filter(|x| !x.is_empty())
                .collect_vec();
            let last_item = (|| -> usize {
                for (i, row) in widgets.iter().enumerate().rev() {
                    for (j, widget) in row.iter().enumerate().rev() {
                        match widget {
                            Widget::Dropdown { item, .. } | Widget::TextInput { item, .. } =>
                                if widget_visible[i][j] {
                                    return *item;
                                } else {
                                    continue;
                                },
                            Widget::StaticText { .. } => continue,
                        }
                    }
                }
                usize::MAX
            })();

            key_event_handler.bind_horizontal(
                (Some(this_tab), None),
                "Navigate".into(),
                move |app, data| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        match data {
                            crate::key_event_handler::Data::Direction(false, _) => {
                                advanced_filter_popup.item =
                                    advanced_filter_popup.item.saturating_sub(1);
                            }
                            crate::key_event_handler::Data::Direction(true, _)
                                if advanced_filter_popup.item < last_item =>
                                advanced_filter_popup.item += 1,
                            _ => (),
                        }
                    }
                },
            );

            let options_block = Block::new()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_set(border::PROPORTIONAL_TALL)
                .fg(tailwind::SKY.c950)
                .bg(tailwind::SKY.c950);
            frame.render_widget(&options_block, options_area);
            let inner_area = options_block.inner(options_area);

            // let valid = self.validate.as_ref().unwrap()(self);
            let actions_mouse_areas = widgets::actions(
                [
                    Action::new("  ", ActionType::Normal, true, valid),
                    Action::new("  ", ActionType::Critical, true, true),
                ],
                HorizontalAlignment::Right,
                true,
                1,
                helpers::add_padding(inner_area, Padding::right(2)),
                frame,
            );
            for (i, mouse_area) in actions_mouse_areas
                .into_iter()
                .enumerate()
                .dropping(if valid { 0 } else { 1 })
            {
                key_event_handler.bind_mouse_button_down(
                    ratatui::crossterm::event::MouseButton::Left,
                    mouse_area,
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            if i == 0 {
                                advanced_filter_popup.confirm.as_ref().unwrap().clone()(
                                    advanced_filter_popup,
                                );
                            }

                            advanced_filter_popup.tab = 1;
                            advanced_filter_popup.item = 0;
                            advanced_filter_popup.dropdown_selected_item = Some(0);
                            advanced_filter_popup.active_criterion = None;
                        }
                    },
                );
            }

            let areas = Layout::vertical(vec![constraint!(==3); widgets.len()])
                .split(helpers::add_padding(inner_area, Padding::new(2, 2, 0, 1)));
            for (row_area, (i, row)) in areas
                .iter()
                .rev()
                .zip_eq(widgets.into_iter().enumerate().rev())
            {
                let row_last_item = row
                    .iter()
                    .enumerate()
                    .filter_map(|(j, x)| match x {
                        Widget::Dropdown { item, .. } | Widget::TextInput { item, .. } =>
                            if widget_visible[i][j] {
                                Some(*item)
                            } else {
                                None
                            },
                        Widget::StaticText { .. } => None,
                    })
                    .next_back()
                    .unwrap_or(usize::MAX);
                let areas = Layout::horizontal(
                    row.iter()
                        .map(|x| x.get_constraint())
                        .intersperse(constraint!(==1))
                        .collect_vec(),
                )
                .split(*row_area);
                for (area, (j, widget)) in areas.iter().step_by(2).zip(row.into_iter().enumerate())
                {
                    if widget_visible[i][j] {
                        widget.render(frame, key_event_handler, *area, self.item, tab_selected);
                        widget.bind(
                            key_event_handler,
                            valid,
                            *area,
                            self.item,
                            tab_selected,
                            self.item == row_last_item || self.item == last_item,
                        );
                    }
                }
            }
        }

        {
            let this_tab = 1;
            let tab_selected = self.tab == this_tab;

            if self.dropdown_selected_item.is_some() {
                key_event_handler.bind_enter(
                    (Some(this_tab), Some(0)),
                    "Select".into(),
                    move |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.item = 0;
                            advanced_filter_popup.tab = 2;

                            let selected =
                                advanced_filter_popup.dropdown_selected_item.take().unwrap();
                            advanced_filter_popup.active_criterion =
                                Some(advanced_filter_popup.available_criteria[selected]);
                            advanced_filter_popup.init_criterion_options();
                        }
                    },
                );
                key_event_handler.bind_vertical(
                    (Some(this_tab), Some(0)),
                    "Choose".into(),
                    move |app, data| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            match data {
                                crate::key_event_handler::Data::Direction(true, _) => {
                                    advanced_filter_popup.dropdown_selected_item =
                                        advanced_filter_popup.dropdown_selected_item.map(|x| {
                                            if x < advanced_filter_popup.available_criteria.len()
                                                - 1
                                            {
                                                if (x + 1).saturating_sub(
                                                    advanced_filter_popup.dropdown_scroll_pos,
                                                ) >= advanced_filter_popup
                                                    .dropdown_num_visible_items
                                                {
                                                    advanced_filter_popup.dropdown_scroll_pos =
                                                        (x + 1).saturating_sub(
                                                            advanced_filter_popup
                                                                .dropdown_num_visible_items
                                                                - 1,
                                                        )
                                                }

                                                x + 1
                                            } else {
                                                x
                                            }
                                        });
                                }
                                crate::key_event_handler::Data::Direction(false, _) => {
                                    advanced_filter_popup.dropdown_selected_item =
                                        advanced_filter_popup
                                            .dropdown_selected_item
                                            .map(|x| x.saturating_sub(1))
                                            .inspect(|x| {
                                                if x < &advanced_filter_popup.dropdown_scroll_pos {
                                                    advanced_filter_popup.dropdown_scroll_pos -= 1;
                                                }
                                            });
                                }
                                _ => {}
                            }
                        }
                    },
                );
            } else {
                key_event_handler.bind_enter(
                    (Some(this_tab), Some(0)),
                    "Open Dropdown".into(),
                    |app, _| {
                        if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                            app.drawer.active_popup.as_mut()
                        {
                            advanced_filter_popup.dropdown_selected_item = Some(
                                if let Some(x) = advanced_filter_popup.active_criterion.as_ref() {
                                    *x as usize
                                } else {
                                    0
                                },
                            );
                            advanced_filter_popup.dropdown_num_visible_items = 5;
                            advanced_filter_popup.dropdown_scroll_pos = advanced_filter_popup
                                .dropdown_selected_item
                                .as_ref()
                                .unwrap()
                                .saturating_sub(
                                    advanced_filter_popup.dropdown_num_visible_items - 1,
                                );
                        }
                    },
                );
            }

            if self.dropdown_selected_item.is_some() {
                key_event_handler.bind_esc((Some(this_tab), Some(0)), "Close".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        _ = advanced_filter_popup.dropdown_selected_item.take();
                    }
                });
            } else if self.active_criterion.is_some() {
                key_event_handler.bind_esc((Some(this_tab), Some(0)), "Clear".into(), |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        advanced_filter_popup.active_criterion = None;
                    }
                });
            }

            let [message_area, _, dropdown_area] = horizontal![==20, ==1, ==20]
                .flex(ratatui::layout::Flex::Center)
                .areas(dropdown_area);

            frame.render_widget(
                "Add a new Criterion:",
                helpers::resize_area_centered(message_area, Offset::new(0, -2)),
            );

            let selected = self.item == 0;
            widgets::dropdown(
                true,
                tab_selected && selected,
                frame,
                dropdown_area,
                helpers::ellipsize_string(
                    self.active_criterion
                        .as_ref()
                        .map(|x| x.into())
                        .unwrap_or("None"),
                    dropdown_area.width as usize - 4,
                ),
            );
            key_event_handler.bind_mouse_button_down(
                ratatui::crossterm::event::MouseButton::Left,
                dropdown_area,
                move |app, _| {
                    if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                        app.drawer.active_popup.as_mut()
                    {
                        let selected = advanced_filter_popup.tab == this_tab
                            && advanced_filter_popup.item == 0;
                        advanced_filter_popup.tab = this_tab;
                        advanced_filter_popup.item = 0;

                        if selected {
                            advanced_filter_popup.dropdown_selected_item = advanced_filter_popup
                                .dropdown_selected_item
                                .map(|_| None)
                                .unwrap_or(
                                    advanced_filter_popup
                                        .active_criterion
                                        .as_ref()
                                        .map(|x| *x as usize)
                                        .or(Some(0)),
                                );
                        } else {
                            advanced_filter_popup.dropdown_selected_item = advanced_filter_popup
                                .active_criterion
                                .as_ref()
                                .map(|x| *x as usize)
                                .or(Some(0));
                        }
                        advanced_filter_popup.dropdown_num_visible_items = 5;
                        advanced_filter_popup.dropdown_scroll_pos = advanced_filter_popup
                            .dropdown_selected_item
                            .as_ref()
                            .map(|x| {
                                x.saturating_sub(
                                    advanced_filter_popup.dropdown_num_visible_items - 1,
                                )
                            })
                            .unwrap_or(0);
                    }
                },
            );

            if tab_selected && selected {
                self.dropdown_selected_item = self.dropdown_selected_item.map(|x| {
                    if x >= self.available_criteria.len() {
                        self.dropdown_scroll_pos -= 1;
                        self.available_criteria.len() - 1
                    } else {
                        x
                    }
                });

                if let Some(index) = self.dropdown_selected_item.as_ref() {
                    let (mut mouse_area, len) = ContextMenu {
                        model: self
                            .available_criteria
                            .iter()
                            .map(|x| {
                                helpers::ellipsize_string(
                                    x.into(),
                                    dropdown_area.width as usize - 2,
                                )
                            })
                            .collect_vec(),
                        selected_index: *index,
                        scroll_pos: self.dropdown_scroll_pos,
                        num_visible_items: self.dropdown_num_visible_items,
                        ..Default::default()
                    }
                    .render_dropdown(dropdown_area, frame, key_event_handler)
                    .into_iter()
                    .nth(0)
                    .unwrap()
                    .1;
                    for i in 0..len {
                        key_event_handler.bind_mouse_button_down(
                            ratatui::crossterm::event::MouseButton::Left,
                            mouse_area,
                            move |app, _| {
                                if let Some(Popups::AdvancedFilter(advanced_filter_popup)) =
                                    app.drawer.active_popup.as_mut()
                                {
                                    advanced_filter_popup.tab = 2;
                                    advanced_filter_popup.item = 0;

                                    _ = advanced_filter_popup.dropdown_selected_item.take();
                                    advanced_filter_popup.active_criterion = Some(
                                        advanced_filter_popup.available_criteria
                                            [i + advanced_filter_popup.dropdown_scroll_pos],
                                    );
                                    advanced_filter_popup.init_criterion_options();
                                }
                            },
                        );
                        mouse_area = mouse_area.offset(Offset { x: 0, y: 1 });
                    }
                }
            }
        }
    }
}
