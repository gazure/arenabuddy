#![expect(clippy::too_many_lines)]
#![expect(clippy::needless_pass_by_value)]

mod app;
mod components;
mod debug_logs;
mod error_logs;
mod match_details;
mod matches;
mod state;

use app::App;


fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}
