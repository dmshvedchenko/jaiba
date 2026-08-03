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

pub fn draw(frame: &mut Frame, app: &mut App) {
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

pub fn handle_input(app: &mut App, key: KeyCode) {
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
