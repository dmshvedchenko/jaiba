use crossterm::event::KeyCode;

use crate::app::{App, Screen};
use crate::db::{calculate_warnings, save_database};

const FIELD_COUNT: usize = 6;
/// "Last modified" is computed from the database, not user-editable.
const LAST_MODIFIED_INDEX: usize = 5;

pub fn handle_edit_input(app: &mut App, key: KeyCode) {
    if app.editing_field {
        handle_field_input(app, key);
        return;
    }

    match key {
        KeyCode::Esc => close_edit(app),

        KeyCode::Enter => start_editing_field(app),

        KeyCode::Char('v') => {
            app.reveal_password = !app.reveal_password;
        }

        KeyCode::Down => move_selection(app, 1),
        KeyCode::Up => move_selection(app, -1),

        _ => {}
    }
}

fn handle_field_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.editing_field = false;
            app.field_buffer.clear();
        }

        KeyCode::Enter => commit_field(app),

        KeyCode::Char(c) => app.field_buffer.push(c),

        KeyCode::Backspace => {
            app.field_buffer.pop();
        }

        _ => {}
    }
}

fn move_selection(app: &mut App, delta: i32) {
    let current = app.edit_state.selected().unwrap_or(0) as i32;
    let next = (current + delta).rem_euclid(FIELD_COUNT as i32) as usize;
    app.edit_state.select(Some(next));
}

fn start_editing_field(app: &mut App) {
    let selected = app.edit_state.selected().unwrap_or(0);

    if selected == LAST_MODIFIED_INDEX {
        app.status = Some("Last modified is set automatically".to_string());
        return;
    }

    let Some(entry) = app.edit_entry.as_ref() else {
        return;
    };

    app.field_buffer = match selected {
        0 => entry.name.clone(),
        1 => entry.user.clone(),
        2 => entry.password.clone(),
        3 => entry.url.clone(),
        4 => entry.totp.clone(),
        _ => return,
    };

    app.editing_field = true;
    app.status = None;

    if selected == 2 {
        app.reveal_password = true;
    }
}

fn commit_field(app: &mut App) {
    let selected = app.edit_state.selected().unwrap_or(0);
    let value = std::mem::take(&mut app.field_buffer);

    if let Some(entry) = app.edit_entry.as_mut() {
        match selected {
            0 => entry.name = value,
            1 => entry.user = value,
            2 => entry.password = value,
            3 => entry.url = value,
            4 => entry.totp = value,
            _ => {}
        }
    }

    app.editing_field = false;
}

fn close_edit(app: &mut App) {
    save_edit(app);

    app.edit_target = None;
    app.editing_field = false;
    app.field_buffer.clear();
    app.reveal_password = false;
    app.edit_state.select(None);
    app.screen = Screen::Index;
}

fn save_edit(app: &mut App) {
    let Some(entry) = app.edit_entry.take() else {
        return;
    };

    let saved_idx = match app.edit_target {
        Some(idx) => {
            if let Some(slot) = app.entries.get_mut(idx) {
                *slot = entry;
            }
            Some(idx)
        }
        None => {
            if entry.name.trim().is_empty() {
                None
            } else {
                app.entries.push(entry);
                Some(app.entries.len() - 1)
            }
        }
    };

    calculate_warnings(&mut app.entries);
    app.refresh_filter();

    if let Some(idx) = saved_idx {
        persist_entry(app, idx);
    }
}

fn persist_entry(app: &mut App, idx: usize) {
    let (Some(db), Some(key), Some(path)) = (
        app.kdbx.as_mut(),
        app.db_key.as_ref(),
        app.config.default_database.as_ref(),
    ) else {
        app.status = Some("Not saved: database is locked".to_string());
        return;
    };

    let Some(target) = app.entries.get_mut(idx) else {
        return;
    };

    match save_database(path, key, db, std::slice::from_mut(target)) {
        Ok(()) => app.status = Some("Saved".to_string()),
        Err(err) => app.status = Some(format!("Failed to save: {err:#}")),
    }
}
