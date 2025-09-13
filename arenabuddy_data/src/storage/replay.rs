use arenabuddy_core::player_log::replay::MatchReplay;

#[expect(async_fn_in_trait)]
pub trait ReplayStorage {
    /// # Errors
    ///
    /// Will return an error if the match replay cannot be written to the storage backend
    async fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()>;
}
