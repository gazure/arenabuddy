use arenabuddy_core::replay::MatchReplay;

pub trait Storage {
    /// # Errors
    ///
    /// Will return an error if the match replay cannot be written to the storage backend
    fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()>;
}
