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
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn format_local_datetime(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%b %-d, %Y %-I:%M %p").to_string()
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
