mod app;
mod components;
mod debug_logs;
mod error_logs;
mod errors;
#[cfg(feature = "server")]
mod ingest;
mod match_details;
mod matches;
mod service;
mod state;

pub use errors::{Error, Result};
#[cfg(feature = "server")]
pub use service::launch_server;

#[cfg(not(feature = "server"))]
pub fn launch_frontend() {
    use crate::app::App;
    dioxus::LaunchBuilder::desktop().launch(App);
}
