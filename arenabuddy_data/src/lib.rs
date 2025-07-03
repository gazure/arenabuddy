mod db;
mod directory;
mod error;
mod storage;

pub type Result<T, E = MatchDBError> = core::result::Result<T, E>;

pub use db::MatchDB;
pub use directory::DirectoryStorage;
pub use error::MatchDBError;
pub use storage::Storage;
