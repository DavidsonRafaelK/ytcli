#[cfg(not(target_os = "macos"))]
compile_error!("yt only supports macOS");

mod app;
mod browser;
mod config;
mod error;
mod feed;
mod format;
mod layout;
mod model;
mod player;
mod thumbnail;
mod ui;
mod videotoolbox;
mod worker;
mod ytdlp;

use std::io;
use std::process::ExitCode;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::Event;
use ratatui::layout::Size;
use ratatui_image::picker::Picker;

use app::App;
use worker::{Msg, Worker};

fn main() -> ExitCode {
    let mut terminal = ratatui::init();
    // Asks the terminal which image protocol it speaks; must run before reading any input.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let result = run(&mut terminal, &picker);
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("yt: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, picker: &Picker) -> io::Result<()> {
    let (worker, inbox) = Worker::spawn(picker);
    let mut app = App::new(worker, picker.font_size(), terminal.size()?);
    while !app.quit {
        app.request_thumbnails();
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        // Sleeps until a key, a resize or background work arrives, so idling costs nothing.
        let Ok(msg) = inbox.recv() else {
            return Ok(());
        };
        handle(&mut app, msg)?;
        // Whatever else is already waiting shares the next redraw.
        while let Ok(msg) = inbox.try_recv() {
            if app.quit {
                break;
            }
            handle(&mut app, msg)?;
        }
    }
    Ok(())
}

fn handle(app: &mut App, msg: Msg) -> io::Result<()> {
    match msg {
        Msg::Input(event) => match event? {
            Event::Key(key) => app.on_key(key),
            Event::Resize(width, height) => app.resize(Size::new(width, height)),
            _ => {}
        },
        msg => app.on_msg(msg),
    }
    Ok(())
}
