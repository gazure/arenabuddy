mod functions;

#[cfg(feature = "server")]
mod app;

#[cfg(feature = "server")]
pub use app::{Service, launch_server};
pub use functions::*;
