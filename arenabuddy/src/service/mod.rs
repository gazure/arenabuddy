mod functions;

#[cfg(feature = "server")]
mod app;

#[cfg(feature = "server")]
pub use app::{AppService, launch_server};
pub use functions::*;
