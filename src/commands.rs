#[derive(Clone, Copy)]
enum Command {
    CopyUser,
    CopyPassword,
    CopyTotp,
    CopyUrl,
    AddEntry,
    Quit,
    Settings,
}

enum CommandMatch {
    Exact(Command),
    Prefix,
    Invalid,
}

const COMMANDS: &[(&str, Command)] = &[
    ("u", Command::CopyUser),
    ("p", Command::CopyPassword),
    ("t", Command::CopyTotp),
    ("r", Command::CopyUrl),
    ("a", Command::AddEntry),
    ("q", Command::Quit),
    ("s", Command::Settings),
];

fn match_command(buffer: &str) -> CommandMatch {
    let mut exact = None;
    let mut is_prefix = false;

    for (name, cmd) in COMMANDS {
        if *name == buffer {
            exact = Some(*cmd);
        } else if name.starts_with(buffer) {
            is_prefix = true;
        }
    }

    match (exact, is_prefix) {
        (Some(cmd), _) => CommandMatch::Exact(cmd),
        (None, true) => CommandMatch::Prefix,
        (None, false) => CommandMatch::Invalid,
    }
}

fn handle_command_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.command_mode = false;
            app.command_buffer.clear();
        }

        KeyCode::Backspace => {
            app.command_buffer.pop();

            if app.command_buffer.is_empty() {
                app.command_mode = false;
            }
        }

        KeyCode::Char(c) => {
            app.command_buffer.push(c);

            match match_command(&app.command_buffer) {
                CommandMatch::Exact(cmd) => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                    execute_command(app, cmd);
                }

                CommandMatch::Prefix => {
                    // Keep collecting characters.
                }

                CommandMatch::Invalid => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                }
            }
        }

        _ => {}
    }
}

fn execute_command(app: &mut App, cmd: Command) {
    match cmd {
        Command::CopyUser => cp_user(app),
        Command::CopyPassword => cp_password(app),
        Command::CopyTotp => cp_totp(app),
        Command::CopyUrl => cp_url(app),
        Command::AddEntry => add_entry(app),
        Command::Quit => app.should_quit = true,
        Command::Settings => open_settings(app),
    }
}
