pub(crate) mod grpc_writer;
pub(crate) mod ingest;
mod launch;
mod service;

pub use launch::launch;
pub type Service = service::AppService<arenabuddy_data::MatchDB>;
