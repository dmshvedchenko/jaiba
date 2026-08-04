use std::io;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{self, Event};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::Style,
    widgets::{Block, ListState, TableState},
};

use crate::clipboard::{ClipboardTimer, maybe_clear_clipboard};
use crate::config::{Config, load_config};
use crate::db::Entry;
use crate::input::edit::handle_edit_input;
use crate::input::index::handle_index_input;
use crate::input::login::handle_login_input;
use crate::theme::{Theme, load_theme};
use crate::ui::edit::draw_edit;
use crate::ui::index::draw_index;
use crate::ui::login::draw_login;

pub enum Screen {
    Login,
    Index,
    Edit,
}

pub struct App {
    pub screen: Screen,
    pub password: String,
    pub query: String,
    pub max_len: usize,
    pub index_state: TableState,
    pub edit_state: ListState,
    pub reveal_password: bool,
    /// Snapshot of the entry being shown on the edit screen. `Some` for both
    /// "editing an existing entry" and "creating a new one" — the two are
    /// told apart by `edit_target`.
    pub edit_entry: Option<Entry>,
    /// Index into `entries` this edit maps back to, or `None` if `edit_entry`
    /// is a brand new entry that doesn't exist in `entries` yet.
    pub edit_target: Option<usize>,
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,
    pub theme: Theme,
    pub config: Config,

    pub login_error: Option<String>,

    pub last_activity: Instant,

    pub command_mode: bool,
    pub command_buffer: String,

    pub status: Option<String>,

    pub clipboard: Option<Clipboard>,

    pub clipboard_timer: Option<ClipboardTimer>,

    pub should_quit: bool,
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

    pub fn refresh_filter(&mut self) {
        self.filtered = self.compute_filtered();

        let count = self.filtered.len();
        self.index_state
            .select(if count == 0 { None } else { Some(0) });
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        let selected = self.index_state.selected()?;
        let entry_idx = *self.filtered.get(selected)?;
        self.entries.get(entry_idx)
    }
}

fn maybe_auto_lock(app: &mut App) {
    if !matches!(app.screen, Screen::Index | Screen::Edit) {
        return;
    }

    if app.last_activity.elapsed() < app.config.auto_lock {
        return;
    }

    app.entries.clear();
    app.filtered.clear();
    app.password.clear();
    app.query.clear();
    app.edit_entry = None;
    app.edit_target = None;
    app.reveal_password = false;
    app.screen = Screen::Login;
    app.login_error = Some("Locked after inactivity".to_string());
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let config = load_config().unwrap_or_default();
    let theme = load_theme(config.theme.as_deref()).unwrap_or_default();

    let mut app = App {
        screen: Screen::Login,
        password: String::new(),
        query: String::new(),
        max_len: 0,
        theme,
        config,
        login_error: None,
        last_activity: Instant::now(),
        index_state: TableState::default().with_selected(Some(0)),
        edit_state: ListState::default(),
        reveal_password: false,
        edit_entry: None,
        edit_target: None,
        entries: Vec::new(),
        filtered: Vec::new(),
        command_mode: false,
        command_buffer: String::new(),
        status: None,
        clipboard: Clipboard::new().ok(),
        clipboard_timer: None,
        should_quit: false,
    };

    const TICK_RATE: Duration = Duration::from_millis(200);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(app.theme.background)),
                frame.area(),
            );

            match app.screen {
                Screen::Login => draw_login(frame, &mut app),
                Screen::Index => draw_index(frame, &mut app),
                Screen::Edit => draw_edit(frame, &mut app),
            }
        })?;

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                app.last_activity = Instant::now();

                match app.screen {
                    Screen::Login => handle_login_input(&mut app, key.code),
                    Screen::Index => handle_index_input(&mut app, key.code),
                    Screen::Edit => handle_edit_input(&mut app, key.code),
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
