use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

#[component]
fn app() -> Element {
    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; font-family: system-ui, -apple-system, sans-serif; background: #1a1a2e; color: #e0e0e0;",
            h1 {
                style: "font-size: 3rem; margin-bottom: 0.5rem; color: #ffffff;",
                "Arena Buddy"
            }
            p {
                style: "font-size: 1.25rem; margin-bottom: 2rem; color: #a0a0b0;",
                "A companion app for Magic: The Gathering Arena"
            }
            a {
                href: "https://github.com/gazure/monorepo/releases",
                target: "_blank",
                rel: "noopener noreferrer",
                style: "padding: 0.75rem 1.5rem; background: #4a90d9; color: #ffffff; text-decoration: none; border-radius: 0.5rem; font-size: 1.1rem; transition: background 0.2s;",
                "Download the latest release"
            }
        }
    }
}
