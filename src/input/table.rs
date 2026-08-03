use crossterm::event::KeyCode;

use crate::app::App;
use crate::input::command::{handle_command_input, preview_entry};

pub fn handle_table_input(app: &mut App, key: KeyCode) {
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
