#![deny(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

mod app;
mod components;
mod debug_logs;
mod error_logs;
mod match_details;
mod matches;
mod state;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! { <App /> }
    });
}
