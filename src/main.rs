use clap::Parser;

mod add;
mod app;
mod args;
mod custom_widgets;
mod database;
mod draw;
mod input_handler;
mod tui;

use app::App;
use database::*;
use ratatui::prelude::*;
use std::{
    io::{stdin, stdout, BufRead, BufReader, Result},
    process::{Command, Stdio},
};
use tui::Tui;

fn main() -> Result<()> {
    let cli = args::Cli::parse();

    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // let (socket, pid) = start_ueberzug();

    let mut tui = Tui::new(terminal);
    tui.init()?;
    let mut app = App::new(cli.command.is_some());
    init_movies(&mut app);

    let result = tui.run(&mut app);

    // let _ = Command::new("kill").arg(&pid).status();
    tui.exit()?;
    result?;

    // println!("{:?}", app.posters_requested);

    Ok(())
    // if let Some(config) = cli.config {
    //     println!("custom config: {}", config.display());
    // }
}

fn start_ueberzug() -> (String, String) {
    let pidfile = format!(
        "{}/.cache/ueberzugpp-pid",
        std::env::var("HOME").expect("couldn't get pid file!")
    );
    let _ = std::fs::remove_file(&pidfile);
    let _ = Command::new("ueberzugpp")
        .args(["layer", "-o", "kitty", "--no-stdin", "--pid-file", &pidfile])
        .spawn();

    let mut pid = String::from_utf8(
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "while ! test -f {0}; do :; done; tail {0}",
                &pidfile
            ))
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .expect("Couldn't get pid")
            .stdout,
    )
    .expect("Couldn't get pid");
    (format!("/tmp/ueberzugpp-{}.socket", pid), pid)
}

fn init_movies(app: &mut App) {
    app.set_movies(fetch_movies().unwrap());
}

// use crossterm::event::{self, KeyCode, KeyEvent};
// use crossterm::execute;
// use serde_json::json;
// use std::io::{self, Write};
// use std::os::unix::net::UnixStream;
// use std::process::Command;
// use tokio::process::Command as TokioCommand;

// #[tokio::main]
// async fn main() -> io::Result<()> {
//     // Start the ueberzugpp daemon
//     let mut daemon = Command::new("ueberzugpp")
//         .arg("layer")
//         .arg("-o")
//         .arg("kitty")
//         .spawn()
//         .expect("Failed to start ueberzugpp daemon");

//     // Ensure the daemon is running
//     if daemon.id() == 0 {
//         eprintln!("Failed to spawn ueberzugpp daemon");
//         return Ok(());
//     }

//     // Prepare terminal for input
//     let mut stdout = io::stdout();
//     // execute!(
//     //     stdout,
//     //     crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
//     // )?;

//     println!("Press '.' to add an image. Press 'q' to quit.");

//     let socket = format!("/tmp/ueberzugpp-{}.socket", daemon.id());

//     // Main event loop
//     loop {
//         if event::poll(std::time::Duration::from_millis(100))? {
//             if let event::Event::Key(KeyEvent { code, .. }) = event::read()? {
//                 match code {
//                     KeyCode::Char('q') => {
//                         break; // Quit on 'q'
//                     }
//                     KeyCode::Char('.') => {
//                         // Add an image on '.'
//                         let image_path = "/home/semirose/HDD/Rust/moviedb/src/0a6eeef0.jpg"; // Change this to your image path
//                         let _ = Command::new("ueberzugpp")
//                             .arg("cmd")
//                             .arg("-s")
//                             .arg(&socket)
//                             .arg("-i")
//                             .arg(1212.to_string())
//                             .arg("-a")
//                             .arg("add")
//                             .arg("-x")
//                             .arg(0.to_string())
//                             .arg("-y")
//                             .arg(0.to_string())
//                             .arg("--max-width")
//                             .arg(100.to_string())
//                             .arg("--max-height")
//                             .arg(100.to_string())
//                             .arg("-f")
//                             .arg(image_path)
//                             .spawn();
//                         let _ = Command::new("ueberzugpp")
//                             .arg("cmd")
//                             .arg("-s")
//                             .arg(&socket)
//                             .arg("-i")
//                             .arg(12.to_string())
//                             .arg("-a")
//                             .arg("add")
//                             .arg("-x")
//                             .arg(20.to_string())
//                             .arg("-y")
//                             .arg(0.to_string())
//                             .arg("--max-width")
//                             .arg(100.to_string())
//                             .arg("--max-height")
//                             .arg(100.to_string())
//                             .arg("-f")
//                             .arg(image_path)
//                             .spawn();

//                         // let command = json!({
//                         //     "action": "add",
//                         //     "identifier": "image1", // Unique identifier for the image
//                         //     "x": 0, // X position
//                         //     "y": 0, // Y position
//                         //     "max_width": 100, // Width
//                         //     "max_height": 100, // Height
//                         //     "path": image_path
//                         // });
//                         // println!("{}", command);

//                         // if let Err(e) = stream.write_all(r#"'{"path": "~/Pictures/HUH/100181378_p0.png", "action": "add", "identifier": "ass", "x": 0, "y": 0, "width": 20, "height": 20}'"#.as_bytes()) {
//                         //     eprintln!("Failed to send command: {}", e);
//                         // }
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     }

//     // Ensure the daemon is terminated
//     let _ = daemon.kill();
//     Ok(())
// }
