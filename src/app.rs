pub enum Screen {
    Login,
    Table,
}

pub struct App {
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

impl App {
    pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
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
                    Screen::Login => ui::login::draw(frame, &mut app),
                    Screen::Table => ui::table::draw(frame, &mut app),
                }
            })?;

            if event::poll(TICK_RATE)? {
                if let Event::Key(key) = event::read()? {
                    app.last_activity = Instant::now();

                    match app.screen {
                        Screen::Login => ui::login::handle_input(&mut app, key.code),
                        Screen::Table => ui::table::handle_input(&mut app, key.code),
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

    fn attempt_unlock(&mut self) {
        let Some(path) = self.config.default_database.clone() else {
            self.login_error = Some("No default_database set in config.toml".to_string());
            return;
        };

        match unlock_database(&path, &self.password) {
            Ok(mut entries) => {
                calculate_warnings(&mut entries);
                self.entries = entries;
                self.password.clear();
                self.login_error = None;
                self.last_activity = Instant::now();
                self.refresh_filter();
                self.screen = Screen::Table;
            }
            Err(err) => {
                self.password.clear();
                self.login_error = Some(format!("{err}"));
            }
        }
    }

    fn maybe_auto_lock(&mut self) {
        if !matches!(self.screen, Screen::Table) {
            return;
        }

        if self.last_activity.elapsed() < self.config.auto_lock {
            return;
        }

        self.entries.clear();
        self.filtered.clear();
        self.password.clear();
        self.query.clear();
        self.screen = Screen::Login;
        self.login_error = Some("Locked after inactivity".to_string());
    }
}
