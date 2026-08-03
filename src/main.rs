use std::io;

mod app;
mod clipboard;
mod commands;
mod config;
mod database;
mod entry;
mod theme;
mod ui;

use app::App;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let result = App::run(&mut terminal);

    ratatui::restore();

    result
}

// use std::{
//     collections::HashMap,
//     fs, io,
//     path::{Path, PathBuf},
//     time::{Duration, Instant},
// };

// use anyhow::Context;
// use arboard::Clipboard;
// use chrono::Utc;
// use crossterm::event::{self, Event, KeyCode};
// use keepass::{Database, DatabaseKey};

// use ratatui::{
//     Frame, Terminal,
//     backend::CrosstermBackend,
//     layout::{Alignment, Constraint, Direction, Layout},
//     style::{Color, Style},
//     text::{Line, Span},
//     widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState},
// };

// use serde::Deserialize;
