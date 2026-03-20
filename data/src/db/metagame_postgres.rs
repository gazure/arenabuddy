use sqlx::types::Uuid;

use super::{
    metagame_models::{
        CardFrequencyRow, MatchArchetype, MetagameDeck, MetagameDeckCard, MetagameTournament, SignatureCard,
        SignatureCardRow, UnclassifiedMatchRow,
    },
    metagame_repository::{MetagameRepository, MetagameStatsResult},
    postgres::PostgresMatchDB,
};
use crate::Result;

#[async_trait::async_trait]
impl MetagameRepository for PostgresMatchDB {
    async fn upsert_metagame_tournament(&self, tournament: &MetagameTournament) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO metagame_tournament (goldfish_id, name, format, date, url)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (goldfish_id) DO UPDATE SET
                 name = EXCLUDED.name,
                 format = EXCLUDED.format,
                 date = EXCLUDED.date,
                 url = EXCLUDED.url,
                 scraped_at = NOW()
             RETURNING id",
        )
        .bind(tournament.goldfish_id)
        .bind(&tournament.name)
        .bind(&tournament.format)
        .bind(tournament.date)
        .bind(&tournament.url)
        .fetch_one(self.pool())
        .await?;
        Ok(row.0)
    }

    async fn upsert_metagame_archetype(&self, name: &str, format: &str, url: Option<&str>) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO metagame_archetype (name, format, url)
             VALUES ($1, $2, $3)
             ON CONFLICT (name, format) DO UPDATE SET
                 url = COALESCE(EXCLUDED.url, metagame_archetype.url)
             RETURNING id",
        )
        .bind(name)
        .bind(format)
        .bind(url)
        .fetch_one(self.pool())
        .await?;
        Ok(row.0)
    }

    async fn upsert_metagame_deck(
        &self,
        deck: &MetagameDeck,
        tournament_id: Option<i32>,
        archetype_id: Option<i32>,
        cards: &[MetagameDeckCard],
    ) -> Result<i32> {
        let mut tx = self.pool().begin().await?;

        let deck_id: (i32,) = sqlx::query_as(
            "INSERT INTO metagame_deck (goldfish_id, tournament_id, archetype_id, player_name, placement, format, date, url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (goldfish_id) DO UPDATE SET
                 tournament_id = EXCLUDED.tournament_id,
                 archetype_id = EXCLUDED.archetype_id,
                 player_name = EXCLUDED.player_name,
                 placement = EXCLUDED.placement,
                 format = EXCLUDED.format,
                 date = EXCLUDED.date,
                 url = EXCLUDED.url,
                 scraped_at = NOW()
             RETURNING id",
        )
        .bind(deck.goldfish_id)
        .bind(tournament_id)
        .bind(archetype_id)
        .bind(&deck.player_name)
        .bind(&deck.placement)
        .bind(&deck.format)
        .bind(deck.date)
        .bind(&deck.url)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM metagame_deck_card WHERE deck_id = $1")
            .bind(deck_id.0)
            .execute(&mut *tx)
            .await?;

        for card in cards {
            sqlx::query(
                "INSERT INTO metagame_deck_card (deck_id, card_name, quantity, is_sideboard)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(deck_id.0)
            .bind(&card.card_name)
            .bind(card.quantity)
            .bind(card.is_sideboard)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(deck_id.0)
    }

    async fn metagame_stats(&self, format: &str) -> Result<MetagameStatsResult> {
        let tournament_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metagame_tournament WHERE format = $1")
            .bind(format)
            .fetch_one(self.pool())
            .await?;

        let archetype_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metagame_archetype WHERE format = $1")
            .bind(format)
            .fetch_one(self.pool())
            .await?;

        let deck_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metagame_deck WHERE format = $1")
            .bind(format)
            .fetch_one(self.pool())
            .await?;

        let card_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM metagame_deck_card dc
             JOIN metagame_deck d ON dc.deck_id = d.id
             WHERE d.format = $1",
        )
        .bind(format)
        .fetch_one(self.pool())
        .await?;

        Ok(MetagameStatsResult {
            tournament_count: tournament_count.0,
            archetype_count: archetype_count.0,
            deck_count: deck_count.0,
            card_count: card_count.0,
        })
    }

    async fn get_card_frequencies(&self, format: &str) -> Result<Vec<CardFrequencyRow>> {
        let rows: Vec<CardFrequencyRow> = sqlx::query_as(
            r"SELECT
                a.id AS archetype_id,
                a.name AS archetype_name,
                dc.card_name,
                COUNT(DISTINCT dc.deck_id)::bigint AS archetype_deck_count,
                (SELECT COUNT(DISTINCT d2.id) FROM metagame_deck d2 WHERE d2.archetype_id = a.id AND d2.format = $1)::bigint AS total_archetype_decks,
                (SELECT COUNT(DISTINCT dc2.deck_id) FROM metagame_deck_card dc2
                 JOIN metagame_deck d3 ON dc2.deck_id = d3.id
                 WHERE dc2.card_name = dc.card_name AND d3.format = $1)::bigint AS total_decks_with_card
            FROM metagame_archetype a
            JOIN metagame_deck d ON d.archetype_id = a.id AND d.format = $1
            JOIN metagame_deck_card dc ON dc.deck_id = d.id AND dc.is_sideboard = false
            WHERE a.format = $1
            GROUP BY a.id, a.name, dc.card_name",
        )
        .bind(format)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }

    async fn replace_signature_cards(&self, format: &str, cards: &[SignatureCard]) -> Result<u64> {
        let mut tx = self.pool().begin().await?;

        sqlx::query("DELETE FROM archetype_signature_card WHERE format = $1")
            .bind(format)
            .execute(&mut *tx)
            .await?;

        let mut count = 0u64;
        for card in cards {
            sqlx::query(
                "INSERT INTO archetype_signature_card (archetype_id, card_name, weight, format)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(card.archetype_id)
            .bind(&card.card_name)
            .bind(card.weight)
            .bind(&card.format)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    async fn get_signature_cards(&self, format: &str) -> Result<Vec<SignatureCardRow>> {
        let rows: Vec<SignatureCardRow> = sqlx::query_as(
            r"SELECT sc.archetype_id, a.name AS archetype_name, sc.card_name, sc.weight
              FROM archetype_signature_card sc
              JOIN metagame_archetype a ON sc.archetype_id = a.id
              WHERE sc.format = $1
              ORDER BY sc.archetype_id, sc.weight DESC",
        )
        .bind(format)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }

    async fn get_unclassified_matches(&self, format: &str) -> Result<Vec<UnclassifiedMatchRow>> {
        // MTGA stores event IDs like 'Ladder', 'Traditional_Ladder', etc.
        // Map metagame format names to matching MTGA event ID patterns.
        let format_patterns = match format.to_lowercase().as_str() {
            "standard" => vec!["Ladder", "Traditional_Ladder"],
            "explorer" => vec!["Explorer_Ladder", "Traditional_Explorer_Ladder"],
            "historic" => vec!["Historic_Ladder", "Traditional_Historic_Ladder"],
            "timeless" => vec!["Timeless_Ladder", "Traditional_Timeless_Ladder"],
            _ => vec![],
        };

        let rows: Vec<UnclassifiedMatchRow> = if format_patterns.is_empty() {
            // No filter or unknown format — return all unclassified matches
            sqlx::query_as(
                r"SELECT m.id AS match_id, m.format
                  FROM match m
                  WHERE m.format IS NOT NULL
                    AND m.id NOT IN (SELECT match_id FROM match_archetype WHERE side = 'controller')
                  ORDER BY m.created_at DESC",
            )
            .fetch_all(self.pool())
            .await?
        } else {
            sqlx::query_as(
                r"SELECT m.id AS match_id, m.format
                  FROM match m
                  WHERE m.format IS NOT NULL
                    AND m.id NOT IN (SELECT match_id FROM match_archetype WHERE side = 'controller')
                    AND m.format = ANY($1)
                  ORDER BY m.created_at DESC",
            )
            .bind(&format_patterns)
            .fetch_all(self.pool())
            .await?
        };

        Ok(rows)
    }

    async fn upsert_match_archetype(&self, archetype: &MatchArchetype) -> Result<()> {
        let match_id = Uuid::parse_str(&archetype.match_id)?;
        sqlx::query(
            "INSERT INTO match_archetype (match_id, side, archetype_id, archetype_name, confidence)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (match_id, side) DO UPDATE SET
                 archetype_id = EXCLUDED.archetype_id,
                 archetype_name = EXCLUDED.archetype_name,
                 confidence = EXCLUDED.confidence,
                 classified_at = NOW()",
        )
        .bind(match_id)
        .bind(&archetype.side)
        .bind(archetype.archetype_id)
        .bind(&archetype.archetype_name)
        .bind(archetype.confidence)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_match_deck_cards(&self, match_id: &str) -> Result<Vec<String>> {
        let match_uuid = Uuid::parse_str(match_id)?;
        // Get all arena IDs from the deck for game 1, then map to card names
        let rows: Vec<(String,)> =
            sqlx::query_as(r"SELECT d.deck_cards FROM deck d WHERE d.match_id = $1 ORDER BY d.game_number LIMIT 1")
                .bind(match_uuid)
                .fetch_all(self.pool())
                .await?;

        let Some(row) = rows.first() else {
            return Ok(Vec::new());
        };

        // deck_cards is JSON array of arena IDs
        let arena_ids: Vec<i32> = serde_json::from_str(&row.0).unwrap_or_default();
        let card_names = self.arena_ids_to_card_names(&arena_ids);
        Ok(card_names)
    }

    async fn get_match_opponent_cards(&self, match_id: &str) -> Result<Vec<String>> {
        let match_uuid = Uuid::parse_str(match_id)?;
        let rows: Vec<(String,)> = sqlx::query_as(r"SELECT od.cards FROM opponent_deck od WHERE od.match_id = $1")
            .bind(match_uuid)
            .fetch_all(self.pool())
            .await?;

        let Some(row) = rows.first() else {
            return Ok(Vec::new());
        };

        let arena_ids: Vec<i32> = serde_json::from_str(&row.0).unwrap_or_default();
        let card_names = self.arena_ids_to_card_names(&arena_ids);
        Ok(card_names)
    }
}
