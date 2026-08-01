use std::io;

use crossterm::event::{self, Event, KeyCode};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

struct App {
    password: String,
    max_len: usize,
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    run(&mut terminal)?;

    ratatui::restore();

    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App {
        password: String::new(),
        max_len: 0,
    };

    loop {
        terminal.draw(|frame| {
            // draw_login(frame, &mut app);
            draw_table(frame, &mut app);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char(c) => {
                    if app.password.chars().count() < app.max_len {
                        app.password.push(c);
                    }
                }

                KeyCode::Backspace => {
                    app.password.pop();
                }

                KeyCode::Esc => {
                    break;
                }

                _ => {}
            }
        }
    }

    Ok(())
}

fn draw_login(frame: &mut Frame, app: &mut App) {
    let background = Block::default().style(Style::default().bg(Color::Rgb(37, 39, 57)));

    frame.render_widget(background, frame.area());

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(frame.area());

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);

    let input_area = horizontal[1];

    // Calculate available input characters
    app.max_len = (input_area.width - 2) as usize;

    // Mask password
    let masked = "•".repeat(app.password.chars().count());

    let input = Paragraph::new(masked).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(71, 73, 108))),
    );

    frame.render_widget(input, input_area);

    // Keep cursor inside the box
    let text_width = app.password.chars().count() as u16;
    let inner_width = input_area.width - 1;

    let cursor_x = input_area.x + (inner_width.saturating_sub(text_width) / 2) + text_width;

    frame.set_cursor_position((cursor_x, input_area.y + 1));
}

fn draw_table(frame: &mut Frame, app: &mut App) {
    let background = Block::default().style(Style::default().bg(Color::Rgb(37, 39, 57)));

    frame.render_widget(background, frame.area());

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(frame.area());

    let first_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Length(50),
        ])
        .split(vertical[0]);

    let table_header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);

    let table_body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(vertical[2]);

    let query_area = first_row[0];

    // Calculate available input characters
    // app.max_len = (query_area.width - 2) as usize;

    let input = Paragraph::new("").block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(71, 73, 108))),
    );

    frame.render_widget(input, query_area);
}
