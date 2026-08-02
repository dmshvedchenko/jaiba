use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState},
};

// ------------------ Types

struct Theme {
    background: Color,

    text: Color,
    // text_dim: Color,
    warning: Color,

    border: Color,
    header: Color,

    help: Color,
    // selected_fg: Color,
    // selected_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),

            text: Color::Rgb(205, 214, 244),
            // text_dim: Color::Rgb(96, 99, 142),
            warning: Color::Rgb(249, 226, 175),

            border: Color::Rgb(71, 73, 108),
            header: Color::Rgb(96, 99, 142),

            help: Color::Rgb(203, 166, 247),
            // selected_fg: Color::Yellow,
            // selected_bg: Color::Black,
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

// ------------------ App

struct App {
    screen: Screen,
    password: String,
    query: String,
    max_len: usize,
    table_state: TableState,
    entries: Vec<Entry>,
    filtered: Vec<usize>,
    theme: Theme,

    // ':' enters command mode; chars accumulate into command_buffer until Esc or a match.
    command_mode: bool,
    command_buffer: String,

    // Feedback for the last action, shown in the help bar until the next command.
    status: Option<String>,

    // Kept alive for the app's lifetime so clipboard content stays servable to other apps.
    clipboard: Option<Clipboard>,

    // Tracks the pending 15s auto-clear for the last copy.
    clipboard_timer: Option<ClipboardTimer>,

    should_quit: bool,
}

const CLIPBOARD_TTL: Duration = Duration::from_secs(15);

struct ClipboardTimer {
    label: String,
    expected: String,
    clear_at: Instant,
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

    fn selected_entry(&self) -> Option<&Entry> {
        let selected = self.table_state.selected()?;
        let entry_idx = *self.filtered.get(selected)?;
        self.entries.get(entry_idx)
    }
}

// ------------------ Main

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
                user: "alice".into(),
                password: "hunter".into(),
                url: "https://www.youtube.com/watch?v=yqGR9b9OItM".into(),
                date_last_modify: "12/12/2024".into(),
                totp: "123456".into(),
                password_reuse_count: 0,
                duplicate_user_count: 0,
            },
        ],
        filtered: Vec::new(),
        command_mode: false,
        command_buffer: String::new(),
        status: None,
        // Retried lazily on first copy if this fails at startup (e.g. no X server yet).
        clipboard: Clipboard::new().ok(),
        clipboard_timer: None,
        should_quit: false,
    };

    // Populate reuse/duplicate counts, then build the initial filtered view.
    calculate_warnings(&mut app.entries);
    app.refresh_filter();

    // Wake periodically even with no input, to tick the countdown and auto-clear.
    const TICK_RATE: Duration = Duration::from_millis(200);

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

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                match app.screen {
                    Screen::Login => handle_login_input(&mut app, key.code),
                    Screen::Table => handle_table_input(&mut app, key.code),
                }
            }
        }

        maybe_clear_clipboard(&mut app);

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

// ------------------ Login

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

        KeyCode::Esc => {
            app.should_quit = true;
        }

        _ => {}
    }
}

// ------------------ Table

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
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(help_height),
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

    let help = if let Some(timer) = &app.clipboard_timer {
        // Recomputed every draw so the countdown ticks down smoothly.
        let remaining = timer
            .clear_at
            .saturating_duration_since(Instant::now())
            .as_secs()
            + 1; // round up so it doesn't flash "0s" before the tick that clears it

        Paragraph::new(format!(
            "  Copied {} :: clearing in {remaining}s",
            timer.label
        ))
        .style(Style::new().fg(app.theme.warning))
    } else if let Some(status) = &app.status {
        // Replace the help text with the latest status message.
        Paragraph::new(format!("  {status}")).style(Style::new().fg(app.theme.warning))
    } else {
        Paragraph::new(help_text).style(Style::new().fg(app.theme.help))
    };

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

    // While in command mode, show ":<buffer>" instead of the search query.
    let (input_text, input_style) = if app.command_mode {
        (
            format!(":{}", app.command_buffer),
            Style::default().fg(app.theme.warning),
        )
    } else {
        (app.query.clone(), Style::default().fg(app.theme.text))
    };

    let input = Paragraph::new(input_text.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(Color::Rgb(71, 73, 108))),
        );

    frame.set_cursor_position((
        query_area.x + 2 + input_text.chars().count() as u16,
        query_area.y + 1,
    ));

    frame.render_widget(input, query_area);
}

fn handle_table_input(app: &mut App, key: KeyCode) {
    if app.command_mode {
        handle_command_input(app, key);
        return;
    }

    match key {
        KeyCode::Char(':') => {
            app.status = None;
            app.command_mode = true;
            app.command_buffer.clear();
        }

        KeyCode::Char(c) => {
            app.status = None;

            if app.query.len() < app.max_len {
                app.query.push(c);
                app.refresh_filter();
            }
        }

        KeyCode::Backspace => {
            app.status = None;
            app.query.pop();
            app.refresh_filter();
        }

        KeyCode::Enter => {
            app.status = None;
            preview_entry(app);
        }

        KeyCode::Down => {
            let row_count = app.filtered.len();

            if row_count == 0 {
                return;
            }

            app.status = None;

            let selected = app.table_state.selected().unwrap_or(0);

            let next = if selected >= row_count - 1 {
                0
            } else {
                selected + 1
            };

            app.table_state.select(Some(next));
        }

        KeyCode::Up => {
            let row_count = app.filtered.len();

            if row_count == 0 {
                return;
            }

            app.status = None;

            let selected = app.table_state.selected().unwrap_or(0);

            let prev = if selected == 0 {
                row_count - 1
            } else {
                selected - 1
            };

            app.table_state.select(Some(prev));
        }

        _ => {}
    }
}

// ------------------ Commands

const COMMANDS: &[(&str, Command)] = &[
    ("u", Command::CopyUser),
    ("p", Command::CopyPassword),
    ("t", Command::CopyTotp),
    ("r", Command::CopyUrl),
    ("a", Command::AddEntry),
    ("e", Command::EditEntry),
    ("q", Command::Quit),
    ("s", Command::Settings),
];

#[derive(Clone, Copy)]
enum Command {
    CopyUser,
    CopyPassword,
    CopyTotp,
    CopyUrl,
    AddEntry,
    EditEntry,
    Quit,
    Settings,
}

enum CommandMatch {
    Exact(Command),
    Prefix,
    Invalid,
}

fn match_command(buffer: &str) -> CommandMatch {
    let mut exact = None;
    let mut is_prefix = false;

    for (name, cmd) in COMMANDS {
        if *name == buffer {
            exact = Some(*cmd);
        } else if name.starts_with(buffer) {
            is_prefix = true;
        }
    }

    match (exact, is_prefix) {
        (Some(cmd), _) => CommandMatch::Exact(cmd),
        (None, true) => CommandMatch::Prefix,
        (None, false) => CommandMatch::Invalid,
    }
}

fn handle_command_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.command_mode = false;
            app.command_buffer.clear();
        }

        KeyCode::Backspace => {
            app.command_buffer.pop();

            if app.command_buffer.is_empty() {
                app.command_mode = false;
            }
        }

        KeyCode::Char(c) => {
            app.command_buffer.push(c);

            match match_command(&app.command_buffer) {
                CommandMatch::Exact(cmd) => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                    execute_command(app, cmd);
                }

                CommandMatch::Prefix => {
                    // Keep collecting characters.
                }

                CommandMatch::Invalid => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                }
            }
        }

        _ => {}
    }
}

fn execute_command(app: &mut App, cmd: Command) {
    match cmd {
        Command::CopyUser => cp_user(app),
        Command::CopyPassword => cp_password(app),
        Command::CopyTotp => cp_totp(app),
        Command::CopyUrl => cp_url(app),
        Command::AddEntry => add_entry(app),
        Command::EditEntry => edit_entry(app),
        Command::Quit => app.should_quit = true,
        Command::Settings => open_settings(app),
    }
}

fn add_entry(_app: &mut App) {
    // TODO: open an "add entry" form/screen
}

fn edit_entry(app: &mut App) {
    if let Some(_entry) = app.selected_entry() {
        // TODO: open an "edit entry" form/screen for _entry
    }
}

fn preview_entry(app: &mut App) {
    if let Some(_entry) = app.selected_entry() {
        // TODO: show entry detail popup for _entry
    }
}

fn open_settings(_app: &mut App) {
    // TODO: open settings screen
}

// ------------------ Clipboard

/// Copies `text`, starts a 15s auto-clear timer, and reports the outcome via `app.status`.
fn copy_and_report(app: &mut App, label: &str, text: &str) {
    let result = match app.clipboard.as_mut() {
        Some(clipboard) => clipboard.set_text(text),
        None => {
            // Wasn't available at startup — try to connect now.
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    let result = clipboard.set_text(text);
                    app.clipboard = Some(clipboard);
                    result
                }
                Err(err) => Err(err),
            }
        }
    };

    match result {
        Ok(()) => {
            app.clipboard_timer = Some(ClipboardTimer {
                label: label.to_string(),
                expected: text.to_string(),
                clear_at: Instant::now() + CLIPBOARD_TTL,
            });
            app.status = None;
        }
        Err(err) => {
            app.clipboard_timer = None;
            app.status = Some(format!("Failed to copy {label}: {err}"));
        }
    }
}

/// Called every tick; clears the clipboard once the 15s window elapses, if it still holds our value.
fn maybe_clear_clipboard(app: &mut App) {
    let Some(timer) = app.clipboard_timer.as_ref() else {
        return;
    };

    if Instant::now() < timer.clear_at {
        return;
    }

    let expected = std::mem::take(&mut app.clipboard_timer).unwrap().expected;

    if let Some(clipboard) = app.clipboard.as_mut() {
        let holds_ours = matches!(clipboard.get_text(), Ok(current) if current == expected);

        if holds_ours {
            if let Err(err) = clipboard.clear() {
                app.status = Some(format!("Couldn't clear clipboard: {err}"));
                return;
            }
        }
    }

    app.status = Some("Clipboard cleared".to_string());
}

fn cp_user(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let user = entry.user.clone();
        copy_and_report(app, "user", &user);
    }
}

fn cp_password(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let password = entry.password.clone();
        copy_and_report(app, "password", &password);
    }
}

fn cp_totp(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let totp = entry.totp.clone();
        copy_and_report(app, "TOTP code", &totp);
    }
}

fn cp_url(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let url = entry.url.clone();
        copy_and_report(app, "URL", &url);
    }
}

// ------------------ Entries

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
