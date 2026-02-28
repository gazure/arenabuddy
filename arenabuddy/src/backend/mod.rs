pub(crate) mod auth;
pub(crate) mod grpc_writer;
pub(crate) mod ingest;
mod launch;
pub(crate) mod paths;
mod service;
pub(crate) mod sync;

pub use auth::{SharedAuthState, new_shared_auth_state};
pub use launch::{BackgroundRuntime, launch};
pub type Service = service::AppService<arenabuddy_data::MatchDB>;
