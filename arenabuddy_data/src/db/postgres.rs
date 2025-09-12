#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::similar_names)]
use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    events::primitives::ArenaId,
    ingest::{DraftWriter, ReplayWriter},
    models::{
        Deck, Draft, DraftPack, MTGADraft, MTGAMatch, MTGAMatchBuilder, MatchResult, MatchResultBuilder, Mulligan,
    },
    replay::MatchReplay,
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
            let mut db = PostgreSQL::default();
            db.setup().await?;
            db.start().await?;
            db.create_database("arenabuddy").await?;

            let pool = PgPool::connect(&db.settings().url("arenabuddy")).await?;
            Ok(Self {
                pool,
                _db: Some(Arc::new(db)),
                cards,
            })
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/postgres");
        sqlx::migrate::Migrator::new(migrations_path)
            .await?
            .run(&self.pool)
            .await?;
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
    async fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()> {
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
            draft.format(),
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
        picked_card_id: ArenaId,
        cards: &[ArenaId],
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let cards_json = serde_json::to_string(cards)?;

        sqlx::query!(
            r#"
            INSERT INTO draft_pack(draft_id, pack_number, pick_number, cards, card_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (draft_id, pack_number, pick_number)
            DO UPDATE SET cards = excluded.cards, card_id = excluded.card_id
            "#,
            draft_id,
            pack_number,
            pick_number,
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
                pack.picked_card(),
                pack.cards(),
                &mut tx,
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn do_list_drafts(&mut self) -> Result<Vec<Draft>> {
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
                    row.status.unwrap_or_default(),
                    row.draft_format.unwrap_or_default(),
                )
                .with_created_at(row.created_at.unwrap_or_default().and_utc())
            })
            .collect())
    }

    async fn do_get_draft(&mut self, draft_id: &str) -> Result<MTGADraft> {
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
            SELECT pack_number, pick_number, cards, card_id
            FROM draft_pack
            WHERE draft_id = $1
            ORDER BY pack_number, pick_number
            "#,
            draft_id
        )
        .fetch_all(&self.pool)
        .await?;

        // Construct the Draft model
        let draft = Draft::new(
            draft_row.id,
            draft_row.set_code,
            draft_row.status.unwrap_or_default(),
            draft_row.draft_format.unwrap_or_default(),
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
                row.card_id.into(),
                cards,
            );
            packs.push(pack);
        }

        Ok(MTGADraft::new(draft, packs))
    }
}

impl ArenabuddyRepository for PostgresMatchDB {
    fn init(&mut self) -> impl Future<Output = Result<()>> + Send {
        self.initialize()
    }

    fn write_replay(&mut self, replay: &MatchReplay) -> impl Future<Output = Result<()>> + Send {
        self.write(replay)
    }

    fn get_match(&mut self, match_id: &str) -> impl Future<Output = Result<(MTGAMatch, Option<MatchResult>)>> + Send {
        self.retrieve_match(match_id)
    }

    fn list_matches(&mut self) -> impl Future<Output = Result<Vec<MTGAMatch>>> + Send {
        self.get_matches()
    }

    fn list_decklists(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<Deck>>> + Send {
        self.get_decklists(match_id)
    }

    fn list_mulligans(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<Mulligan>>> + Send {
        self.get_mulligans(match_id)
    }

    fn list_match_results(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<MatchResult>>> + Send {
        self.get_match_results(match_id)
    }

    fn get_opponent_deck(&mut self, match_id: &str) -> impl Future<Output = Result<Deck>> + Send {
        self.do_get_opponent_deck(match_id)
    }

    fn list_drafts(&mut self) -> impl Future<Output = Result<Vec<Draft>>> + Send {
        self.do_list_drafts()
    }

    fn get_draft(&mut self, draft_id: &str) -> impl Future<Output = Result<MTGADraft>> + Send {
        self.do_get_draft(draft_id)
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
