mod cards;
mod components;
mod debug_logs;
mod draft_details;
mod drafts;
mod error_logs;
mod match_details;
mod matches;
mod pages;
mod stats;
use chrono::{DateTime, Local, Utc};
use dioxus::prelude::*;
use pages::Route;

use crate::backend::theme::load_theme;
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn format_local_datetime(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%b %-d, %Y %-I:%M %p").to_string()
}

#[component]
pub fn App() -> Element {
    let theme = use_context_provider(|| Signal::new(load_theme()));

    // Mirror the theme onto <html> so the Tailwind `dark:` variant applies.
    use_effect(move || {
        let is_dark = theme().is_dark();
        document::eval(&format!(
            "document.documentElement.classList.toggle('dark', {is_dark});"
        ));
    });

    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
