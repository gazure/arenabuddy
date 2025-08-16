mod debug_logs;
mod error_logs;
mod match_details;
mod matches;
mod pages;
use dioxus::prelude::*;
use pages::Route;
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
