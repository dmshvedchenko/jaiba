use std::io;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| {
        draw_login(frame);
    })?;

    event::read()?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn draw_login(frame: &mut Frame) {
    // Background
    let background = Block::default().style(Style::default().bg(Color::Rgb(30, 30, 40)));

    frame.render_widget(background, frame.area());

    // Center vertically
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(frame.area());

    // Center horizontally
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);

    let password = Block::default()
        // .title("Password")
        .borders(Borders::ALL)
        // .style(Style::default().bg(Color::Rgb(45, 46, 67)));
        .border_style(Style::default().fg(Color::Rgb(71, 73, 108)));

    frame.render_widget(password, horizontal[1]);
}
