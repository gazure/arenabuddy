mod db;
mod error;

pub type Result<T, E = MatchDBError> = core::result::Result<T, E>;

pub use db::MatchDB;
pub use error::MatchDBError;
