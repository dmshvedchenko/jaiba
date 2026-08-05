use std::time::Duration;

use crossterm::event::KeyCode;

use crate::app::{App, Screen};
use crate::config::save_config;
use crate::theme::load_theme;
use crate::util::expand_tilde;

pub const ROW_COUNT: usize = 4;

pub const DATABASE_ROW: usize = 0;
pub const AUTO_LOCK_ROW: usize = 1;
pub const CLIPBOARD_TIMEOUT_ROW: usize = 2;
pub const THEME_ROW: usize = 3;

pub fn handle_settings_input(app: &mut App, key: KeyCode) {
    if app.choosing_theme {
        handle_theme_picker_input(app, key);
        return;
    }

    if app.editing_field {
        handle_field_input(app, key);
        return;
    }

    match key {
        KeyCode::Down | KeyCode::Char('j') => {
            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let next = if selected + 1 >= ROW_COUNT { 0 } else { selected + 1 };
            app.settings_state.select(Some(next));
        }

        KeyCode::Up | KeyCode::Char('k') => {
            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let prev = if selected == 0 { ROW_COUNT - 1 } else { selected - 1 };
            app.settings_state.select(Some(prev));
        }

        KeyCode::Enter => activate_selected(app),

        KeyCode::Esc => {
            app.status = None;
            app.screen = Screen::Index;
        }

        _ => {}
    }
}

fn activate_selected(app: &mut App) {
    let Some(selected) = app.settings_state.selected() else {
        return;
    };

    match selected {
        DATABASE_ROW => start_editing_database(app),
        AUTO_LOCK_ROW => start_editing_auto_lock(app),
        CLIPBOARD_TIMEOUT_ROW => start_editing_clipboard_timeout(app),
        THEME_ROW => start_choosing_theme(app),
        _ => {}
    }
}

fn start_editing_database(app: &mut App) {
    app.field_buffer = app
        .config
        .default_database
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    app.editing_field = true;
    app.status = None;
}

fn start_editing_auto_lock(app: &mut App) {
    app.field_buffer = app.config.auto_lock.as_secs().to_string();
    app.editing_field = true;
    app.status = None;
}

fn start_editing_clipboard_timeout(app: &mut App) {
    app.field_buffer = app.config.clipboard_timeout.as_secs().to_string();
    app.editing_field = true;
    app.status = None;
}

fn handle_field_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.editing_field = false;
            app.field_buffer.clear();
            app.status = None;
        }

        KeyCode::Enter => commit_field(app),

        KeyCode::Char(c) => app.field_buffer.push(c),

        KeyCode::Backspace => {
            app.field_buffer.pop();
        }

        _ => {}
    }
}

fn commit_field(app: &mut App) {
    let selected = app.settings_state.selected().unwrap_or(0);
    let value = std::mem::take(&mut app.field_buffer);
    app.editing_field = false;

    match selected {
        DATABASE_ROW => {
            app.config.default_database = if value.trim().is_empty() {
                None
            } else {
                Some(expand_tilde(value.trim()))
            };
        }

        AUTO_LOCK_ROW => match parse_seconds(&value) {
            Ok(secs) => app.config.auto_lock = Duration::from_secs(secs),
            Err(err) => {
                app.status = Some(err);
                return;
            }
        },

        CLIPBOARD_TIMEOUT_ROW => match parse_seconds(&value) {
            Ok(secs) => app.config.clipboard_timeout = Duration::from_secs(secs),
            Err(err) => {
                app.status = Some(err);
                return;
            }
        },

        _ => return,
    }

    app.status = Some(match save_config(&app.config) {
        Ok(()) => "saved".to_string(),
        Err(err) => format!("couldn't save config: {err}"),
    });
}

fn parse_seconds(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("\"{}\" isn't a whole number of seconds", value.trim()))
}

fn start_choosing_theme(app: &mut App) {
    if app.available_themes.is_empty() {
        app.status = Some("no themes found in ~/.config/jaiba/themes".to_string());
        return;
    }

    let current = app.config.theme.as_deref();
    let start = app
        .available_themes
        .iter()
        .position(|name| current.is_some_and(|c| c.eq_ignore_ascii_case(name)))
        .unwrap_or(0);

    app.theme_state.select(Some(start));
    app.choosing_theme = true;
    app.status = None;
}

fn handle_theme_picker_input(app: &mut App, key: KeyCode) {
    let count = app.available_themes.len();

    match key {
        KeyCode::Down | KeyCode::Char('j') => {
            if count == 0 {
                return;
            }
            let selected = app.theme_state.selected().unwrap_or(0);
            let next = if selected + 1 >= count { 0 } else { selected + 1 };
            app.theme_state.select(Some(next));
        }

        KeyCode::Up | KeyCode::Char('k') => {
            if count == 0 {
                return;
            }
            let selected = app.theme_state.selected().unwrap_or(0);
            let prev = if selected == 0 { count - 1 } else { selected - 1 };
            app.theme_state.select(Some(prev));
        }

        KeyCode::Enter => {
            apply_selected_theme(app);
            app.choosing_theme = false;
        }

        KeyCode::Esc => {
            app.choosing_theme = false;
            app.status = None;
        }

        _ => {}
    }
}

fn apply_selected_theme(app: &mut App) {
    let Some(selected) = app.theme_state.selected() else {
        return;
    };
    let Some(name) = app.available_themes.get(selected).cloned() else {
        return;
    };

    match load_theme(Some(&name)) {
        Ok(theme) => {
            app.theme = theme;
            app.config.theme = Some(name.clone());

            app.status = Some(match save_config(&app.config) {
                Ok(()) => format!("theme set to {name}"),
                Err(err) => format!("theme applied, but couldn't save config: {err}"),
            });
        }

        Err(err) => {
            app.status = Some(format!("couldn't load theme \"{name}\": {err}"));
        }
    }
}
