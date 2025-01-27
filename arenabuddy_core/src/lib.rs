#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod cards;
pub mod match_insights;
pub mod models;
pub mod mtga_events;
pub mod processor;
pub mod replay;
pub mod storage_backends;
