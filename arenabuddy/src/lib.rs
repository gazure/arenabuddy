#![allow(non_snake_case)]

mod app;
mod components;
mod debug_logs;
mod error_logs;
mod errors;
mod ingest;
mod match_details;
mod matches;
mod service;
mod state;

pub use errors::{Error, Result};
pub use service::launch_app;
