use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding},
};

use crate::app::App;

pub fn draw_edit(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let Some(entry) = app.edit_entry.as_ref() else {
        return;
    };

    let name = entry.name.clone();
    let user = entry.user.clone();
    let url = entry.url.clone();
    let totp = entry.totp.clone();
    let last_modified = entry.date_last_modify.clone();
    let duplicate_user_count = entry.duplicate_user_count;
    let password_reuse_count = entry.password_reuse_count;
    let password_is_set = !entry.password.is_empty();

    let password_text = if !password_is_set {
        String::new()
    } else if app.reveal_password {
        entry.password.clone()
    } else {
        "•".repeat(entry.password.chars().count().max(8))
    };
    let is_new_entry = app.edit_target.is_none();

    let theme = &app.theme;
    let label_style = Style::new().fg(theme.header).bold();
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);
    let placeholder = Style::new().fg(theme.border).italic();

    let value_or_placeholder = |value: String| -> Line<'static> {
        if value.is_empty() {
            Line::from(Span::styled("(empty)", placeholder))
        } else {
            Line::from(Span::styled(value, normal))
        }
    };

    let mut user_spans = if user.is_empty() {
        vec![Span::styled("(empty)", placeholder)]
    } else {
        vec![Span::styled(user, normal)]
    };
    if duplicate_user_count > 1 {
        user_spans.push(Span::styled(format!(" [{duplicate_user_count}]"), warning));
    }

    let mut password_spans = if !password_is_set {
        vec![Span::styled("(empty)", placeholder)]
    } else {
        vec![Span::styled(password_text, normal)]
    };
    if password_reuse_count > 1 {
        password_spans.push(Span::styled(format!(" [{password_reuse_count}]"), warning));
    }
    if password_is_set && app.reveal_password {
        password_spans.push(Span::styled("  [visible]", warning));
    }

    let field = |label: &'static str, value: Line<'static>| -> ListItem<'static> {
        ListItem::new(vec![
            Line::from(Span::styled(label, label_style)),
            value,
            Line::from(""), // spacer between fields
        ])
    };

    let items = vec![
        field("Name", value_or_placeholder(name)),
        field("User", Line::from(user_spans)),
        field("Password", Line::from(password_spans)),
        field("URL", value_or_placeholder(url)),
        field("TOTP", value_or_placeholder(totp)),
        field("Last modified", value_or_placeholder(last_modified)),
    ];

    let title = if is_new_entry {
        " New Entry "
    } else {
        " Entry "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(app.theme.border))
                .title(title),
        )
        .highlight_style(
            Style::new()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        )
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, full_area, &mut app.edit_state);
}
