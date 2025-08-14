mod functions;

#[cfg(feature = "server")]
mod app;

#[cfg(feature = "server")]
pub use app::launch_server;
pub use functions::*;
