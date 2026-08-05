use crossterm::event::KeyCode;

use crate::app::{App, Screen};
use crate::config::save_config;
use crate::theme::load_theme;

pub fn handle_settings_input(app: &mut App, key: KeyCode) {
    let count = app.available_themes.len();

    match key {
        KeyCode::Down | KeyCode::Char('j') => {
            if count == 0 {
                return;
            }

            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let next = if selected + 1 >= count { 0 } else { selected + 1 };
            app.settings_state.select(Some(next));
        }

        KeyCode::Up | KeyCode::Char('k') => {
            if count == 0 {
                return;
            }

            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let prev = if selected == 0 { count - 1 } else { selected - 1 };
            app.settings_state.select(Some(prev));
        }

        KeyCode::Enter => apply_selected_theme(app),

        KeyCode::Esc => {
            app.status = None;
            app.screen = Screen::Index;
        }

        _ => {}
    }
}

fn apply_selected_theme(app: &mut App) {
    let Some(selected) = app.settings_state.selected() else {
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
