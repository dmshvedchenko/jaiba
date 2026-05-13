use gpui::*;
use std::collections::HashMap;

#[derive(Clone)]
struct KeyEntry {
    name: SharedString,
    user: SharedString,
    password: SharedString,
    totp: SharedString,
    url: SharedString,
}

#[derive(Clone)]
struct FieldCounts {
    name: HashMap<String, usize>,
    user: HashMap<String, usize>,
    password: HashMap<String, usize>,
    url: HashMap<String, usize>,
}

impl FieldCounts {
    fn from_entries(entries: &[KeyEntry]) -> Self {
        let mut name: HashMap<String, usize> = HashMap::new();
        let mut user: HashMap<String, usize> = HashMap::new();
        let mut password: HashMap<String, usize> = HashMap::new();
        let mut url: HashMap<String, usize> = HashMap::new();

        for e in entries {
            *name.entry(e.name.to_string()).or_insert(0) += 1;
            *user.entry(e.user.to_string()).or_insert(0) += 1;
            *password.entry(e.password.to_string()).or_insert(0) += 1;
            *url.entry(e.url.to_string()).or_insert(0) += 1;
        }
        Self {
            name,
            user,
            password,
            url,
        }
    }
}

struct KeyRow {
    entry: KeyEntry,
    counts: FieldCounts,
}

fn count_badge(count: usize) -> Div {
    if count > 1 {
        div()
            .ml_1()
            .px_1()
            .rounded_md()
            .bg(rgb(0xf38ba8))
            .text_color(rgb(0x1e1e2e))
            .text_sm()
            .child(format!("{}", count))
    } else {
        div()
    }
}

impl Render for KeyRow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let name_count = self
            .counts
            .name
            .get(self.entry.name.as_ref())
            .copied()
            .unwrap_or(1);
        let user_count = self
            .counts
            .user
            .get(self.entry.user.as_ref())
            .copied()
            .unwrap_or(1);
        let pass_count = self
            .counts
            .password
            .get(self.entry.password.as_ref())
            .copied()
            .unwrap_or(1);
        let url_count = self
            .counts
            .url
            .get(self.entry.url.as_ref())
            .copied()
            .unwrap_or(1);

        div()
            .flex()
            .flex_row()
            .w_full()
            .justify_between()
            .gap_4()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(rgb(0x313244))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .w_32()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(self.entry.name.clone())
                    .child(count_badge(name_count)),
            )
            .child(
                div()
                    .w_32()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(self.entry.user.clone())
                    .child(count_badge(user_count)),
            )
            .child(
                div()
                    .w_32()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(self.entry.password.clone())
                    .child(count_badge(pass_count)),
            )
            .child(div().w_24().child(self.entry.totp.clone()))
            .child(
                div()
                    .w_48()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(self.entry.url.clone())
                    .child(count_badge(url_count)),
            )
    }
}

struct KeyListing {
    all_rows: Vec<KeyEntry>,
    query: String,
    search_input: Entity<InputHandler>,
    focus_handle: FocusHandle,
}

struct InputHandler {
    value: String,
}

impl EventEmitter<String> for InputHandler {}

impl KeyListing {
    fn filtered_rows(&self) -> Vec<KeyEntry> {
        if self.query.is_empty() {
            return self.all_rows.clone();
        }

        let q = self.query.to_ascii_lowercase();

        self.all_rows
            .iter()
            .filter(|e| {
                e.name.as_ref().to_ascii_lowercase().contains(&q)
                    || e.user.as_ref().to_ascii_lowercase().contains(&q)
                    || e.url.as_ref().to_ascii_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }
}

impl Render for KeyListing {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered = self.filtered_rows();
        let counts = FieldCounts::from_entries(&filtered);

        // Collect into Vec first so cx borrow is fully released
        let items: Vec<_> = filtered
            .iter()
            .cloned()
            .map(|entry| {
                cx.new(|_| KeyRow {
                    entry,
                    counts: counts.clone(),
                })
            })
            .collect();

        let search_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .mb_3()
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(0x313244))
            .child(div().text_color(rgb(0x6c7086)).mr_2().child("🔍"))
            .child({
                let query = self.query.clone();
                if query.is_empty() {
                    div()
                        .flex_1()
                        .text_color(rgb(0x6c7086))
                        .child("Search entries...")
                } else {
                    div().flex_1().text_color(rgb(0xcdd6f4)).child(query)
                }
            });

        div()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .size_full()
            .track_focus(&self.focus_handle)
            .p_4()
            .gap_2()
            .text_color(rgb(0xcdd6f4))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let key = &event.keystroke;
                let has_modifier =
                    key.modifiers.alt || key.modifiers.control || key.modifiers.platform;
                if key.key == "backspace" {
                    this.query.pop();
                    cx.notify();
                } else if key.key == "escape" {
                    this.query.clear();
                    cx.notify();
                } else if key.key.len() == 1 && !has_modifier {
                    this.query.push_str(&key.key);
                    cx.notify();
                }
            }))
            .child(search_bar)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .font_weight(FontWeight::BOLD)
                    .border_b_1()
                    .border_color(rgb(0x45475a))
                    .child(div().w_32().child("Name"))
                    .child(div().w_32().child("User"))
                    .child(div().w_32().child("Password"))
                    .child(div().w_24().child("TOTP"))
                    .child(div().w_48().child("URL")),
            )
            .children(items)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|cx| {
                let search_input = cx.new(|_| InputHandler {
                    value: String::new(),
                });
                KeyListing {
                    all_rows: vec![
                        KeyEntry {
                            name: "Proton".into(),
                            user: "karl".into(),
                            password: "1234".into(),
                            totp: "381920".into(),
                            url: "github.com".into(),
                        },
                        KeyEntry {
                            name: "Proton".into(),
                            user: "karl@proton.me".into(),
                            password: "1234".into(),
                            totp: "918221".into(),
                            url: "proton.me".into(),
                        },
                        KeyEntry {
                            name: "Codeberg".into(),
                            user: "pandora".into(),
                            password: "123456".into(),
                            totp: "102991".into(),
                            url: "codeberg.org".into(),
                        },
                    ],
                    query: String::new(),
                    search_input,
                    focus_handle: cx.focus_handle(),
                }
            })
        })
        .unwrap();
    });
}
