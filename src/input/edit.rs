use crossterm::event::KeyCode;

use crate::app::{App, Screen};

const FIELD_COUNT: usize = 6;

pub fn handle_edit_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => close_edit(app),

        KeyCode::Char('v') => {
            app.reveal_password = !app.reveal_password;
        }

        KeyCode::Down => move_selection(app, 1),
        KeyCode::Up => move_selection(app, -1),

        _ => {}
    }
}

fn move_selection(app: &mut App, delta: i32) {
    let current = app.edit_state.selected().unwrap_or(0) as i32;
    let next = (current + delta).rem_euclid(FIELD_COUNT as i32) as usize;
    app.edit_state.select(Some(next));
}

fn close_edit(app: &mut App) {
    app.edit_entry = None;
    app.edit_target = None;
    app.reveal_password = false;
    app.edit_state.select(None);
    app.screen = Screen::Index;
}
