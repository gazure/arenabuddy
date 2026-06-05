use arenabuddy_core::models::{Card, CardFace};
use sqlx::FromRow;
use tracing::{info, warn};

use super::{card_repository::CardRepository, postgres::PostgresMatchDB};
use crate::Result;

#[derive(FromRow)]
struct CardRow {
    arena_id: i64,
    name: String,
    set_code: String,
    lang: String,
    image_uri: String,
    mana_cost: String,
    cmc: i32,
    type_line: String,
    layout: String,
    colors: String,
    color_identity: String,
    card_faces: String,
}

impl CardRow {
    fn into_card(self) -> Card {
        let colors: Vec<String> = serde_json::from_str(&self.colors).unwrap_or_else(|e| {
            warn!("Failed to parse colors for card {}: {e}", self.arena_id);
            Vec::new()
        });
        let color_identity: Vec<String> = serde_json::from_str(&self.color_identity).unwrap_or_else(|e| {
            warn!("Failed to parse color_identity for card {}: {e}", self.arena_id);
            Vec::new()
        });
        let card_faces: Vec<CardFaceJson> = serde_json::from_str(&self.card_faces).unwrap_or_else(|e| {
            warn!("Failed to parse card_faces for card {}: {e}", self.arena_id);
            Vec::new()
        });

        Card {
            id: self.arena_id,
            name: self.name,
            set: self.set_code,
            lang: self.lang,
            image_uri: self.image_uri,
            mana_cost: self.mana_cost,
            cmc: self.cmc,
            type_line: self.type_line,
            layout: self.layout,
            colors,
            color_identity,
            card_faces: card_faces.into_iter().map(CardFaceJson::into_card_face).collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CardFaceJson {
    name: String,
    type_line: String,
    mana_cost: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_uri: Option<String>,
    colors: Vec<String>,
}

impl CardFaceJson {
    fn from_card_face(face: &CardFace) -> Self {
        Self {
            name: face.name.clone(),
            type_line: face.type_line.clone(),
            mana_cost: face.mana_cost.clone(),
            image_uri: face.image_uri.clone(),
            colors: face.colors.clone(),
        }
    }

    fn into_card_face(self) -> CardFace {
        CardFace {
            name: self.name,
            type_line: self.type_line,
            mana_cost: self.mana_cost,
            image_uri: self.image_uri,
            colors: self.colors,
        }
    }
}

const BATCH_SIZE: usize = 1000;

#[async_trait::async_trait]
impl CardRepository for PostgresMatchDB {
    async fn load_cards(&self, cards: &[Card]) -> Result<()> {
        let mut tx = self.pool().begin().await?;

        sqlx::query("TRUNCATE TABLE card").execute(&mut *tx).await?;

        for chunk in cards.chunks(BATCH_SIZE) {
            let arena_ids: Vec<i64> = chunk.iter().map(|c| c.id).collect();
            let names: Vec<&str> = chunk.iter().map(|c| c.name.as_str()).collect();
            let set_codes: Vec<&str> = chunk.iter().map(|c| c.set.as_str()).collect();
            let langs: Vec<&str> = chunk.iter().map(|c| c.lang.as_str()).collect();
            let image_uris: Vec<&str> = chunk.iter().map(|c| c.image_uri.as_str()).collect();
            let mana_costs: Vec<&str> = chunk.iter().map(|c| c.mana_cost.as_str()).collect();
            let cmcs: Vec<i32> = chunk.iter().map(|c| c.cmc).collect();
            let type_lines: Vec<&str> = chunk.iter().map(|c| c.type_line.as_str()).collect();
            let layouts: Vec<&str> = chunk.iter().map(|c| c.layout.as_str()).collect();

            let colors: Vec<String> = chunk
                .iter()
                .map(|c| serde_json::to_string(&c.colors).unwrap_or_else(|_| "[]".to_string()))
                .collect();

            let color_identities: Vec<String> = chunk
                .iter()
                .map(|c| serde_json::to_string(&c.color_identity).unwrap_or_else(|_| "[]".to_string()))
                .collect();

            let card_faces_json: Vec<String> = chunk
                .iter()
                .map(|c| {
                    let faces: Vec<CardFaceJson> = c.card_faces.iter().map(CardFaceJson::from_card_face).collect();
                    serde_json::to_string(&faces).unwrap_or_else(|_| "[]".to_string())
                })
                .collect();

            sqlx::query(
                r"INSERT INTO card (arena_id, name, set_code, lang, image_uri, mana_cost, cmc, type_line, layout, colors, color_identity, card_faces)
                  SELECT * FROM UNNEST(
                      $1::bigint[], $2::text[], $3::text[], $4::text[], $5::text[],
                      $6::text[], $7::integer[], $8::text[], $9::text[], $10::text[],
                      $11::text[], $12::text[]
                  )",
            )
            .bind(&arena_ids)
            .bind(&names)
            .bind(&set_codes)
            .bind(&langs)
            .bind(&image_uris)
            .bind(&mana_costs)
            .bind(&cmcs)
            .bind(&type_lines)
            .bind(&layouts)
            .bind(&colors)
            .bind(&color_identities)
            .bind(&card_faces_json)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        info!("Loaded {} cards into database", cards.len());
        Ok(())
    }

    async fn get_card(&self, arena_id: i64) -> Result<Option<Card>> {
        let row: Option<CardRow> = sqlx::query_as(
            "SELECT arena_id, name, set_code, lang, image_uri, mana_cost, cmc, type_line, layout, colors, color_identity, card_faces FROM card WHERE arena_id = $1",
        )
        .bind(arena_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(CardRow::into_card))
    }

    async fn get_cards(&self, arena_ids: &[i64]) -> Result<Vec<Card>> {
        let rows: Vec<CardRow> = sqlx::query_as(
            "SELECT arena_id, name, set_code, lang, image_uri, mana_cost, cmc, type_line, layout, colors, color_identity, card_faces FROM card WHERE arena_id = ANY($1)",
        )
        .bind(arena_ids)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(CardRow::into_card).collect())
    }

    async fn card_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM card")
            .fetch_one(self.pool())
            .await?;
        Ok(row.0)
    }
}
