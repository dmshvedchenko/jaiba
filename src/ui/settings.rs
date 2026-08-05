use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;
use crate::util::wrap_help_items;

const HELP_ITEMS: &[&str] = &["[↑↓] navigate", "[enter] apply theme", "[esc] back"];

pub fn draw_settings(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(HELP_ITEMS, help_width);
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

    let items: Vec<ListItem> = if app.available_themes.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "no themes found in ~/.config/jaiba/themes",
            warning,
        )))]
    } else {
        app.available_themes
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
            .collect()
    };

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
