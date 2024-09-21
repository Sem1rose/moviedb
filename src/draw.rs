use crate::{
    app::{App, Movie},
    custom_widgets::*,
};
// use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
// use image::{DynamicImage, ImageReader};
use rand::prelude::*;
use ratatui::{
    layout::*,
    prelude::*,
    style::*,
    text::Line,
    widgets::{block::*, *},
    Frame,
};
// use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::{fs, path::PathBuf, process::Command};
use style::palette::tailwind;

pub fn ui(frame: &mut Frame, app: &mut App) {
    render_movies_list(frame, app);
}

fn render_movies_list(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Clear, frame.area());

    let frame_size = frame.area();

    let num_movies = ((frame_size.height - 3) as f32 / 8.0).floor() as usize;
    let status_height = (((frame_size.height - 3) % 8) % num_movies as u16) + 2;
    let vert_lay = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(status_height),
            Constraint::Min(12),
            Constraint::Length(1),
        ],
    )
    .split(frame_size);

    let horiz_lay = Layout::new(
        Direction::Horizontal,
        [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)],
    )
    .split(vert_lay[1]);

    let movies_lay =
        Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies]).split(horiz_lay[1]);
    app.set_num_movies_visible(num_movies as u32);

    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
    frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

    frame.render_widget(Block::new().bg(tailwind::SLATE.c800), horiz_lay[0]);

    // let _ = Command::new("kitten").args(["icat", "--clear"]).status();
    for (i, x) in movies_lay.iter().enumerate() {
        if (i + app.movies_list_screen_options.scroll_pos as usize) < app.movies.len() {
            let movie = &app.movies[app.movies_list_screen_options.scroll_pos as usize + i];
            draw_movie_widget(
                i + 1,
                frame,
                *x,
                movie,
                get_movie_poster(movie),
                app.movies_list_screen_options.selected as usize == i,
                (app.movies_list_screen_options.scroll_pos as usize + i) % 2 == 0,
            );
        } else {
            frame.render_widget(
                Block::new().bg(if i % 2 == 0 {
                    tailwind::NEUTRAL.c900
                } else {
                    tailwind::STONE.c900
                }),
                *x,
            );
        }
    }

    // ⥘⥠🢑⥙⥡🢓 ｜¦│❚ ▉🮋🬸🬶🬗 🢕🢗
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("🢑"))
        .end_symbol(Some("🢓"))
        .track_symbol(Some("│"))
        .thumb_symbol("▉")
        .thumb_style(Style::new().fg(Color::White))
        .track_style(Style::new().fg(Color::DarkGray).bold())
        .begin_style(Style::new().fg(Color::DarkGray).bold())
        .end_style(Style::new().fg(Color::DarkGray).bold());

    let mut scrollbar_state = ScrollbarState::new(app.movies.len() - num_movies)
        .position(app.movies_list_screen_options.scroll_pos as usize);

    frame.render_stateful_widget(scrollbar, horiz_lay[1], &mut scrollbar_state);
}

fn draw_movie_widget(
    id: usize,
    frame: &mut Frame,
    area: Rect,
    movie: &Movie,
    poster: PathBuf,
    selected: bool,
    alt: bool,
) {
    let (background, text, border, selection_highlight) = if selected {
        (
            Color::Rgb(16, 48, 16),
            Color::Rgb(48, 144, 48),
            Color::Rgb(64, 192, 64),
            Color::Rgb(32, 96, 32),
        )
    } else if alt {
        (
            Color::Rgb(48, 16, 16),
            Color::Rgb(144, 48, 48),
            Color::Rgb(192, 64, 64),
            Color::Rgb(96, 32, 32),
        )
    } else {
        (
            Color::Rgb(16, 24, 48),
            Color::Rgb(48, 72, 144),
            Color::Rgb(64, 96, 192),
            Color::Rgb(32, 48, 96),
        )
    };

    let vert_lay = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(area);
    let movie_width = (vert_lay[1].height as f32 / 1.5).ceil() as u16 * 2;
    print!("{}", movie_width);
    let horiz_lay = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Length(2),
            Constraint::Length(movie_width),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ],
    )
    .split(vert_lay[1]);

    let block = Block::new().bg(background).fg(text);
    frame.render_widget(&block, area);

    //▐🭻▔
    if selected {
        frame.render_widget(
            Paragraph::new("▐\n".repeat(horiz_lay[0].height as usize)).fg(selection_highlight),
            horiz_lay[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new("▔".repeat(vert_lay[0].width as usize)).fg(border),
            vert_lay[0],
        );
        frame.render_widget(
            Paragraph::new("▁".repeat(vert_lay[2].width as usize)).fg(border),
            vert_lay[2],
        );
    }

    let _ = Command::new("kitten")
        .args([
            "icat",
            "--place",
            &format!(
                "{}x{}@{}x{}",
                horiz_lay[1].width, horiz_lay[1].height, horiz_lay[1].x, horiz_lay[1].y
            ),
            "--image-id",
            &id.to_string(),
            // "--bg",
            // &format!("{}", background),
            poster.to_str().unwrap(),
        ])
        .status();
    // let _ = Command::new("chafa")
    //     .args([
    //         "--align",
    //         "top,right",
    //         "--relative",
    //         "on",
    //         "--view-size",
    //         &format!("{}x{}", movie_width, area.height),
    //         "--bg",
    //         &format!("{}", background),
    //         get_movie_poster(movie).to_str().unwrap(),
    //     ])
    //     .spawn();

    // println!("ass");
    // let mut picker = Picker::from_termios().expect("Couldn't read font size from terminal!");
    // picker.guess_protocol();
    // let mut image = picker.new_resize_protocol(poster);
    // let mut_image = StatefulImage::new(None).resize(Resize::Fit(None));
    // frame.render_stateful_widget(mut_image, horiz_lay[0], &mut image);
    // println!("dick");

    let mut name = movie.name.clone();
    if name.len() > 50 {
        name.truncate(50);
        name += "...";
    }

    let mut text = vec![
        (name.bold() + " ".not_bold() + movie.year.clone().italic().dim()),
        Line::from(movie.rating.to_string()),
    ];

    frame.render_widget(Paragraph::new(text), horiz_lay[3]);
}

fn get_movie_poster(movie: &Movie) -> PathBuf {
    let paths = fs::read_dir("./src")
        .unwrap()
        .filter_map(|x| x.ok())
        .map(|x| x.path())
        .filter(|x| x.extension().map_or(false, |x| x == "jpg"))
        .collect::<Vec<_>>();

    paths[rand::thread_rng().gen_range(0..paths.len())].clone()
    // ImageReader::open("./src/01e0e96e.jpg")
    //     .unwrap_or_else(|_| panic!("Couldn't open image at {}", "./src/01e0e96e.jpg"))
    //     .decode()
    //     .unwrap_or_else(|_| panic!("Couldn't decode image at {}", "./src/01e0e96e.jpg"))
}
