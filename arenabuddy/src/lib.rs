#![allow(non_snake_case)]

mod app;
mod errors;
mod service;

pub use errors::{Error, Result};
pub use service::launch_app;
