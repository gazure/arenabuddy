use arenabuddy_core::{
    models::{Deck, GameEventLog, MTGAMatch, MTGAMatchBuilder, MatchData, OpponentDeck},
    player_log::replay::MatchReplay,
};
use sqlx::{FromRow, Postgres, Transaction, types::Uuid};

use super::{EventLogRow, MatchRow, PostgresMatchDB};
use crate::{Error, Result};

#[cfg(test)]
#[path = "upload_tests.rs"]
mod tests;

/// A durable upload attempt leased to one worker.
#[derive(Debug, FromRow)]
pub struct MatchUploadJob {
    /// Arena match ID.
    pub match_id: String,
    /// Remote account that owns this upload.
    pub user_id: String,
    /// Payload revision used to reject stale acknowledgments.
    pub revision: i64,
    /// Unique identifier for this lease.
    pub lease_token: String,
    /// Serialized `MatchData` captured when the local transaction committed.
    pub payload: String,
    /// Number of delivery attempts for this revision.
    pub attempts: i32,
}

/// Persisted delivery status for the match details page.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MatchUploadStatus {
    /// Remote account, or `None` until the user explicitly queues a signed-out match.
    pub user_id: Option<String>,
    /// One of `pending`, `sent`, or `blocked`.
    pub state: String,
    /// Most recent failure, if any.
    pub last_error: Option<String>,
}

fn stored_match(row: MatchRow) -> Result<MTGAMatch> {
    Ok(MTGAMatchBuilder::default()
        .id(row.id.to_string())
        .controller_seat_id(row.controller_seat_id)
        .controller_player_name(row.controller_player_name)
        .opponent_player_name(row.opponent_player_name)
        .created_at(row.created_at.map(|date| date.and_utc()).unwrap_or_default())
        .format(row.format)
        .build()?)
}

impl PostgresMatchDB {
    /// Saves a replay and its upload job in one transaction.
    ///
    /// `user_id` is the remote account at capture time, or `None` while signed out.
    /// Returns an error if parsing, persistence, or the upload account check fails.
    pub async fn write_replay_for_upload(&self, replay: &MatchReplay, user_id: Option<&str>) -> Result<()> {
        let data = MatchData::from_replay(replay, &self.cards)?;
        self.save_match_for_upload(&data, user_id).await
    }

    /// Saves local match data and queues its exact snapshot atomically.
    ///
    /// `user_id` must identify the original remote account, if the job is bound.
    /// Unassigned existing jobs remain unassigned until explicitly queued.
    /// Returns an error without saving either record if the transaction fails.
    pub async fn save_match_for_upload(&self, data: &MatchData, user_id: Option<&str>) -> Result<()> {
        let id = Uuid::parse_str(data.mtga_match.id())?;
        let owner = user_id.map(Uuid::parse_str).transpose()?;
        let mut tx = self.pool.begin().await?;
        Self::insert_match(&id, &data.mtga_match, None, &mut tx).await?;
        for deck in &data.decks {
            Self::insert_deck(&id, deck, &mut tx).await?;
        }
        for mulligan in &data.mulligans {
            Self::insert_mulligan_info(&id, mulligan, &mut tx).await?;
        }
        for result in &data.results {
            Self::insert_match_result(&id, result, &mut tx).await?;
        }
        Self::insert_opponent_deck(&id, &data.opponent_deck.cards, &mut tx).await?;
        for event_log in &data.event_logs {
            Self::insert_event_log(&id, event_log, &mut tx).await?;
        }
        Self::enqueue_upload(&mut tx, data, owner, false).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn enqueue_upload(
        tx: &mut Transaction<'_, Postgres>,
        data: &MatchData,
        owner: Option<Uuid>,
        explicit: bool,
    ) -> Result<()> {
        let id = Uuid::parse_str(data.mtga_match.id())?;
        // Use persisted metadata: replays without a timestamp use a clock
        // fallback, which must not create a new upload on every log reread.
        let row: MatchRow = sqlx::query_as(
            "SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at, format
             FROM match WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
        let snapshot = MatchData {
            mtga_match: stored_match(row)?,
            ..data.clone()
        };
        let payload = serde_json::to_string(&snapshot)?;
        let inserted = sqlx::query(
            "INSERT INTO match_upload_outbox (match_id, user_id, payload) VALUES ($1, $2, $3)
             ON CONFLICT (match_id) DO NOTHING",
        )
        .bind(id)
        .bind(owner)
        .bind(&payload)
        .execute(&mut **tx)
        .await?;
        if inserted.rows_affected() != 0 {
            return Ok(());
        }
        let (existing_owner, existing_payload): (Option<Uuid>, String) =
            sqlx::query_as("SELECT user_id, payload FROM match_upload_outbox WHERE match_id = $1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;
        if existing_owner.is_some() && owner.is_some() && existing_owner != owner {
            return Err(Error::UploadAccountConflict);
        }
        // Re-reading the same log must not repeatedly upload already-sent data.
        if !explicit && payload == existing_payload {
            return Ok(());
        }
        let owner = if explicit {
            existing_owner.or(owner)
        } else {
            existing_owner
        };
        sqlx::query(
            "UPDATE match_upload_outbox SET user_id = $2, payload = $3, revision = revision + 1,
             state = 'pending', attempts = 0, next_attempt_at = now(), last_error = NULL
             WHERE match_id = $1",
        )
        .bind(id)
        .bind(owner)
        .bind(payload)
        .execute(&mut **tx)
        .await?;
        // Retain an active lease until its worker finishes. Otherwise a newer
        // payload could reach the server before an older in-flight request.
        Ok(())
    }

    /// Queues a stored local match for the specified remote account.
    ///
    /// Explicit retries reset blocked jobs and can bind unassigned jobs. Returns
    /// `false` for a missing match and an error for another account's job or a
    /// database failure. Existing data is read under the parent row lock.
    pub async fn queue_match_upload(&self, match_id: &str, user_id: &str) -> Result<bool> {
        let id = Uuid::parse_str(match_id)?;
        let owner = Uuid::parse_str(user_id)?;
        let mut tx = self.pool.begin().await?;
        let row: Option<MatchRow> = sqlx::query_as(
            "SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at, format
             FROM match WHERE id = $1 AND user_id IS NULL FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else { return Ok(false) };
        let mtga_match = stored_match(row)?;
        let decks: Vec<(i32, String, String)> = sqlx::query_as(
            "SELECT game_number, deck_cards, sideboard_cards FROM deck WHERE match_id = $1 ORDER BY game_number",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let decks = decks
            .into_iter()
            .map(|(game, main, side)| {
                Ok(Deck::new(
                    "Found Deck".into(),
                    game,
                    serde_json::from_str(&main)?,
                    serde_json::from_str(&side)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mulligans: Vec<String> = sqlx::query_scalar(
            "SELECT to_jsonb(m)::text FROM mulligan m WHERE match_id = $1 ORDER BY game_number, number_to_keep",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let results: Vec<String> =
            sqlx::query_scalar("SELECT to_jsonb(r)::text FROM match_result r WHERE match_id = $1 ORDER BY game_number")
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;
        let opponent: Option<String> = sqlx::query_scalar("SELECT cards FROM opponent_deck WHERE match_id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let events: Vec<EventLogRow> = sqlx::query_as(
            "SELECT game_number, events_json FROM match_event_log WHERE match_id = $1 ORDER BY game_number",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let data = MatchData {
            mtga_match,
            decks,
            mulligans: mulligans
                .iter()
                .map(|json| serde_json::from_str(json))
                .collect::<std::result::Result<_, _>>()?,
            results: results
                .iter()
                .map(|json| serde_json::from_str(json))
                .collect::<std::result::Result<_, _>>()?,
            opponent_deck: OpponentDeck::new(serde_json::from_str(opponent.as_deref().unwrap_or("[]"))?),
            event_logs: events
                .into_iter()
                .map(|row| {
                    Ok(GameEventLog {
                        game_number: row.game_number,
                        events: serde_json::from_str(&row.events_json)?,
                    })
                })
                .collect::<Result<_>>()?,
        };
        Self::enqueue_upload(&mut tx, &data, Some(owner), true).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Leases the next due job belonging to `user_id` for two minutes.
    /// Returns `None` when no job is ready, or an error on database failure.
    pub async fn claim_match_upload(&self, user_id: &str) -> Result<Option<MatchUploadJob>> {
        let owner = Uuid::parse_str(user_id)?;
        Ok(sqlx::query_as(
            "WITH candidate AS (
                SELECT match_id FROM match_upload_outbox
                WHERE user_id = $1 AND state = 'pending' AND next_attempt_at <= now()
                  AND (lease_until IS NULL OR lease_until <= now())
                ORDER BY next_attempt_at, match_id FOR UPDATE SKIP LOCKED LIMIT 1
             )
             UPDATE match_upload_outbox q SET lease_token = gen_random_uuid(),
                lease_until = now() + interval '2 minutes', attempts = LEAST(q.attempts, 1000000) + 1
             FROM candidate c WHERE q.match_id = c.match_id
             RETURNING q.match_id::text, q.user_id::text, q.revision, q.lease_token::text, q.payload, q.attempts",
        )
        .bind(owner)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Completes a lease, preserving a newer payload if the match was requeued.
    ///
    /// With no `failure`, marks the revision sent. Otherwise, records the error
    /// and schedules a retry after `delay_seconds`, or blocks it if `blocked`.
    /// A stale or replaced lease cannot change the job. Database errors propagate.
    pub async fn finish_match_upload(
        &self,
        job: &MatchUploadJob,
        failure: Option<&str>,
        delay_seconds: i32,
        blocked: bool,
    ) -> Result<()> {
        let state = if failure.is_none() {
            "sent"
        } else if blocked {
            "blocked"
        } else {
            "pending"
        };
        sqlx::query(
            "UPDATE match_upload_outbox SET
                state = CASE WHEN revision = $3 THEN $4 ELSE state END,
                last_error = CASE WHEN revision = $3 THEN $5 ELSE last_error END,
                next_attempt_at = CASE WHEN revision = $3 THEN now() + make_interval(secs => $6) ELSE next_attempt_at END,
                lease_token = NULL, lease_until = NULL
             WHERE match_id = $1 AND lease_token = $2",
        )
        .bind(Uuid::parse_str(&job.match_id)?)
        .bind(Uuid::parse_str(&job.lease_token)?)
        .bind(job.revision)
        .bind(state)
        .bind(failure)
        .bind(f64::from(delay_seconds.max(0)))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns the durable upload status for `match_id`, if it has a job.
    /// Returns an error for invalid IDs or database failures.
    pub async fn match_upload_status(&self, match_id: &str) -> Result<Option<MatchUploadStatus>> {
        Ok(
            sqlx::query_as("SELECT user_id::text, state, last_error FROM match_upload_outbox WHERE match_id = $1")
                .bind(Uuid::parse_str(match_id)?)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}
