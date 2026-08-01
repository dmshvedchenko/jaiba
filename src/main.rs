use std::io;

use crossterm::event::{self, Event, KeyCode};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState},
};

enum Screen {
    Login,
    Table,
}

struct Entry {
    name: String,
    user: String,
    password: String,
    totp: String,
}

struct App {
    screen: Screen,
    password: String,
    query: String,
    max_len: usize,
    table_state: TableState,
    entries: Vec<Entry>,
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    run(&mut terminal)?;

    ratatui::restore();

    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App {
        screen: Screen::Login,
        password: String::new(),
        query: String::new(),
        max_len: 0,
        table_state: TableState::default().with_selected(Some(0)),
        entries: vec![
            Entry {
                name: "GitHub".into(),
                user: "alice".into(),
                password: "•••••".into(),
                totp: "•••••".into(),
            },
            Entry {
                name: "Google".into(),
                user: "alice@gmail.com".into(),
                password: "•••••".into(),
                totp: "•••••".into(),
            },
            Entry {
                name: "Discord".into(),
                user: "Alice".into(),
                password: "•••••".into(),
                totp: "--".into(),
            },
            Entry {
                name: "GitHub".into(),
                user: "alice".into(),
                password: "•••••".into(),
                totp: "•••••".into(),
            },
            Entry {
                name: "Google".into(),
                user: "alice@gmail.com".into(),
                password: "•••••".into(),
                totp: "•••••".into(),
            },
            Entry {
                name: "Discord".into(),
                user: "Alice".into(),
                password: "•••••".into(),
                totp: "--".into(),
            },
        ],
    };

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Rgb(30, 30, 46))),
                frame.area(),
            );

            match app.screen {
                Screen::Login => draw_login(frame, &mut app),
                Screen::Table => draw_table(frame, &mut app),
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match app.screen {
                Screen::Login => handle_login_input(&mut app, key.code),
                Screen::Table => handle_table_input(&mut app, key.code),
            }

            if matches!(key.code, KeyCode::Esc) {
                break;
            }
        }
    }

    Ok(())
}

fn draw_login(frame: &mut Frame, app: &mut App) {
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

fn handle_login_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            if app.password.len() < app.max_len {
                app.password.push(c);
            }
        }

        KeyCode::Backspace => {
            app.password.pop();
        }

        KeyCode::Enter => {
            // TODO replace for actual password
            if app.password == "aa" {
                app.screen = Screen::Table;
            }
        }

        _ => {}
    }
}

fn draw_table(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search
            Constraint::Fill(1),   // table
            Constraint::Length(1), // help bar
        ])
        .split(frame.area());

    let query_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])
        .split(vertical[0]);

    let header = Row::new(["Name", "User", "Password", "TOTP"])
        .style(Style::new().bold().fg(Color::Rgb(96, 99, 142)))
        .bottom_margin(1);

    let help =
        Paragraph::new("  [u]cp_user  [p]cp_password  [t]cp_totp  [a]add_entry  [e]edit  [x]expand  ")
            .style(Style::new().fg(Color::Rgb(203, 166, 247)));

    frame.render_widget(help, vertical[2]);

    let button_style = Style::new().fg(Color::White);

    let rows = app.entries.iter().map(|entry| {
        Row::new([
            Cell::from(Span::styled(format!("{} ", entry.name.as_str()), button_style)),
            Cell::from(Span::styled(format!("{} ", entry.user), button_style)),
            Cell::from(Span::styled(format!("{} ", entry.password), button_style)),
            Cell::from(Span::styled(format!("{} ", entry.totp), button_style)),
        ])
    });

    let column_widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, column_widths)
        .header(header)
        .column_spacing(1)
        .style(Color::Rgb(216, 218, 234))
        .row_highlight_style(Style::new().on_black().bold())
        .column_highlight_style(Color::Gray)
        .cell_highlight_style(Style::new().reversed().yellow())
        .highlight_symbol("→ ");

    let table_area = vertical[1];

    frame.render_stateful_widget(table, table_area, &mut app.table_state);

    let query_area = query_row[1];

    app.max_len = query_area.width.saturating_sub(4) as usize;

    let input = Paragraph::new(app.query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .border_style(Style::default().fg(Color::Rgb(71, 73, 108))),
    );

    frame.set_cursor_position((
        query_area.x + 2 + app.query.chars().count() as u16,
        query_area.y + 1,
    ));

    frame.render_widget(input, query_area);
}

fn handle_table_input(app: &mut App, key: KeyCode) {
    let row_count = app.entries.len();

    match key {
        KeyCode::Char(c) => {
            if app.query.len() < app.max_len {
                app.query.push(c);
            }
        }

        KeyCode::Backspace => {
            app.query.pop();
        }

        KeyCode::Down => {
            if row_count == 0 {
                return;
            }

            let selected = app.table_state.selected().unwrap_or(0);
            let next = (selected + 1).min(row_count - 1);
            app.table_state.select(Some(next));
        }

        KeyCode::Up => {
            if row_count == 0 {
                return;
            }

            let selected = app.table_state.selected().unwrap_or(0);
            let prev = selected.saturating_sub(1);
            app.table_state.select(Some(prev));
        }

        _ => {}
    }
}
