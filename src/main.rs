use std::{collections::HashMap, io};

use crossterm::event::{self, Event, KeyCode};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState},
};

struct Theme {
    background: Color,

    text: Color,
    text_dim: Color,
    warning: Color,

    border: Color,
    header: Color,

    help: Color,

    selected_fg: Color,
    selected_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),

            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(96, 99, 142),
            warning: Color::Rgb(249, 226, 175),

            border: Color::Rgb(71, 73, 108),
            header: Color::Rgb(96, 99, 142),

            help: Color::Rgb(203, 166, 247),

            selected_fg: Color::Yellow,
            selected_bg: Color::Black,
        }
    }
}

enum Screen {
    Login,
    Table,
}

struct Entry {
    name: String,
    user: String,
    password: String,
    url: String,
    totp: String,
    date_last_modify: String,
    password_reuse_count: u32,
    duplicate_user_count: u32,
}

struct App {
    screen: Screen,
    password: String,
    query: String,
    max_len: usize,
    table_state: TableState,
    entries: Vec<Entry>,
    filtered: Vec<usize>,
    theme: Theme,
}

impl App {
    fn compute_filtered(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();

        let mut indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.name.to_lowercase().contains(&query)
                    || entry.user.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        indices.sort_by(|&a, &b| {
            let ea = &self.entries[a];
            let eb = &self.entries[b];

            eb.password_reuse_count
                .cmp(&ea.password_reuse_count)
                .then_with(|| eb.duplicate_user_count.cmp(&ea.duplicate_user_count))
                .then_with(|| ea.name.to_lowercase().cmp(&eb.name.to_lowercase()))
        });

        indices
    }

    fn refresh_filter(&mut self) {
        self.filtered = self.compute_filtered();

        let count = self.filtered.len();
        self.table_state
            .select(if count == 0 { None } else { Some(0) });
    }
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
        theme: Theme::default(),
        table_state: TableState::default().with_selected(Some(0)),
        entries: vec![
            Entry {
                name: "GitHub".into(),
                user: "alice".into(),
                password: "hunter2".into(),
                url: "https://www.youtube.com/watch?v=yqGR9b9OItM".into(),
                date_last_modify: "12/12/2024".into(),
                totp: "123456".into(),
                password_reuse_count: 0,
                duplicate_user_count: 0,
            },
            Entry {
                name: "GitHub".into(),
                user: "malice".into(),
                password: "hunter2".into(),
                url: "https://www.youtube.com/watch?v=yqGR9b9OItM".into(),
                date_last_modify: "12/12/2024".into(),
                totp: "123456".into(),
                password_reuse_count: 0,
                duplicate_user_count: 0,
            },
            Entry {
                name: "GitHub".into(),
                user: "yalice".into(),
                password: "hunter2".into(),
                url: "https://www.youtube.com/watch?v=yqGR9b9OItM".into(),
                date_last_modify: "12/12/2024".into(),
                totp: "123456".into(),
                password_reuse_count: 0,
                duplicate_user_count: 0,
            },
        ],
        filtered: Vec::new(),
    };

    // Populate reuse/duplicate counts, then build the initial filtered view.
    calculate_warnings(&mut app.entries);
    app.refresh_filter();

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(app.theme.background)),
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
            .border_style(Style::default().fg(app.theme.border)),
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

const HELP_ITEMS: &[&str] = &[
    "[:u] cp_user",
    "[:p] cp_password",
    "[:t] cp_totp",
    "[:r] cp_url",
    "[:a] add_entry",
    "[:e] edit_entry",
    "[enter] preview_entry",
    "[:q] quit",
    "[:s] settings",
];

fn wrap_help_items(items: &[&str], width: u16) -> Vec<String> {
    let width = width as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for item in items {
        let candidate_len = if current.is_empty() {
            item.len()
        } else {
            current.len() + 2 + item.len()
        };

        if !current.is_empty() && candidate_len > width {
            lines.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push_str("  ");
        }
        current.push_str(item);
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }

    lines
}

fn draw_table(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),           // search
            Constraint::Fill(1),             // table
            Constraint::Length(help_height), // help bar
        ])
        .split(full_area);

    let query_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(vertical[0]);

    let header = Row::new(["Name", "User", "Password", "TOTP", "Last Modify"])
        .style(Style::new().bold().fg(app.theme.header))
        .bottom_margin(1);

    let help_text = help_lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");

    let help = Paragraph::new(help_text).style(Style::new().fg(app.theme.help));

    frame.render_widget(help, vertical[2]);

    let rows = app.filtered.iter().map(|&i| {
        let entry = &app.entries[i];
        let password = masked_password(entry, &app.theme);
        let user = masked_user(entry, &app.theme);

        Row::new([
            Cell::from(entry.name.as_str()),
            Cell::from(user),
            Cell::from(password),
            Cell::from(entry.totp.as_str()),
            Cell::from(entry.date_last_modify.as_str()),
        ])
    });

    let column_widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(35),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, column_widths)
        .header(header)
        .column_spacing(1)
        .style(Color::Rgb(205, 214, 244))
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
    match key {
        KeyCode::Char(c) => {
            if app.query.len() < app.max_len {
                app.query.push(c);
                app.refresh_filter();
            }
        }

        KeyCode::Backspace => {
            app.query.pop();
            app.refresh_filter();
        }

        KeyCode::Down => {
            let row_count = app.filtered.len();

            if row_count == 0 {
                return;
            }

            let selected = app.table_state.selected().unwrap_or(0);
            let next = (selected + 1).min(row_count - 1);
            app.table_state.select(Some(next));
        }

        KeyCode::Up => {
            let row_count = app.filtered.len();

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

fn calculate_warnings(entries: &mut Vec<Entry>) {
    let mut password_counts: HashMap<String, u32> = HashMap::new();
    let mut user_counts: HashMap<String, u32> = HashMap::new();

    for entry in entries.iter() {
        *password_counts.entry(entry.password.clone()).or_insert(0) += 1;
        *user_counts.entry(entry.user.clone()).or_insert(0) += 1;
    }

    for entry in entries.iter_mut() {
        entry.password_reuse_count = password_counts.get(&entry.password).copied().unwrap_or(0);

        entry.duplicate_user_count = user_counts.get(&entry.user).copied().unwrap_or(0);
    }
}

fn masked_password<'a>(entry: &'a Entry, theme: &Theme) -> Line<'a> {
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);

    let mut spans = vec![Span::styled("•••••", normal)];

    if entry.password_reuse_count > 1 {
        spans.push(Span::styled(
            format!(" [{}]", entry.password_reuse_count),
            warning,
        ));
    }

    Line::from(spans)
}

fn masked_user<'a>(entry: &'a Entry, theme: &Theme) -> Line<'a> {
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);

    let mut spans = vec![Span::styled(entry.user.as_str(), normal)];

    if entry.duplicate_user_count > 1 {
        spans.push(Span::styled(
            format!(" [{}]", entry.duplicate_user_count),
            warning,
        ));
    }

    Line::from(spans)
}
