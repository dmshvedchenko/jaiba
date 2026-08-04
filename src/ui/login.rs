use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn draw_login(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(frame.area());

    let logo_area = vertical[1];
    let input_row = vertical[2];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .split(input_row);

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

    let logo_base = Paragraph::new(
        "
|                  ███                  ███                 |
|                 ███                    ███                |
|                ███   █              █   ███               |
|               ███  ██                ██  ███              |
|               █████                    █████              |
|           █    ██          █  █          ██    █          |
|         ███     ██         █  █         ██     ███        |
|         ████     ██   ██████████████   ██     ████        |
|           ████     ████████████████████     ████          |
|             ████  ██████████████████████  ████            |
|                ████████████████████████████               |
|                   ██████████████████████                  |
|                ████████████████████████████               |
|              ███  ██████████████████████  ███             |
|             ██   ██  ████████████████  ██   ██            |
|              █  ██    ██          ██    ██   █            |
|                  █   ██            ██    █                |
|                       █            █                      |
",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Rgb(119, 169, 192)));

    let logo_shell = Paragraph::new(
        "
|
|
|
|
|
|
|
|                         ██████████                        |
|                    ████████████████████                   |
|                   ██████████████████████                  |
|                   ██████████████████████                  |
|                   ██████████████████████                  |
|                     ██████████████████                    |
|                       ██████████████                      |
|                         ██████████                        |
",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Rgb(231, 97, 96)));

    frame.render_widget(logo_base, logo_area);
    // frame.render_widget(logo_shell, logo_area);

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
