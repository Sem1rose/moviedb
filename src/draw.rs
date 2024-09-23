use crate::app::{App, Movie};
// use image::{DynamicImage, ImageReader};
use crossterm::ExecutableCommand;
use rand::prelude::*;
use ratatui::{layout::*, prelude::*, style::*, text::Line, widgets::*, Frame};
use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::stdout,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};
use style::palette::tailwind;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let now = std::time::Instant::now();
    render_movies_list(frame, app);
    frame.render_widget(
        Paragraph::new(format!("{:.1}", 1.0 / now.elapsed().as_secs_f32())),
        // Paragraph::new(format!("{}", app.clear_images)),
        frame.area(),
    );
}

fn render_movies_list(frame: &mut Frame, app: &mut App) {
    // Command::new("chafa")
    //     .args(["--clear", "./src/placeholder.png"])
    //     .status();
    // frame.render_widget(Clear, frame.area());

    // stdout().execute(crossterm::terminal::Clear(
    //     crossterm::terminal::ClearType::All,
    // ));

    let frame_area = frame.area();

    let num_movies = ((frame_area.height - 3) as f32 / 8.0).floor() as usize;
    let status_height = (((frame_area.height - 3) % 8) % num_movies as u16) + 2;
    let vert_lay = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(status_height),
            Constraint::Min(12),
            Constraint::Length(1),
        ],
    )
    .split(frame_area);

    let horiz_lay = Layout::new(
        Direction::Horizontal,
        [Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)],
    )
    .split(vert_lay[1]);

    // if app.clear_images {
    //     // (" ".repeat(area.width as usize) + "\n").repeat(area.height as usize)
    //     frame.render_widget(
    //         (" ".repeat(horiz_lay[1].width as usize) + "\n").repeat(horiz_lay[1].height as usize),
    //         horiz_lay[1],
    //     );
    //     app.update_images = true;
    // }

    frame.render_widget(Block::new().bg(tailwind::SLATE.c900), vert_lay[0]);
    frame.render_widget(Block::new().bg(tailwind::EMERALD.c950), vert_lay[2]);

    frame.render_widget(Block::new().bg(tailwind::SLATE.c800), horiz_lay[0]);

    let movies_lay =
        Layout::new(Direction::Vertical, vec![Constraint::Min(8); num_movies]).split(horiz_lay[1]);
    app.set_num_movies_visible(num_movies as u32);

    // Command::new("kitten").arg("icat").arg("--clear").status();

    // stdout().execute(crossterm::cursor::MoveTo(horiz_lay[1].x, horiz_lay[1].y));
    // print!(
    //     "{}",
    //     (" ".repeat(horiz_lay[1].width as usize)
    //         + format!("{}\n", crossterm::cursor::MoveLeft(horiz_lay[1].width)).as_str())
    //     .repeat(horiz_lay[1].height as usize)
    // );

    for (i, area) in movies_lay.iter().enumerate() {
        // {
        // (" ".repeat(area.width as usize) + "\n").repeat(area.height as usize)
        // for y in 0..frame_area.height {
        //     for x in 0..frame_area.width {
        //         if let Some(cell) = frame.buffer_mut().cell_mut(Position::new(x, y)) {
        //             cell.reset();
        //         }
        //     }
        // }
        // app.update_images = true;
        // continue;
        // }

        if (i + app.movies_list_screen_options.scroll_pos as usize) < app.movies.len() {
            draw_movie_widget(i, app, frame, *area);
        } else {
            frame.render_widget(
                Block::new().bg(if i % 2 == 0 {
                    tailwind::NEUTRAL.c900
                } else {
                    tailwind::STONE.c900
                }),
                *area,
            );
        }
    }
    if !app.clear_images {
        app.update_images = false;
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

fn draw_movie_widget(id: usize, app: &mut App, frame: &mut Frame, area: Rect) {
    let selected = app.movies_list_screen_options.selected as usize == id;
    let alt = (app.movies_list_screen_options.scroll_pos as usize + id) % 2 == 0;
    let movie = app.movies[app.movies_list_screen_options.scroll_pos as usize + id].clone();
    let movie_id = id as u32 + app.movies_list_screen_options.scroll_pos;
    // let poster = get_movie_poster(movie);

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

    // let sckt = String::from(socket);
    // let x = thread::spawn(move || {
    //     let _ = Command::new("ueberzugpp")
    //         .args(["cmd", "-s", &sckt, "-i", &id.to_string(), "-a", "remove"])
    //         .stderr(Stdio::null())
    //         .stdout(Stdio::null())
    //         .status();
    //     let x = Command::new("ueberzugpp")
    //         .args([
    //             "cmd",
    //             "-s",
    //             &sckt,
    //             "-i",
    //             &id.to_string(),
    //             "-a",
    //             "add",
    //             "-x",
    //             &x.to_string(),
    //             "-y",
    //             &y.to_string(),
    //             "--max-width",
    //             &w.to_string(),
    //             "--max-height",
    //             &h.to_string(),
    //             "-f",
    //             poster.to_str().unwrap(),
    //         ])
    //         .stderr(Stdio::null())
    //         .stdout(Stdio::null())
    //         .spawn()
    //         .unwrap()
    //         .id();
    // });

    let block = Block::new().bg(background).fg(text);
    frame.render_widget(&block, area);

    // let (h, w, x, y) = (
    //     horiz_lay[1].height,
    //     horiz_lay[1].width,
    //     horiz_lay[1].x,
    //     horiz_lay[1].y,
    // );

    if app.update_images {
        // let data_clone = Arc::clone(&app.movies_posters);
        // let mut data = data_clone.lock().unwrap();

        if app.movies_posters.contains_key(&movie_id) {
            let poster = app.movies_posters.get(&movie_id);

            let _ = stdout().execute(crossterm::cursor::MoveTo(horiz_lay[1].x, horiz_lay[1].y));
            println!("{}", poster.cloned().unwrap());
        } else {
            app.clear_images = true;
            if !app.posters_requested.iter().any(|x| *x == movie_id) {
                app.posters_requested.push(movie_id);
                request_poster_async(app, movie_id, horiz_lay[1]);
            }
        }
        // let out = String::from_utf8_lossy(
        //     &Command::new("chafa")
        //         .args([
        //             "--align",
        //             "top,right",
        //             "--relative",
        //             "on",
        //             "--view-size",
        //             &format!("{}x{}", w, h),
        //             "--polite",
        //             "true",
        //             poster.to_str().unwrap(),
        //         ])
        //         .stdout(Stdio::piped())
        //         .output()
        //         .unwrap()
        //         .stdout,
        // )
        // .to_string();
        // let out = String::from_utf8_lossy(
        //     &Command::new("kitten")
        //         .args([
        //             "icat",
        //             "--align",
        //             "right",
        //             "--place",
        //             &format!("{}x{}@0x0", w, h),
        //             "--stdin",
        //             "no",
        //             "--image-id",
        //             "1",
        //             "./src/01e0e96e.jpg",
        //         ])
        //         .stdout(Stdio::piped())
        //         .output()
        //         .unwrap()
        //         .stdout,
        // )
        // .to_string();
    }
    // //▐🭻▔
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

    let mut name = movie.name.clone();
    if name.len() > 50 {
        name.truncate(50);
        name += "...";
    }

    let mut text = vec![
        (name.bold() + " ".into() + movie.year.clone().italic().dim()),
        movie.rating.to_string().into(),
    ];

    frame.render_widget(Paragraph::new(text), horiz_lay[3]);
}

fn request_poster_async(app: &mut App, id: u32, area: Rect) {
    // TODO: wrap with a thread.

    let path = get_movie_poster(&app.movies[id as usize])
        .to_str()
        .unwrap()
        .to_string();

    let data = String::from_utf8_lossy(
        &Command::new("chafa")
            .args([
                "--align",
                "top,right",
                "--relative",
                "on",
                "--view-size",
                // "11x7",
                &format!("{}x{}", area.width, area.height),
                &path,
            ])
            .stdout(Stdio::piped())
            .output()
            .unwrap()
            .stdout,
    )
    .to_string();

    app.movies_posters.insert(id, data);
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
