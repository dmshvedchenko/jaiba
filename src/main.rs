use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Context;
use arboard::Clipboard;
use chrono::Utc;
use crossterm::event::{self, Event, KeyCode};
use keepass::{Database, DatabaseKey};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, TableState},
};

use serde::Deserialize;

// ------------------ Theme

struct Theme {
    background: Color,
    text: Color,
    warning: Color,
    error: Color,
    success: Color,
    border: Color,
    header: Color,
    accent: Color,
    selection_fg: Color,
    selection_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Catppuccin Mocha, used if ~/.config/puma/theme.toml is missing or invalid.
        Self {
            background: Color::Rgb(30, 30, 46),
            text: Color::Rgb(205, 214, 244),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            border: Color::Rgb(69, 71, 90),
            header: Color::Rgb(147, 153, 178),
            accent: Color::Rgb(203, 166, 247),
            selection_fg: Color::Rgb(30, 30, 46),
            selection_bg: Color::Rgb(137, 180, 250),
        }
    }
}

#[derive(Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub text: String,

    pub border: String,
    pub header: String,

    pub accent: String,

    pub warning: String,
    pub error: String,
    pub success: String,

    pub selection_fg: String,
    pub selection_bg: String,
}

impl TryFrom<ThemeConfig> for Theme {
    type Error = anyhow::Error;

    fn try_from(cfg: ThemeConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            background: parse_hex(&cfg.colors.background)?,
            text: parse_hex(&cfg.colors.text)?,

            border: parse_hex(&cfg.colors.border)?,
            header: parse_hex(&cfg.colors.header)?,

            accent: parse_hex(&cfg.colors.accent)?,

            warning: parse_hex(&cfg.colors.warning)?,
            error: parse_hex(&cfg.colors.error)?,
            success: parse_hex(&cfg.colors.success)?,

            selection_fg: parse_hex(&cfg.colors.selection_fg)?,
            selection_bg: parse_hex(&cfg.colors.selection_bg)?,
        })
    }
}

fn parse_hex(hex: &str) -> anyhow::Result<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);

    if hex.len() != 6 {
        anyhow::bail!("expected 6 hex digits");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(Color::Rgb(r, g, b))
}

/// Expands a leading `~/` using $HOME. `fs::read_to_string` won't do this on its own
/// since there's no shell involved to interpret it.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

/// Loads the theme from ~/.config/puma/theme.toml. Callers should fall back to
/// `Theme::default()` if this fails (missing file, bad toml, bad hex, etc).
fn load_theme() -> anyhow::Result<Theme> {
    let path = expand_tilde("~/.config/puma/theme.toml");
    let text = fs::read_to_string(path)?;
    let config: ThemeConfig = toml::from_str(&text)?;
    Theme::try_from(config)
}

// ------------------ Config

struct Config {
    // None means no database configured yet; login will show an error saying so.
    default_database: Option<PathBuf>,
    auto_lock: Duration,
    clipboard_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_database: None,
            auto_lock: Duration::from_secs(300),
            clipboard_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    default_database: Option<String>,
    auto_lock: Option<u64>,
    clipboard_timeout: Option<u64>,
}

/// Loads ~/.config/puma/config.toml. Callers should fall back to `Config::default()`
/// if this fails (missing file, bad toml).
fn load_config() -> anyhow::Result<Config> {
    let path = expand_tilde("~/.config/puma/config.toml");
    let text =
        fs::read_to_string(&path).with_context(|| format!("couldn't read {}", path.display()))?;
    let raw: ConfigFile = toml::from_str(&text)?;

    let defaults = Config::default();

    Ok(Config {
        default_database: raw.default_database.map(|s| expand_tilde(&s)),
        auto_lock: raw
            .auto_lock
            .map(Duration::from_secs)
            .unwrap_or(defaults.auto_lock),
        clipboard_timeout: raw
            .clipboard_timeout
            .map(Duration::from_secs)
            .unwrap_or(defaults.clipboard_timeout),
    })
}

// ------------------ Types

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
    config: Config,

    // Shown under the password box on the login screen (bad password, no db configured, etc).
    login_error: Option<String>,

    // Reset on every keypress; if it's been idle longer than config.auto_lock while
    // unlocked, we wipe the decrypted entries and drop back to the login screen.
    last_activity: Instant,

    // ':' enters command mode; chars accumulate into command_buffer until Esc or a match.
    command_mode: bool,
    command_buffer: String,

    // Feedback for the last action, shown in the help bar until the next command.
    status: Option<String>,

    // Kept alive for the app's lifetime so clipboard content stays servable to other apps.
    clipboard: Option<Clipboard>,

    // Tracks the pending auto-clear (config.clipboard_timeout) for the last copy.
    clipboard_timer: Option<ClipboardTimer>,

    should_quit: bool,
}

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
    // Fall back to built-in defaults if either config file is missing/invalid, so a bad
    // or absent theme.toml / config.toml never prevents the app from starting.
    let theme = load_theme().unwrap_or_default();
    let config = load_config().unwrap_or_default();

    let mut app = App {
        screen: Screen::Login,
        password: String::new(),
        query: String::new(),
        max_len: 0,
        theme,
        config,
        login_error: None,
        last_activity: Instant::now(),
        table_state: TableState::default().with_selected(Some(0)),
        // Populated by attempt_unlock() once the user enters the master password.
        entries: Vec::new(),
        filtered: Vec::new(),
        command_mode: false,
        command_buffer: String::new(),
        status: None,
        // Retried lazily on first copy if this fails at startup (e.g. no X server yet).
        clipboard: Clipboard::new().ok(),
        clipboard_timer: None,
        should_quit: false,
    };

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
                app.last_activity = Instant::now();

                match app.screen {
                    Screen::Login => handle_login_input(&mut app, key.code),
                    Screen::Table => handle_table_input(&mut app, key.code),
                }
            }
        }

        maybe_clear_clipboard(&mut app);
        maybe_auto_lock(&mut app);

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

    if let Some(error) = &app.login_error {
        let error_area = ratatui::layout::Rect {
            x: input_area.x,
            y: input_area.y + input_area.height,
            width: input_area.width,
            height: 1,
        };

        let error_text = Paragraph::new(error.as_str())
            .alignment(Alignment::Center)
            .style(Style::new().fg(app.theme.error));

        frame.render_widget(error_text, error_area);
    }

    // Keep cursor inside the box
    let text_width = app.password.chars().count() as u16;
    let inner_width = input_area.width - 1;

    let cursor_x = input_area.x + (inner_width.saturating_sub(text_width) / 2) + text_width;

    frame.set_cursor_position((cursor_x, input_area.y + 1));
}

fn handle_login_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.login_error = None;

            if app.password.len() < app.max_len {
                app.password.push(c);
            }
        }

        KeyCode::Backspace => {
            app.login_error = None;
            app.password.pop();
        }

        KeyCode::Enter => {
            attempt_unlock(app);
        }

        KeyCode::Esc => {
            app.should_quit = true;
        }

        _ => {}
    }
}

fn attempt_unlock(app: &mut App) {
    let Some(path) = app.config.default_database.clone() else {
        app.login_error = Some("No default_database set in config.toml".to_string());
        return;
    };

    match unlock_database(&path, &app.password) {
        Ok(mut entries) => {
            calculate_warnings(&mut entries);
            app.entries = entries;
            app.password.clear();
            app.login_error = None;
            app.last_activity = Instant::now();
            app.refresh_filter();
            app.screen = Screen::Table;
        }
        Err(err) => {
            app.password.clear();
            app.login_error = Some(format!("{err}"));
        }
    }
}

/// Renders a last-modified timestamp as a relative "N days ago" string.
fn format_days_ago(dt: chrono::NaiveDateTime) -> String {
    let days = (Utc::now().naive_utc() - dt).num_days();

    match days {
        d if d <= 0 => "today".to_string(),
        1 => "1 day ago".to_string(),
        d => format!("{d} days ago"),
    }
}

fn unlock_database(path: &Path, password: &str) -> anyhow::Result<Vec<Entry>> {
    let mut file =
        fs::File::open(path).with_context(|| format!("couldn't open {}", path.display()))?;

    let key = DatabaseKey::new().with_password(password);
    let db = Database::open(&mut file, key).context("failed to unlock database")?;

    let entries = db
        .iter_all_entries()
        .map(|e| {
            let date_last_modify = e
                .times
                .last_modification
                .map(format_days_ago)
                .unwrap_or_default();

            Entry {
                name: e.get_title().unwrap_or("(no title)").to_string(),
                user: e.get_username().unwrap_or_default().to_string(),
                password: e.get_password().unwrap_or_default().to_string(),
                url: e.get_url().unwrap_or_default().to_string(),
                totp: e.get_raw_otp_value().unwrap_or_default().to_string(),
                date_last_modify,
                password_reuse_count: 0,
                duplicate_user_count: 0,
            }
        })
        .collect();

    Ok(entries)
}

// ------------------ Table

const HELP_ITEMS: &[&str] = &[
    "[:u] cp_user",
    "[:p] cp_password",
    "[:t] cp_totp",
    "[:r] cp_url",
    "[:a] add_entry",
    "[enter] expand_entry",
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

    let header = Row::new(["Name", "User", "Password", "Last Modify"])
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
        Paragraph::new(help_text).style(Style::new().fg(app.theme.accent))
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
            Cell::from(entry.date_last_modify.as_str()),
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
        .style(Style::new().fg(app.theme.text))
        .row_highlight_style(
            Style::new()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg)
                .bold(),
        )
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
                .border_style(Style::default().fg(app.theme.border)),
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
        Command::Quit => app.should_quit = true,
        Command::Settings => open_settings(app),
    }
}

fn add_entry(_app: &mut App) {
    // TODO: open an "add entry" form/screen
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

/// Copies `text`, starts a config.clipboard_timeout auto-clear timer, and reports the
/// outcome via `app.status`.
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
                clear_at: Instant::now() + app.config.clipboard_timeout,
            });
            app.status = None;
        }
        Err(err) => {
            app.clipboard_timer = None;
            app.status = Some(format!("Failed to copy {label}: {err}"));
        }
    }
}

/// Called every tick; locks the app and wipes decrypted entries after config.auto_lock
/// seconds of no keypresses. No-op on the login screen (nothing sensitive there yet).
fn maybe_auto_lock(app: &mut App) {
    if !matches!(app.screen, Screen::Table) {
        return;
    }

    if app.last_activity.elapsed() < app.config.auto_lock {
        return;
    }

    app.entries.clear();
    app.filtered.clear();
    app.password.clear();
    app.query.clear();
    app.screen = Screen::Login;
    app.login_error = Some("Locked after inactivity".to_string());
}

/// Called every tick; clears the clipboard once the configured window elapses, if it still holds our value.
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
