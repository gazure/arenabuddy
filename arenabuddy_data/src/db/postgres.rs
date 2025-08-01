use arenabuddy_core::{
    cards::CardsDatabase,
    models::{Deck, MTGAMatch, MTGAMatchBuilder, MatchResult, MatchResultBuilder, Mulligan},
    replay::MatchReplay,
};
use chrono::Utc;
use postgresql_embedded::PostgreSQL;
use sqlx::{PgPool, Postgres, Transaction, types::Uuid};
use tracing::{debug, error, info};

use crate::{MatchDBError, Result, Storage};

pub struct PostgresMatchDB {
    pool: PgPool,
    _db: Option<PostgreSQL>,
    pub cards: CardsDatabase,
}

impl PostgresMatchDB {
    pub async fn new(url: Option<&str>, cards: CardsDatabase) -> Result<Self> {
        if let Some(url) = url {
            let pool = PgPool::connect(url).await?;
            Ok(Self {
                pool,
                _db: None,
                cards,
            })
        } else {
            let mut db = PostgreSQL::default();
            db.setup().await?;
            db.start().await?;
            db.create_database("arenabuddy").await?;

            let pool = PgPool::connect(&db.settings().url("arenabuddy")).await?;
            Ok(Self {
                pool,
                _db: Some(db),
                cards,
            })
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let migrations_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/postgres");
        sqlx::migrate::Migrator::new(migrations_path)
            .await?
            .run(&self.pool)
            .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// passes along errors from sqlx
    pub async fn execute(&self, query: &str) -> Result<u64> {
        let rows_affected = sqlx::query(query).execute(&self.pool).await?;
        Ok(rows_affected.rows_affected())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    async fn insert_match(
        match_id: &Uuid,
        mtga_match: &MTGAMatch,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
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
    async fn insert_deck(
        match_id: &Uuid,
        deck: &Deck,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
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
    pub async fn get_match_results(&self, match_id: &str) -> Result<Vec<MatchResult>> {
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
    pub async fn get_decklists(&self, match_id: &str) -> Result<Vec<Deck>> {
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

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    pub async fn get_mulligans(&self, match_id: &str) -> Result<Vec<Mulligan>> {
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
    pub async fn get_matches(&self) -> Result<Vec<MTGAMatch>> {
        let results = sqlx::query!(
            "SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at FROM match"
        )
        .fetch_all(&self.pool)
        .await?;

        let matches = results
            .into_iter()
            .map(|row| {
                MTGAMatch::new_with_timestamp(
                    row.id,
                    row.controller_seat_id,
                    row.controller_player_name,
                    row.opponent_player_name,
                    row.created_at
                        .map(|naive| naive.and_utc())
                        .unwrap_or_default(),
                )
            })
            .collect();

        Ok(matches)
    }

    /// # Errors
    ///
    /// Errors if underlying data is malformed
    pub async fn get_match(&self, match_id: &str) -> Result<(MTGAMatch, Option<MatchResult>)> {
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
                    row.id.clone(),
                    row.controller_seat_id,
                    row.controller_player_name,
                    row.opponent_player_name,
                    row.created_at
                        .map(|naive| naive.and_utc())
                        .unwrap_or_default(),
                ),
                Some(MatchResult::new_match_result(row.id, row.winning_team_id)),
            ))
        } else {
            error!("Error getting match details for match_id: {}", match_id);
            Ok((MTGAMatch::default(), None))
        }
    }
}

impl Storage for PostgresMatchDB {
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

        let mut tx = self.pool.begin().await.map_err(MatchDBError::from)?;

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

        tx.commit().await?;
        Ok(())
    }
}
