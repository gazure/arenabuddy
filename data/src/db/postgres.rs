#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::similar_names)]
use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    models::{
        ArenaId, Deck, Draft, DraftPack, Format, MTGADraft, MTGAMatch, MTGAMatchBuilder, MatchResult,
        MatchResultBuilder, Mulligan,
    },
    player_log::{
        ingest::{DraftWriter, ReplayWriter},
        replay::MatchReplay,
    },
};
use chrono::{NaiveDateTime, Utc};
use postgresql_embedded::PostgreSQL;
use sqlx::{PgPool, Postgres, Transaction, types::Uuid};
use tracingx::{debug, error, info};

use crate::{Error, Result, db::repository::ArenabuddyRepository};

#[derive(Debug, Clone)]
pub struct PostgresMatchDB {
    pool: PgPool,
    _db: Option<Arc<PostgreSQL>>,
    cards: Arc<CardsDatabase>,
}

impl PostgresMatchDB {
    pub async fn new(url: Option<&str>, cards: Arc<CardsDatabase>) -> Result<Self> {
        if let Some(url) = url {
            let pool = PgPool::connect(url).await?;
            Ok(Self { pool, _db: None, cards })
        } else {
            // Configure persistent embedded PostgreSQL
            let db_path = Self::get_embedded_db_path()?;
            info!("Using embedded PostgreSQL at: {}", db_path.display());

            std::fs::create_dir_all(&db_path)?;

            let mut settings = postgresql_embedded::Settings::default();
            settings.installation_dir = db_path.join("postgres_install");
            settings.data_dir = db_path.join("data");
            settings.password_file = db_path.join("password.txt");
            settings.temporary = false; // Make database persistent across restarts

            // Use a fixed password for persistent database
            // This is safe because the embedded DB only listens on localhost
            settings.password = "arenabuddy_local".to_string();

            let mut db = PostgreSQL::new(settings);
            db.setup().await?;

            // Try to start the database, handling the case where it might already be running
            // or there's a stale PID file from an unclean shutdown
            match db.start().await {
                Ok(()) => {
                    info!("PostgreSQL started successfully");
                }
                Err(e) => {
                    info!(
                        "First start attempt failed ({}), trying to stop any existing instance...",
                        e
                    );
                    // Try to stop any running instance first
                    let _ = db.stop().await;

                    // Clean up stale PID file if it exists
                    let pid_file = db.settings().data_dir.join("postmaster.pid");
                    if pid_file.exists() {
                        info!("Removing stale PID file: {}", pid_file.display());
                        let _ = std::fs::remove_file(&pid_file);
                    }

                    // Try starting again
                    db.start().await?;
                    info!("PostgreSQL started successfully after cleanup");
                }
            }

            // Create database only if it doesn't exist yet
            // The database will persist between app restarts
            let _ = db.create_database("arenabuddy").await;

            let pool = PgPool::connect(&db.settings().url("arenabuddy")).await?;
            Ok(Self {
                pool,
                _db: Some(Arc::new(db)),
                cards,
            })
        }
    }

    fn get_embedded_db_path() -> Result<std::path::PathBuf> {
        // Use platform-appropriate application data directory
        let home = std::env::home_dir().ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        })?;

        let db_path = match std::env::consts::OS {
            "macos" => home.join("Library/Application Support/com.gazure.dev.arenabuddy.app/postgres"),
            "windows" => home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app/postgres"),
            "linux" => home.join(".local/share/com.gazure.dev.arenabuddy.app/postgres"),
            os => {
                return Err(Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    format!("Unsupported OS: {os}"),
                )));
            }
        };

        Ok(db_path)
    }

    pub async fn initialize(&self) -> Result<()> {
        sqlx::migrate!("./migrations/postgres").run(&self.pool).await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_match(match_id: &Uuid, mtga_match: &MTGAMatch, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO match
            (id, controller_seat_id, controller_player_name, opponent_player_name, created_at)
            VALUES ($1, $2, $3, $4, $5) ON CONFLICT(id) DO NOTHING"#,
            match_id,
            mtga_match.controller_seat_id(),
            mtga_match.controller_player_name(),
            mtga_match.opponent_player_name(),
            mtga_match.created_at().naive_utc()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_deck(match_id: &Uuid, deck: &Deck, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        let deck_string = serde_json::to_string(deck.mainboard())?;
        let sideboard_string = serde_json::to_string(deck.sideboard())?;

        sqlx::query!(
            r#"INSERT INTO deck
            (match_id, game_number, deck_cards, sideboard_cards)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (match_id, game_number)
            DO UPDATE SET deck_cards = excluded.deck_cards, sideboard_cards = excluded.sideboard_cards"#,
            match_id,
            deck.game_number(),
            deck_string,
            sideboard_string
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_mulligan_info(
        match_id: &Uuid,
        mulligan_info: &Mulligan,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO mulligan (match_id, game_number, number_to_keep, hand, play_draw, opponent_identity, decision)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (match_id, game_number, number_to_keep)
             DO UPDATE SET hand = excluded.hand, play_draw = excluded.play_draw, opponent_identity = excluded.opponent_identity, decision = excluded.decision"#,
            match_id,
            mulligan_info.game_number(),
            mulligan_info.number_to_keep(),
            mulligan_info.hand(),
            mulligan_info.play_draw(),
            mulligan_info.opponent_identity(),
            mulligan_info.decision()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_match_result(
        match_id: &Uuid,
        match_result: &MatchResult,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO match_result (match_id, game_number, winning_team_id, result_scope)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (match_id, game_number)
             DO UPDATE SET winning_team_id = excluded.winning_team_id, result_scope = excluded.result_scope"#,
            match_id,
            match_result.game_number(),
            match_result.winning_team_id(),
            match_result.result_scope()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_opponent_deck(
        match_id: &Uuid,
        opponent_cards: &[ArenaId],
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let mut unique_cards = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for card in opponent_cards {
            if seen.insert(card) {
                unique_cards.push(*card);
            }
        }
        let opponent_cards_string = serde_json::to_string(&unique_cards)?;

        sqlx::query!(
            r#"INSERT INTO opponent_deck
            (match_id, cards)
            VALUES ($1, $2)
            ON CONFLICT (match_id)
            DO UPDATE SET cards = excluded.cards"#,
            match_id,
            opponent_cards_string
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn get_match_results(&self, match_id: &str) -> Result<Vec<MatchResult>> {
        let match_id = Uuid::parse_str(match_id)?;
        let results = sqlx::query!(
            "SELECT game_number, winning_team_id, result_scope FROM match_result WHERE match_id = $1 AND game_number > 0",
            match_id
        )
        .fetch_all(&self.pool)
        .await?;

        let match_results = results
            .into_iter()
            .map(|row| {
                MatchResult::new(
                    match_id.to_string(),
                    row.game_number,
                    row.winning_team_id,
                    row.result_scope,
                )
            })
            .collect();

        Ok(match_results)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn get_decklists(&self, match_id: &str) -> Result<Vec<Deck>> {
        let match_id = Uuid::parse_str(match_id)?;
        let results = sqlx::query!(
            "SELECT game_number, deck_cards, sideboard_cards FROM deck WHERE match_id = $1",
            match_id
        )
        .fetch_all(&self.pool)
        .await?;

        let decks = results
            .into_iter()
            .map(|row| {
                Deck::from_raw(
                    "Found Deck".to_string(),
                    row.game_number,
                    &row.deck_cards,
                    &row.sideboard_cards,
                )
            })
            .collect();

        Ok(decks)
    }

    async fn do_get_opponent_deck(&self, match_id: &str) -> Result<Deck> {
        let match_id = Uuid::parse_str(match_id)?;
        let result = sqlx::query!("SELECT cards FROM opponent_deck WHERE match_id = $1", match_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Deck::from_raw("Opponent_deck".to_string(), 0, &result.cards, ""))
    }

    /// # Errors
    // will return an error if the database cannot be contacted for some reason
    async fn get_mulligans(&self, match_id: &str) -> Result<Vec<Mulligan>> {
        let match_id = Uuid::parse_str(match_id)?;
        let results = sqlx::query!(
            "SELECT game_number, number_to_keep, hand, play_draw, opponent_identity, decision FROM mulligan WHERE match_id = $1",
            match_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mulligans = results
            .into_iter()
            .map(|row| {
                Mulligan::new(
                    match_id.to_string(),
                    row.game_number,
                    row.number_to_keep,
                    row.hand,
                    row.play_draw,
                    row.opponent_identity,
                    row.decision,
                )
            })
            .collect();

        Ok(mulligans)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn get_matches(&self) -> Result<Vec<MTGAMatch>> {
        let results = sqlx::query!(
            "SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at FROM match"
        )
        .fetch_all(&self.pool)
        .await?;

        let matches: Vec<_> = results
            .into_iter()
            .map(|row| {
                MTGAMatch::new_with_timestamp(
                    row.id,
                    row.controller_seat_id,
                    row.controller_player_name,
                    row.opponent_player_name,
                    row.created_at
                        .map(|naive: NaiveDateTime| naive.and_utc())
                        .unwrap_or_default(),
                )
            })
            .collect();

        info!("found {} matches", matches.len());
        Ok(matches)
    }

    /// # Errors
    ///
    /// Errors if underlying data is malformed
    async fn retrieve_match(&self, match_id: &str) -> Result<(MTGAMatch, Option<MatchResult>)> {
        info!("Getting match details for match_id: {}", match_id);
        let match_id = Uuid::parse_str(match_id)?;

        let result = sqlx::query!(
            r#"
            SELECT
                m.id, m.controller_player_name, m.opponent_player_name, mr.winning_team_id, m.controller_seat_id, m.created_at
            FROM match m JOIN match_result mr ON m.id = mr.match_id
            WHERE m.id = $1 AND mr.result_scope = 'MatchScope_Match' LIMIT 1
            "#,
            match_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok((
                MTGAMatch::new_with_timestamp(
                    row.id,
                    row.controller_seat_id,
                    row.controller_player_name,
                    row.opponent_player_name,
                    row.created_at
                        .map(|naive: NaiveDateTime| naive.and_utc())
                        .unwrap_or_default(),
                ),
                Some(MatchResult::new_match_result(row.id, row.winning_team_id)),
            ))
        } else {
            error!("Error getting match details for match_id: {}", match_id);
            Ok((MTGAMatch::default(), None))
        }
    }

    /// # Errors
    ///
    /// will return an error if if the match replay cannot be written to the database due to missing data
    /// or connection error
    async fn write(&self, match_replay: &MatchReplay) -> crate::Result<()> {
        info!("Writing match replay to database");
        let controller_seat_id = match_replay.get_controller_seat_id();
        let match_id = Uuid::parse_str(&match_replay.match_id)?;
        let (controller_name, opponent_name) = match_replay.get_player_names(controller_seat_id)?;
        let event_start = match_replay.match_start_time().unwrap_or(Utc::now());

        let mtga_match = MTGAMatchBuilder::default()
            .id(match_id.to_string())
            .controller_seat_id(controller_seat_id)
            .controller_player_name(controller_name)
            .opponent_player_name(opponent_name)
            .created_at(event_start)
            .build()?;

        let mut tx = self.pool.begin().await.map_err(Error::from)?;

        Self::insert_match(&match_id, &mtga_match, &mut tx).await?;

        let decklists = match_replay.get_decklists()?;
        for deck in &decklists {
            Self::insert_deck(&match_id, deck, &mut tx).await?;
        }

        let mulligan_infos = match_replay.get_mulligan_infos(&self.cards)?;
        for mulligan_info in &mulligan_infos {
            Self::insert_mulligan_info(&match_id, mulligan_info, &mut tx).await?;
        }

        // not too keen on this data model
        let match_results = match_replay.get_match_results()?;
        debug!("{:?}", match_results);
        for (i, result) in match_results.result_list.iter().enumerate() {
            let game_number = if result.scope == "MatchScope_Game" {
                i32::try_from(i + 1).unwrap_or(0)
            } else {
                0
            };

            let match_result = MatchResultBuilder::default()
                .match_id(match_id.to_string())
                .game_number(game_number)
                .winning_team_id(result.winning_team_id)
                .result_scope(result.scope.clone())
                .build()?;

            Self::insert_match_result(&match_id, &match_result, &mut tx).await?;
        }

        Self::insert_opponent_deck(&match_id, &match_replay.get_opponent_cards(), &mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn insert_draft(draft: &Draft, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO draft(id, set_code, draft_format, status, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id)
            DO UPDATE SET set_code = excluded.set_code, draft_format = excluded.draft_format, status = excluded.status, created_at = excluded.created_at
            "#,
            draft.id(),
            draft.set_code(),
            draft.format().to_string(),
            draft.status(),
            draft.created_at().naive_utc()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn insert_draft_pack(
        draft_id: &Uuid,
        pack_number: i32,
        pick_number: i32,
        selection_num: i32,
        picked_card_id: ArenaId,
        cards: &[ArenaId],
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let cards_json = serde_json::to_string(cards)?;

        sqlx::query!(
            r#"
            INSERT INTO draft_pack(draft_id, pack_number, pick_number, selection_number, cards, card_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (draft_id, pack_number, pick_number, selection_number)
            DO UPDATE SET cards = excluded.cards, card_id = excluded.card_id
            "#,
            draft_id,
            pack_number,
            pick_number,
            selection_num,
            cards_json,
            picked_card_id.inner(),
            Utc::now().naive_utc()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn write_draft(&mut self, draft: &MTGADraft) -> Result<()> {
        info!("Writing draft to database!");

        let mut tx = self.pool.begin().await.map_err(Error::from)?;

        Self::insert_draft(draft.draft(), &mut tx).await?;

        for pack in draft.packs() {
            Self::insert_draft_pack(
                &draft.draft().id(),
                pack.pack_number().into(),
                pack.pick_number().into(),
                pack.selection_number().into(),
                pack.picked_card(),
                pack.cards(),
                &mut tx,
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn do_list_drafts(&self) -> Result<Vec<Draft>> {
        let result = sqlx::query!(
            r#"
                SELECT id, set_code, draft_format, status, created_at
                FROM draft
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result
            .into_iter()
            .map(|row| {
                Draft::new(
                    row.id,
                    row.set_code,
                    row.status.map(Format::from_str).unwrap_or_default(),
                    row.draft_format.unwrap_or_default(),
                )
                .with_created_at(row.created_at.unwrap_or_default().and_utc())
            })
            .collect())
    }

    async fn do_get_draft(&self, draft_id: &str) -> Result<MTGADraft> {
        let draft_id = Uuid::parse_str(draft_id)?;

        // Get the draft details
        let draft_row = sqlx::query!(
            r#"
            SELECT id, set_code, draft_format, status, created_at
            FROM draft
            WHERE id = $1
            "#,
            draft_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Get all packs for this draft
        let pack_rows = sqlx::query!(
            r#"
            SELECT id, pack_number, pick_number, selection_number, cards, card_id
            FROM draft_pack
            WHERE draft_id = $1
            ORDER BY pack_number, pick_number, selection_number
            "#,
            draft_id
        )
        .fetch_all(&self.pool)
        .await?;

        // Construct the Draft model
        let draft = Draft::new(
            draft_row.id,
            draft_row.set_code,
            draft_row.draft_format.map(Format::from_str).unwrap_or_default(),
            draft_row.status.unwrap_or_default(),
        )
        .with_created_at(draft_row.created_at.unwrap_or_default().and_utc());

        // Construct the packs
        let mut packs = Vec::new();
        for row in pack_rows {
            let cards: Vec<ArenaId> = serde_json::from_str(&row.cards)?;
            let pack = DraftPack::new(
                draft.id(),
                row.pack_number as u8,
                row.pick_number as u8,
                row.selection_number as u8,
                row.card_id.into(),
                cards,
            )
            .with_id(row.id as u64);
            packs.push(pack);
        }

        Ok(MTGADraft::new(draft, packs))
    }

    async fn do_upsert_match_data(
        &self,
        mtga_match: &MTGAMatch,
        decks: &[Deck],
        mulligans: &[Mulligan],
        results: &[MatchResult],
        opponent_cards: &[ArenaId],
    ) -> Result<()> {
        info!("Upserting match data for match_id: {}", mtga_match.id());
        let match_id = Uuid::parse_str(mtga_match.id())?;

        let mut tx = self.pool.begin().await.map_err(Error::from)?;

        Self::insert_match(&match_id, mtga_match, &mut tx).await?;

        for deck in decks {
            Self::insert_deck(&match_id, deck, &mut tx).await?;
        }

        for mulligan in mulligans {
            Self::insert_mulligan_info(&match_id, mulligan, &mut tx).await?;
        }

        for result in results {
            Self::insert_match_result(&match_id, result, &mut tx).await?;
        }

        Self::insert_opponent_deck(&match_id, opponent_cards, &mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn do_delete_match(&self, match_id: &str) -> Result<()> {
        info!("Deleting match: {}", match_id);
        let match_id = Uuid::parse_str(match_id)?;

        sqlx::query("DELETE FROM match WHERE id = $1")
            .bind(match_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ArenabuddyRepository for PostgresMatchDB {
    async fn init(&self) -> Result<()> {
        self.initialize().await
    }

    async fn write_replay(&self, replay: &MatchReplay) -> Result<()> {
        self.write(replay).await
    }

    async fn get_match(&self, match_id: &str) -> Result<(MTGAMatch, Option<MatchResult>)> {
        self.retrieve_match(match_id).await
    }

    async fn list_matches(&self) -> Result<Vec<MTGAMatch>> {
        self.get_matches().await
    }

    async fn list_decklists(&self, match_id: &str) -> Result<Vec<Deck>> {
        self.get_decklists(match_id).await
    }

    async fn list_mulligans(&self, match_id: &str) -> Result<Vec<Mulligan>> {
        self.get_mulligans(match_id).await
    }

    async fn list_match_results(&self, match_id: &str) -> Result<Vec<MatchResult>> {
        self.get_match_results(match_id).await
    }

    async fn get_opponent_deck(&self, match_id: &str) -> Result<Deck> {
        self.do_get_opponent_deck(match_id).await
    }

    async fn list_drafts(&self) -> Result<Vec<Draft>> {
        self.do_list_drafts().await
    }

    async fn get_draft(&self, draft_id: &str) -> Result<MTGADraft> {
        self.do_get_draft(draft_id).await
    }

    async fn upsert_match_data(
        &self,
        mtga_match: &MTGAMatch,
        decks: &[Deck],
        mulligans: &[Mulligan],
        results: &[MatchResult],
        opponent_cards: &[ArenaId],
    ) -> Result<()> {
        self.do_upsert_match_data(mtga_match, decks, mulligans, results, opponent_cards)
            .await
    }

    async fn delete_match(&self, match_id: &str) -> Result<()> {
        self.do_delete_match(match_id).await
    }
}

#[async_trait::async_trait]
impl ReplayWriter for PostgresMatchDB {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        self.write_replay(replay).await.map_err(|e| {
            error!("Failed to write replay: {}", e);
            arenabuddy_core::Error::Io(e.to_string())
        })
    }
}

#[async_trait::async_trait]
impl DraftWriter for PostgresMatchDB {
    async fn write(&mut self, draft: &MTGADraft) -> arenabuddy_core::Result<()> {
        self.write_draft(draft).await.map_err(|e| {
            error!("Failed to write draft: {}", e);
            arenabuddy_core::Error::Io(e.to_string())
        })
    }
}
