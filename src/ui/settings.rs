use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;
use crate::input::settings::{AUTO_LOCK_ROW, CLIPBOARD_TIMEOUT_ROW, DATABASE_ROW};
use crate::util::wrap_help_items;

const NAV_HELP_ITEMS: &[&str] = &["[↑↓] navigate", "[enter] edit / choose theme", "[esc] back"];
const FIELD_HELP_ITEMS: &[&str] = &["[enter] save", "[esc] cancel"];
const THEME_PICKER_HELP_ITEMS: &[&str] = &["[↑↓] navigate", "[enter] apply", "[esc] cancel"];

pub fn draw_settings(frame: &mut Frame, app: &mut App) {
    if app.choosing_theme {
        draw_theme_picker(frame, app);
    } else {
        draw_main_settings(frame, app);
    }
}

fn draw_main_settings(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let editing_field = app.editing_field;
    let field_buffer = app.field_buffer.clone();
    let selected = app.settings_state.selected().unwrap_or(0);

    let help_items = if editing_field {
        FIELD_HELP_ITEMS
    } else {
        NAV_HELP_ITEMS
    };
    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(help_items, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(help_height)])
        .split(full_area);

    let theme = &app.theme;
    let normal = Style::new().fg(theme.text);
    let accent = Style::new().fg(theme.accent);
    let warning = Style::new().fg(theme.warning);
    let border_style = Style::new().fg(theme.border);
    let placeholder = Style::new().fg(theme.border).italic();
    let editing_style = Style::new().fg(theme.selection_fg).bg(theme.selection_bg);
    let label_style = Style::new().fg(theme.header).bold();

    let render_value = |field_index: usize, value: String| -> Line<'static> {
        if editing_field && selected == field_index {
            return Line::from(vec![
                Span::styled(field_buffer.clone(), editing_style),
                Span::styled("▏", accent),
            ]);
        }

        if value.is_empty() {
            return Line::from(Span::styled("(not set)", placeholder));
        }

        Line::from(Span::styled(value, normal))
    };

    let field = |label: &'static str, value: Line<'static>| -> ListItem<'static> {
        ListItem::new(vec![
            Line::from(Span::styled(label, label_style)),
            value,
            Line::from(""), // spacer
        ])
    };

    let database_value = app
        .config
        .default_database
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let auto_lock_value = app.config.auto_lock.as_secs().to_string();
    let clipboard_timeout_value = app.config.clipboard_timeout.as_secs().to_string();

    let theme_value = if app.available_themes.is_empty() {
        Line::from(Span::styled(
            "no themes found in ~/.config/jaiba/themes",
            warning,
        ))
    } else {
        match app.config.theme.as_deref() {
            Some(name) => Line::from(Span::styled(name.to_string(), normal)),
            None => Line::from(Span::styled("(not set)", placeholder)),
        }
    };

    let items = vec![
        field(
            "Default database",
            render_value(DATABASE_ROW, database_value),
        ),
        field(
            "Auto-lock (seconds)",
            render_value(AUTO_LOCK_ROW, auto_lock_value),
        ),
        field(
            "Clipboard timeout (seconds)",
            render_value(CLIPBOARD_TIMEOUT_ROW, clipboard_timeout_value),
        ),
        field("Theme", theme_value),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(border_style)
                .title(" Settings "),
        )
        .highlight_style(editing_style.bold())
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, vertical[0], &mut app.settings_state);

    let help = if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(warning)
    } else {
        let help_text = help_lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(help_text).style(accent)
    };

    frame.render_widget(help, vertical[1]);
}

fn draw_theme_picker(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(THEME_PICKER_HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(help_height)])
        .split(full_area);

    let theme = &app.theme;
    let normal = Style::new().fg(theme.text);
    let accent = Style::new().fg(theme.accent);
    let warning = Style::new().fg(theme.warning);
    let border_style = Style::new().fg(theme.border);
    let current_theme = app.config.theme.clone();

    let items: Vec<ListItem> = app
        .available_themes
        .iter()
        .map(|name| {
            let is_active = current_theme
                .as_deref()
                .is_some_and(|current| current.eq_ignore_ascii_case(name));

            let marker = if is_active { "◉" } else { "○" };
            let marker_style = if is_active { accent } else { normal };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} "), marker_style),
                Span::styled(name.clone(), normal),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(border_style)
                .title(" Theme "),
        )
        .highlight_style(
            Style::new()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
                .bold(),
        )
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, vertical[0], &mut app.theme_state);

    let help = if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(warning)
    } else {
        let help_text = help_lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(help_text).style(accent)
    };

    frame.render_widget(help, vertical[1]);
}
