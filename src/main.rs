mod app;
mod clipboard;
mod config;
mod db;
mod export;
mod import;
mod input;
mod theme;
mod ui;
mod util;

use std::io;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    app::run(&mut terminal)?;

    ratatui::restore();

    Ok(())
}
