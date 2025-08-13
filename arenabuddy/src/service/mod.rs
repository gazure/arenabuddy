mod functions;

#[cfg(feature = "server")]
mod app;

#[cfg(feature = "server")]
pub use app::{AppService, server_start};
pub use functions::*;
