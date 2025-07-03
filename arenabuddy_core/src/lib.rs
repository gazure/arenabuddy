#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod cards;
pub mod display;
pub mod errors;
pub mod events;
pub mod models;
pub mod mtga_events;
pub mod processor;
pub mod replay;

pub use errors::{Error, Result};
