use arenabuddy_core::models::{Card, CardFace, Legalities};
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
    oracle_text: String,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
    defense: Option<String>,
    keywords: String,
    produced_mana: String,
    rarity: String,
    collector_number: String,
    set_name: String,
    artist: String,
    flavor_text: String,
    legalities: String,
    edhrec_rank: Option<i32>,
    penny_rank: Option<i32>,
    oracle_id: String,
    scryfall_id: String,
    scryfall_uri: String,
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
        let keywords: Vec<String> = serde_json::from_str(&self.keywords).unwrap_or_else(|e| {
            warn!("Failed to parse keywords for card {}: {e}", self.arena_id);
            Vec::new()
        });
        let produced_mana: Vec<String> = serde_json::from_str(&self.produced_mana).unwrap_or_else(|e| {
            warn!("Failed to parse produced_mana for card {}: {e}", self.arena_id);
            Vec::new()
        });
        let legalities: Option<Legalities> = serde_json::from_str(&self.legalities).unwrap_or_else(|e| {
            warn!("Failed to parse legalities for card {}: {e}", self.arena_id);
            None
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
            oracle_text: self.oracle_text,
            power: self.power,
            toughness: self.toughness,
            loyalty: self.loyalty,
            defense: self.defense,
            keywords,
            produced_mana,
            rarity: self.rarity,
            collector_number: self.collector_number,
            set_name: self.set_name,
            artist: self.artist,
            flavor_text: self.flavor_text,
            legalities,
            edhrec_rank: self.edhrec_rank,
            penny_rank: self.penny_rank,
            oracle_id: self.oracle_id,
            scryfall_id: self.scryfall_id,
            scryfall_uri: self.scryfall_uri,
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
    #[serde(default)]
    oracle_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    toughness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    loyalty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    defense: Option<String>,
    #[serde(default)]
    flavor_text: String,
    #[serde(default)]
    artist: String,
}

impl CardFaceJson {
    fn from_card_face(face: &CardFace) -> Self {
        Self {
            name: face.name.clone(),
            type_line: face.type_line.clone(),
            mana_cost: face.mana_cost.clone(),
            image_uri: face.image_uri.clone(),
            colors: face.colors.clone(),
            oracle_text: face.oracle_text.clone(),
            power: face.power.clone(),
            toughness: face.toughness.clone(),
            loyalty: face.loyalty.clone(),
            defense: face.defense.clone(),
            flavor_text: face.flavor_text.clone(),
            artist: face.artist.clone(),
        }
    }

    fn into_card_face(self) -> CardFace {
        CardFace {
            name: self.name,
            type_line: self.type_line,
            mana_cost: self.mana_cost,
            image_uri: self.image_uri,
            colors: self.colors,
            oracle_text: self.oracle_text,
            power: self.power,
            toughness: self.toughness,
            loyalty: self.loyalty,
            defense: self.defense,
            flavor_text: self.flavor_text,
            artist: self.artist,
        }
    }
}

const BATCH_SIZE: usize = 1000;

macro_rules! card_columns {
    () => {
        "arena_id, name, set_code, lang, image_uri, mana_cost, cmc, type_line, layout, colors, \
         color_identity, card_faces, oracle_text, power, toughness, loyalty, defense, keywords, \
         produced_mana, rarity, collector_number, set_name, artist, flavor_text, legalities, \
         edhrec_rank, penny_rank, oracle_id, scryfall_id, scryfall_uri"
    };
}

fn to_json_or<T: serde::Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string())
}

async fn insert_card_chunk(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, chunk: &[Card]) -> Result<()> {
    let arena_ids: Vec<i64> = chunk.iter().map(|c| c.id).collect();
    let names: Vec<&str> = chunk.iter().map(|c| c.name.as_str()).collect();
    let set_codes: Vec<&str> = chunk.iter().map(|c| c.set.as_str()).collect();
    let langs: Vec<&str> = chunk.iter().map(|c| c.lang.as_str()).collect();
    let image_uris: Vec<&str> = chunk.iter().map(|c| c.image_uri.as_str()).collect();
    let mana_costs: Vec<&str> = chunk.iter().map(|c| c.mana_cost.as_str()).collect();
    let cmcs: Vec<i32> = chunk.iter().map(|c| c.cmc).collect();
    let type_lines: Vec<&str> = chunk.iter().map(|c| c.type_line.as_str()).collect();
    let layouts: Vec<&str> = chunk.iter().map(|c| c.layout.as_str()).collect();
    let colors: Vec<String> = chunk.iter().map(|c| to_json_or(&c.colors, "[]")).collect();
    let color_identities: Vec<String> = chunk.iter().map(|c| to_json_or(&c.color_identity, "[]")).collect();

    let card_faces_json: Vec<String> = chunk
        .iter()
        .map(|c| {
            let faces: Vec<CardFaceJson> = c.card_faces.iter().map(CardFaceJson::from_card_face).collect();
            to_json_or(&faces, "[]")
        })
        .collect();

    let oracle_texts: Vec<&str> = chunk.iter().map(|c| c.oracle_text.as_str()).collect();
    let powers: Vec<Option<&str>> = chunk.iter().map(|c| c.power.as_deref()).collect();
    let toughnesses: Vec<Option<&str>> = chunk.iter().map(|c| c.toughness.as_deref()).collect();
    let loyalties: Vec<Option<&str>> = chunk.iter().map(|c| c.loyalty.as_deref()).collect();
    let defenses: Vec<Option<&str>> = chunk.iter().map(|c| c.defense.as_deref()).collect();
    let keywords: Vec<String> = chunk.iter().map(|c| to_json_or(&c.keywords, "[]")).collect();
    let produced_mana: Vec<String> = chunk.iter().map(|c| to_json_or(&c.produced_mana, "[]")).collect();
    let rarities: Vec<&str> = chunk.iter().map(|c| c.rarity.as_str()).collect();
    let collector_numbers: Vec<&str> = chunk.iter().map(|c| c.collector_number.as_str()).collect();
    let set_names: Vec<&str> = chunk.iter().map(|c| c.set_name.as_str()).collect();
    let artists: Vec<&str> = chunk.iter().map(|c| c.artist.as_str()).collect();
    let flavor_texts: Vec<&str> = chunk.iter().map(|c| c.flavor_text.as_str()).collect();
    let legalities: Vec<String> = chunk.iter().map(|c| to_json_or(&c.legalities, "null")).collect();
    let edhrec_ranks: Vec<Option<i32>> = chunk.iter().map(|c| c.edhrec_rank).collect();
    let penny_ranks: Vec<Option<i32>> = chunk.iter().map(|c| c.penny_rank).collect();
    let oracle_ids: Vec<&str> = chunk.iter().map(|c| c.oracle_id.as_str()).collect();
    let scryfall_ids: Vec<&str> = chunk.iter().map(|c| c.scryfall_id.as_str()).collect();
    let scryfall_uris: Vec<&str> = chunk.iter().map(|c| c.scryfall_uri.as_str()).collect();

    sqlx::query(concat!(
        "INSERT INTO card (",
        card_columns!(),
        ")
          SELECT * FROM UNNEST(
              $1::bigint[], $2::text[], $3::text[], $4::text[], $5::text[],
              $6::text[], $7::integer[], $8::text[], $9::text[], $10::text[],
              $11::text[], $12::text[], $13::text[], $14::text[], $15::text[],
              $16::text[], $17::text[], $18::text[], $19::text[], $20::text[],
              $21::text[], $22::text[], $23::text[], $24::text[], $25::text[],
              $26::integer[], $27::integer[], $28::text[], $29::text[], $30::text[]
          )"
    ))
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
    .bind(&oracle_texts)
    .bind(&powers)
    .bind(&toughnesses)
    .bind(&loyalties)
    .bind(&defenses)
    .bind(&keywords)
    .bind(&produced_mana)
    .bind(&rarities)
    .bind(&collector_numbers)
    .bind(&set_names)
    .bind(&artists)
    .bind(&flavor_texts)
    .bind(&legalities)
    .bind(&edhrec_ranks)
    .bind(&penny_ranks)
    .bind(&oracle_ids)
    .bind(&scryfall_ids)
    .bind(&scryfall_uris)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[async_trait::async_trait]
impl CardRepository for PostgresMatchDB {
    async fn load_cards(&self, cards: &[Card]) -> Result<()> {
        let mut tx = self.pool().begin().await?;

        sqlx::query("TRUNCATE TABLE card").execute(&mut *tx).await?;

        for chunk in cards.chunks(BATCH_SIZE) {
            insert_card_chunk(&mut tx, chunk).await?;
        }

        tx.commit().await?;
        info!("Loaded {} cards into database", cards.len());
        Ok(())
    }

    async fn get_card(&self, arena_id: i64) -> Result<Option<Card>> {
        let row: Option<CardRow> =
            sqlx::query_as(concat!("SELECT ", card_columns!(), " FROM card WHERE arena_id = $1"))
                .bind(arena_id)
                .fetch_optional(self.pool())
                .await?;

        Ok(row.map(CardRow::into_card))
    }

    async fn get_cards(&self, arena_ids: &[i64]) -> Result<Vec<Card>> {
        let rows: Vec<CardRow> = sqlx::query_as(concat!(
            "SELECT ",
            card_columns!(),
            " FROM card WHERE arena_id = ANY($1)"
        ))
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
