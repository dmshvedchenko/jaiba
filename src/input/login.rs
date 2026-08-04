use std::time::Instant;

use crossterm::event::KeyCode;

use crate::app::App;
use crate::db::{calculate_warnings, unlock_database};

pub fn handle_login_input(app: &mut App, key: KeyCode) {
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
            app.screen = crate::app::Screen::Index;
        }
        Err(err) => {
            app.password.clear();
            app.login_error = Some(format!("{err}"));
        }
    }
}
