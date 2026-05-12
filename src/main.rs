use gpui::*;

#[derive(Clone)]
struct KeyEntry {
    name: SharedString,
    user: SharedString,
    password: SharedString,
    totp: SharedString,
    url: SharedString,
}

struct KeyRow {
    entry: KeyEntry,
}

impl Render for KeyRow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(div().w_32().child(self.entry.name.clone()))
            .child(div().w_32().child(self.entry.user.clone()))
            .child(div().w_32().child(self.entry.password.clone()))
            .child(div().w_24().child(self.entry.totp.clone()))
            .child(div().w_48().child(self.entry.url.clone()))
    }
}

struct KeyListing {
    rows: Vec<KeyEntry>,
}

impl Render for KeyListing {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self
            .rows
            .iter()
            .cloned()
            .map(|entry| cx.new(|_| KeyRow { entry }));

        div()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .size_full()
            .p_4()
            .gap_2()
            .text_color(rgb(0xcdd6f4))
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
            cx.new(|_| KeyListing {
                rows: vec![
                    KeyEntry {
                        name: "GitHub".into(),
                        user: "karl".into(),
                        password: "••••••••".into(),
                        totp: "381920".into(),
                        url: "github.com".into(),
                    },
                    KeyEntry {
                        name: "Proton".into(),
                        user: "karl@proton.me".into(),
                        password: "••••••••".into(),
                        totp: "918221".into(),
                        url: "proton.me".into(),
                    },
                    KeyEntry {
                        name: "Codeberg".into(),
                        user: "pandora".into(),
                        password: "••••••••".into(),
                        totp: "102991".into(),
                        url: "codeberg.org".into(),
                    },
                ],
            })
        })
        .unwrap();
    });
}
