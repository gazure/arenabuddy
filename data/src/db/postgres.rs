#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::similar_names)]
#![expect(clippy::field_reassign_with_default)]

use arenabuddy_core::{
    cards::CardsDatabase,
    models::{
        ArenaId, Deck, Draft, DraftPack, Format, GameEventLog, MTGADraft, MTGAMatch, MTGAMatchBuilder, MatchResult,
        MatchResultBuilder, Mulligan,
    },
    player_log::{
        ingest::{DraftWriter, ReplayWriter},
        replay::MatchReplay,
    },
};
use chrono::{DateTime, NaiveDateTime, Utc};
use postgresql_embedded::PostgreSQL;
use sqlx::{FromRow, PgPool, Postgres, Transaction, types::Uuid};
use tracing::{debug, error, info, instrument};

#[derive(FromRow)]
struct MatchRow {
    id: Uuid,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    created_at: Option<NaiveDateTime>,
    format: Option<String>,
}

#[derive(FromRow)]
struct MatchWithResultRow {
    id: Uuid,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    winning_team_id: i32,
    created_at: Option<NaiveDateTime>,
    format: Option<String>,
}

#[derive(FromRow)]
struct MatchSummaryRow {
    id: Uuid,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    created_at: Option<NaiveDateTime>,
    format: Option<String>,
    match_winning_team_id: Option<i32>,
    game_wins: i64,
    game_losses: i64,
    controller_archetype: Option<String>,
    opponent_archetype: Option<String>,
}

#[derive(FromRow)]
struct EventLogRow {
    game_number: i32,
    events_json: String,
}

use std::sync::Arc;

use arenabuddy_core::display::{
    match_summary::MatchSummary,
    stats::{MatchStats, MulliganBucket, OpponentRecord, TimeWindow},
};

use super::{
    auth_repository::AuthRepository,
    debug_repository::DebugRepository,
    models::{AppUser, RefreshToken},
};
use crate::{Error, Result, db::repository::ArenabuddyRepository};

#[derive(Debug, Clone)]
pub struct PostgresMatchDB {
    pool: PgPool,
    _db: Option<Arc<PostgreSQL>>,
    cards: CardsDatabase,
}

impl PostgresMatchDB {
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Map Arena IDs to card names using the cards database.
    pub(crate) fn arena_ids_to_card_names(&self, arena_ids: &[i32]) -> Vec<String> {
        let mut names = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for id in arena_ids {
            if !seen.insert(id) {
                continue;
            }
            if let Some(card) = self.cards.get(id) {
                names.push(card.name.clone());
            }
        }
        names
    }

    pub async fn new(url: Option<&str>, cards: CardsDatabase) -> Result<Self> {
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
        let home = dirs::home_dir().ok_or_else(|| {
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

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_match(
        match_id: &Uuid,
        mtga_match: &MTGAMatch,
        user_id: Option<Uuid>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        sqlx::query(
            r"INSERT INTO match
            (id, controller_seat_id, controller_player_name, opponent_player_name, created_at, user_id, format)
            VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT(id) DO UPDATE SET format = COALESCE(excluded.format, match.format)",
        )
        .bind(match_id)
        .bind(mtga_match.controller_seat_id())
        .bind(mtga_match.controller_player_name())
        .bind(mtga_match.opponent_player_name())
        .bind(mtga_match.created_at().naive_utc())
        .bind(user_id)
        .bind(mtga_match.format())
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

    async fn insert_event_log(
        match_id: &Uuid,
        event_log: &GameEventLog,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let events_json = serde_json::to_string(&event_log.events)?;

        sqlx::query(
            r"INSERT INTO match_event_log (match_id, game_number, events_json)
             VALUES ($1, $2, $3)
             ON CONFLICT (match_id, game_number)
             DO UPDATE SET events_json = excluded.events_json",
        )
        .bind(match_id)
        .bind(event_log.game_number)
        .bind(events_json)
        .execute(&mut **tx)
        .await?;
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

    #[instrument(skip(self, draft), fields(draft_id = %draft.draft().id()))]
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

    async fn query_record(
        &self,
        user_id: Option<Uuid>,
        scope: &str,
        cutoff: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64)> {
        #[derive(FromRow)]
        struct RecordRow {
            total: i64,
            wins: i64,
        }

        let row: RecordRow = sqlx::query_as(
            r"SELECT
                COUNT(DISTINCT m.id) AS total,
                COUNT(DISTINCT CASE WHEN mr.winning_team_id = m.controller_seat_id THEN m.id END) AS wins
            FROM match m
            JOIN match_result mr ON m.id = mr.match_id AND mr.result_scope = $2
            WHERE ($1::uuid IS NULL OR m.user_id = $1)
              AND ($3::timestamptz IS NULL OR m.created_at >= $3)",
        )
        .bind(user_id)
        .bind(scope)
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await?;

        Ok((row.total, row.wins))
    }

    async fn query_play_draw_stats(
        &self,
        user_id: Option<Uuid>,
        cutoff: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64, i64, i64)> {
        #[derive(FromRow)]
        struct PlayDrawRow {
            play_draw: String,
            wins: i64,
            losses: i64,
        }

        let rows: Vec<PlayDrawRow> = sqlx::query_as(
            r"SELECT
                mul.play_draw,
                COUNT(CASE WHEN mr.winning_team_id = m.controller_seat_id THEN 1 END) AS wins,
                COUNT(CASE WHEN mr.winning_team_id != m.controller_seat_id THEN 1 END) AS losses
            FROM match m
            JOIN mulligan mul ON m.id = mul.match_id AND mul.decision = 'Keep'
                AND mul.play_draw IN ('Play', 'Draw')
            JOIN match_result mr ON m.id = mr.match_id AND mr.result_scope = 'MatchScope_Game' AND mr.game_number = mul.game_number
            WHERE ($1::uuid IS NULL OR m.user_id = $1)
              AND ($2::timestamptz IS NULL OR m.created_at >= $2)
            GROUP BY mul.play_draw",
        )
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        let mut play_wins = 0i64;
        let mut play_losses = 0i64;
        let mut draw_wins = 0i64;
        let mut draw_losses = 0i64;
        for row in &rows {
            if row.play_draw == "Play" {
                play_wins = row.wins;
                play_losses = row.losses;
            } else {
                draw_wins = row.wins;
                draw_losses = row.losses;
            }
        }
        Ok((play_wins, play_losses, draw_wins, draw_losses))
    }

    async fn query_mulligan_stats(
        &self,
        user_id: Option<Uuid>,
        cutoff: Option<DateTime<Utc>>,
    ) -> Result<Vec<MulliganBucket>> {
        #[derive(FromRow)]
        struct MulliganRow {
            number_to_keep: i32,
            count: i64,
            wins: i64,
            losses: i64,
        }

        let rows: Vec<MulliganRow> = sqlx::query_as(
            r"SELECT
                mul.number_to_keep,
                COUNT(*) AS count,
                COUNT(CASE WHEN mr.winning_team_id = m.controller_seat_id THEN 1 END) AS wins,
                COUNT(CASE WHEN mr.winning_team_id != m.controller_seat_id THEN 1 END) AS losses
            FROM match m
            JOIN mulligan mul ON m.id = mul.match_id AND mul.decision = 'Keep'
            JOIN match_result mr ON m.id = mr.match_id AND mr.result_scope = 'MatchScope_Game' AND mr.game_number = mul.game_number
            WHERE ($1::uuid IS NULL OR m.user_id = $1)
              AND ($2::timestamptz IS NULL OR m.created_at >= $2)
            GROUP BY mul.number_to_keep
            ORDER BY mul.number_to_keep DESC",
        )
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| MulliganBucket {
                cards_kept: row.number_to_keep,
                count: row.count,
                wins: row.wins,
                losses: row.losses,
            })
            .collect())
    }

    async fn query_opponent_stats(
        &self,
        user_id: Option<Uuid>,
        cutoff: Option<DateTime<Utc>>,
    ) -> Result<Vec<OpponentRecord>> {
        #[derive(FromRow)]
        struct OpponentRow {
            opponent_player_name: String,
            matches: i64,
            wins: i64,
            losses: i64,
        }

        let rows: Vec<OpponentRow> = sqlx::query_as(
            r"SELECT
                m.opponent_player_name,
                COUNT(DISTINCT m.id) AS matches,
                COUNT(DISTINCT CASE WHEN mr.winning_team_id = m.controller_seat_id THEN m.id END) AS wins,
                COUNT(DISTINCT CASE WHEN mr.winning_team_id != m.controller_seat_id THEN m.id END) AS losses
            FROM match m
            JOIN match_result mr ON m.id = mr.match_id AND mr.result_scope = 'MatchScope_Match'
            WHERE ($1::uuid IS NULL OR m.user_id = $1)
              AND ($2::timestamptz IS NULL OR m.created_at >= $2)
            GROUP BY m.opponent_player_name
            ORDER BY matches DESC
            LIMIT 10",
        )
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OpponentRecord {
                name: row.opponent_player_name,
                matches: row.matches,
                wins: row.wins,
                losses: row.losses,
            })
            .collect())
    }
}

#[async_trait::async_trait]
impl ArenabuddyRepository for PostgresMatchDB {
    #[instrument(skip(self))]
    async fn init(&self) -> Result<()> {
        sqlx::migrate!("./migrations/postgres").run(&self.pool).await?;
        Ok(())
    }

    #[instrument(skip(self, replay), fields(match_id = %replay.match_id))]
    async fn write_replay(&self, replay: &MatchReplay) -> Result<()> {
        info!("Writing match replay to database");
        let controller_seat_id = replay.get_controller_seat_id();
        let match_id = Uuid::parse_str(&replay.match_id)?;
        let (controller_name, opponent_name) = replay.get_player_names(controller_seat_id)?;
        let event_start = replay.match_start_time().unwrap_or(Utc::now());

        let mtga_match = MTGAMatchBuilder::default()
            .id(match_id.to_string())
            .controller_seat_id(controller_seat_id)
            .controller_player_name(controller_name)
            .opponent_player_name(opponent_name)
            .created_at(event_start)
            .format(replay.match_format())
            .build()?;

        let mut tx = self.pool.begin().await.map_err(Error::from)?;

        Self::insert_match(&match_id, &mtga_match, None, &mut tx).await?;

        let decklists = replay.get_decklists()?;
        for deck in &decklists {
            Self::insert_deck(&match_id, deck, &mut tx).await?;
        }

        let mulligan_infos = replay.get_mulligan_infos(&self.cards)?;
        for mulligan_info in &mulligan_infos {
            Self::insert_mulligan_info(&match_id, mulligan_info, &mut tx).await?;
        }

        // not too keen on this data model
        let match_results = replay.get_match_results()?;
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

        Self::insert_opponent_deck(&match_id, &replay.get_opponent_cards(), &mut tx).await?;

        let event_logs = replay.get_event_logs(&self.cards);
        for event_log in &event_logs {
            Self::insert_event_log(&match_id, event_log, &mut tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_match(&self, match_id: &str, user_id: Option<Uuid>) -> Result<(MTGAMatch, Option<MatchResult>)> {
        info!("Getting match details for match_id: {}", match_id);
        let match_id = Uuid::parse_str(match_id)?;

        let result: Option<MatchWithResultRow> = sqlx::query_as(
            r"
            SELECT
                m.id, m.controller_player_name, m.opponent_player_name, mr.winning_team_id, m.controller_seat_id, m.created_at, m.format
            FROM match m JOIN match_result mr ON m.id = mr.match_id
            WHERE m.id = $1 AND mr.result_scope = 'MatchScope_Match' AND ($2::uuid IS NULL OR m.user_id = $2) LIMIT 1
            ",
        )
        .bind(match_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let mtga_match = MTGAMatchBuilder::default()
                .id(row.id.to_string())
                .controller_seat_id(row.controller_seat_id)
                .controller_player_name(row.controller_player_name)
                .opponent_player_name(row.opponent_player_name)
                .created_at(
                    row.created_at
                        .map(|naive: NaiveDateTime| naive.and_utc())
                        .unwrap_or_default(),
                )
                .format(row.format)
                .build()
                .expect("all fields provided");
            Ok((
                mtga_match,
                Some(MatchResult::new_match_result(row.id, row.winning_team_id)),
            ))
        } else {
            debug!("No match-level result found for match_id: {}", match_id);
            Ok((MTGAMatch::default(), None))
        }
    }

    #[instrument(skip(self))]
    async fn list_matches(&self, user_id: Option<Uuid>) -> Result<Vec<MTGAMatch>> {
        let results: Vec<MatchRow> = sqlx::query_as(
            "SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at, format FROM match WHERE ($1::uuid IS NULL OR user_id = $1) ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let matches: Vec<_> = results
            .into_iter()
            .map(|row| {
                MTGAMatchBuilder::default()
                    .id(row.id.to_string())
                    .controller_seat_id(row.controller_seat_id)
                    .controller_player_name(row.controller_player_name)
                    .opponent_player_name(row.opponent_player_name)
                    .created_at(
                        row.created_at
                            .map(|naive: NaiveDateTime| naive.and_utc())
                            .unwrap_or_default(),
                    )
                    .format(row.format)
                    .build()
                    .expect("all fields provided")
            })
            .collect();

        info!("found {} matches", matches.len());
        Ok(matches)
    }

    #[instrument(skip(self))]
    async fn list_match_summaries(&self, user_id: Option<Uuid>) -> Result<Vec<MatchSummary>> {
        let rows: Vec<MatchSummaryRow> = sqlx::query_as(
            r"SELECT
                m.id,
                m.controller_seat_id,
                m.controller_player_name,
                m.opponent_player_name,
                m.created_at,
                m.format,
                match_mr.winning_team_id AS match_winning_team_id,
                COALESCE(gs.game_wins, 0) AS game_wins,
                COALESCE(gs.game_losses, 0) AS game_losses,
                ca.archetype_name AS controller_archetype,
                oa.archetype_name AS opponent_archetype
            FROM match m
            LEFT JOIN match_result match_mr
                ON m.id = match_mr.match_id AND match_mr.result_scope = 'MatchScope_Match'
            LEFT JOIN LATERAL (
                SELECT
                    COUNT(CASE WHEN gr.winning_team_id = m.controller_seat_id THEN 1 END) AS game_wins,
                    COUNT(CASE WHEN gr.winning_team_id != m.controller_seat_id THEN 1 END) AS game_losses
                FROM match_result gr
                WHERE gr.match_id = m.id AND gr.result_scope = 'MatchScope_Game'
            ) gs ON true
            LEFT JOIN match_archetype ca ON m.id = ca.match_id AND ca.side = 'controller'
            LEFT JOIN match_archetype oa ON m.id = oa.match_id AND oa.side = 'opponent'
            WHERE ($1::uuid IS NULL OR m.user_id = $1)
            ORDER BY m.created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let summaries = rows
            .into_iter()
            .map(|row| MatchSummary {
                id: row.id.to_string(),
                controller_player_name: row.controller_player_name,
                opponent_player_name: row.opponent_player_name,
                created_at: row
                    .created_at
                    .map(|naive: NaiveDateTime| naive.and_utc())
                    .unwrap_or_default(),
                format: row.format,
                did_controller_win: row.match_winning_team_id.map(|wt| wt == row.controller_seat_id),
                game_wins: row.game_wins,
                game_losses: row.game_losses,
                controller_archetype: row.controller_archetype,
                opponent_archetype: row.opponent_archetype,
            })
            .collect();

        Ok(summaries)
    }

    #[instrument(skip(self))]
    async fn list_decklists(&self, match_id: &str) -> Result<Vec<Deck>> {
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

    #[instrument(skip(self))]
    async fn list_mulligans(&self, match_id: &str) -> Result<Vec<Mulligan>> {
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

    #[instrument(skip(self))]
    async fn list_match_results(&self, match_id: &str) -> Result<Vec<MatchResult>> {
        let match_id = Uuid::parse_str(match_id)?;
        let results = sqlx::query!(
            "SELECT game_number, winning_team_id, result_scope FROM match_result WHERE match_id = $1",
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

    #[instrument(skip(self))]
    async fn get_opponent_deck(&self, match_id: &str) -> Result<Deck> {
        let match_id = Uuid::parse_str(match_id)?;
        let result = sqlx::query!("SELECT cards FROM opponent_deck WHERE match_id = $1", match_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Deck::from_raw("Opponent_deck".to_string(), 0, &result.cards, ""))
    }

    #[instrument(skip(self))]
    async fn list_drafts(&self) -> Result<Vec<Draft>> {
        let result = sqlx::query!(
            r#"
                SELECT id, set_code, draft_format, status, created_at
                FROM draft
                ORDER BY created_at DESC
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
                    row.draft_format.map(Format::parse_format).unwrap_or_default(),
                    row.status.unwrap_or_default(),
                )
                .with_created_at(row.created_at.unwrap_or_default().and_utc())
            })
            .collect())
    }

    #[instrument(skip(self))]
    async fn get_draft(&self, draft_id: &str) -> Result<MTGADraft> {
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
            draft_row.draft_format.map(Format::parse_format).unwrap_or_default(),
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

    #[instrument(skip(self, mtga_match, decks, mulligans, results, opponent_cards, event_logs), fields(match_id = %mtga_match.id()))]
    async fn upsert_match_data(
        &self,
        mtga_match: &MTGAMatch,
        decks: &[Deck],
        mulligans: &[Mulligan],
        results: &[MatchResult],
        opponent_cards: &[ArenaId],
        event_logs: &[GameEventLog],
        user_id: Option<Uuid>,
    ) -> Result<()> {
        info!("Upserting match data for match_id: {}", mtga_match.id());
        let match_id = Uuid::parse_str(mtga_match.id())?;

        let mut tx = self.pool.begin().await.map_err(Error::from)?;

        Self::insert_match(&match_id, mtga_match, user_id, &mut tx).await?;

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

        for event_log in event_logs {
            Self::insert_event_log(&match_id, event_log, &mut tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_event_logs(&self, match_id: &str) -> Result<Vec<GameEventLog>> {
        let match_id = Uuid::parse_str(match_id)?;
        let rows: Vec<EventLogRow> = sqlx::query_as(
            "SELECT game_number, events_json FROM match_event_log WHERE match_id = $1 ORDER BY game_number",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;

        let event_logs = rows
            .into_iter()
            .map(|row| {
                let events = serde_json::from_str(&row.events_json).unwrap_or_default();
                GameEventLog {
                    game_number: row.game_number,
                    events,
                }
            })
            .collect();

        Ok(event_logs)
    }

    #[instrument(skip(self))]
    async fn delete_match(&self, match_id: &str, user_id: Option<Uuid>) -> Result<()> {
        info!("Deleting match: {}", match_id);
        let match_id = Uuid::parse_str(match_id)?;

        sqlx::query("DELETE FROM match WHERE id = $1 AND ($2::uuid IS NULL OR user_id = $2)")
            .bind(match_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_match_stats(&self, user_id: Option<Uuid>, time_window: TimeWindow) -> Result<MatchStats> {
        let cutoff = time_window.cutoff();
        let (total_matches, match_wins) = self.query_record(user_id, "MatchScope_Match", cutoff).await?;
        let (total_games, game_wins) = self.query_record(user_id, "MatchScope_Game", cutoff).await?;
        let (play_wins, play_losses, draw_wins, draw_losses) = self.query_play_draw_stats(user_id, cutoff).await?;
        let mulligan_stats = self.query_mulligan_stats(user_id, cutoff).await?;
        let opponents = self.query_opponent_stats(user_id, cutoff).await?;

        Ok(MatchStats {
            total_matches,
            match_wins,
            match_losses: total_matches - match_wins,
            total_games,
            game_wins,
            game_losses: total_games - game_wins,
            play_wins,
            play_losses,
            draw_wins,
            draw_losses,
            mulligan_stats,
            opponents,
        })
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

#[async_trait::async_trait]
impl AuthRepository for PostgresMatchDB {
    #[instrument(skip(self, avatar_url))]
    async fn upsert_user(&self, discord_id: &str, username: &str, avatar_url: Option<&str>) -> Result<Uuid> {
        let row: (Uuid,) = sqlx::query_as(
            r"
            INSERT INTO app_user (discord_id, username, avatar_url)
            VALUES ($1, $2, $3)
            ON CONFLICT (discord_id)
            DO UPDATE SET username = excluded.username, avatar_url = excluded.avatar_url, updated_at = now()
            RETURNING id
            ",
        )
        .bind(discord_id)
        .bind(username)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    #[instrument(skip(self))]
    async fn get_user(&self, user_id: Uuid) -> Result<Option<AppUser>> {
        let row: Option<AppUser> =
            sqlx::query_as("SELECT id, discord_id, username, avatar_url FROM app_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row)
    }

    #[instrument(skip(self, token_hash))]
    async fn create_refresh_token(&self, user_id: Uuid, token_hash: &[u8], expires_at: DateTime<Utc>) -> Result<()> {
        sqlx::query("INSERT INTO refresh_token (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self, token_hash))]
    async fn find_refresh_token(&self, token_hash: &[u8]) -> Result<Option<RefreshToken>> {
        let row: Option<RefreshToken> = sqlx::query_as(
            "SELECT id, user_id, revoked FROM refresh_token WHERE token_hash = $1 AND expires_at > now()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    #[instrument(skip(self))]
    async fn revoke_refresh_token(&self, token_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE refresh_token SET revoked = true WHERE id = $1")
            .bind(token_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self, token_hash))]
    async fn revoke_refresh_token_by_hash(&self, token_hash: &[u8]) -> Result<()> {
        sqlx::query("UPDATE refresh_token SET revoked = true WHERE token_hash = $1 AND revoked = false")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn cleanup_expired_tokens(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM refresh_token WHERE user_id = $1 AND expires_at < now()")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl DebugRepository for PostgresMatchDB {
    #[instrument(skip(self, raw_json))]
    async fn insert_parse_error(
        &self,
        user_id: Option<Uuid>,
        raw_json: &str,
        reported_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO parse_error (user_id, raw_json, reported_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(raw_json)
            .bind(reported_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
