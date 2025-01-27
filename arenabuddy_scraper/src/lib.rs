#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_sign_loss)]

mod clean;
mod process;
mod scrape;

pub use clean::clean;
pub use process::process;
pub use scrape::scrape;
