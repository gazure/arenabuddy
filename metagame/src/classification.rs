use std::collections::HashMap;

use anyhow::Result;
use arenabuddy_data::{
    MetagameRepository,
    metagame_models::{MatchArchetype, SignatureCard},
};
use tracing::info;

/// Minimum weight threshold for a card to be considered a signature card.
const SIGNATURE_WEIGHT_THRESHOLD: f32 = 0.15;

/// Minimum total score for a classification to be considered valid.
const MIN_CLASSIFICATION_SCORE: f32 = 0.5;

/// Expected number of cards in a standard deck (used for confidence scaling).
const EXPECTED_DECK_SIZE: f32 = 60.0;

/// Compute signature cards for all archetypes in a given format.
///
/// For each card in each archetype, computes:
/// - `frequency`: fraction of the archetype's decks containing this card
/// - `exclusivity`: fraction of all decks with this card that belong to this archetype
/// - `weight = frequency * exclusivity`
///
/// Cards above the threshold are saved as signature cards.
pub async fn compute_signature_cards(repo: &impl MetagameRepository, format: &str) -> Result<u64> {
    info!("Computing signature cards for format: {format}");

    let frequencies = repo.get_card_frequencies(format).await?;
    info!("Fetched {} card-archetype frequency rows", frequencies.len());

    let total_decks_in_format: i64 = frequencies.iter().map(|r| r.total_archetype_decks).max().unwrap_or(0);

    if total_decks_in_format == 0 {
        info!("No decks found for format {format}, skipping");
        return Ok(0);
    }

    let mut signature_cards = Vec::new();

    for row in &frequencies {
        if row.total_archetype_decks == 0 || row.total_decks_with_card == 0 {
            continue;
        }

        #[expect(clippy::cast_precision_loss)]
        let frequency = row.archetype_deck_count as f32 / row.total_archetype_decks as f32;
        #[expect(clippy::cast_precision_loss)]
        let exclusivity = row.archetype_deck_count as f32 / row.total_decks_with_card as f32;
        let weight = frequency * exclusivity;

        if weight >= SIGNATURE_WEIGHT_THRESHOLD {
            signature_cards.push(SignatureCard {
                archetype_id: row.archetype_id,
                archetype_name: row.archetype_name.clone(),
                card_name: row.card_name.clone(),
                weight,
                format: format.to_string(),
            });
        }
    }

    info!("Found {} signature cards across archetypes", signature_cards.len());

    let count = repo.replace_signature_cards(format, &signature_cards).await?;
    info!("Stored {count} signature cards for {format}");

    Ok(count)
}

/// Classify all unclassified matches in a given format using signature cards.
///
/// For each match, maps the controller and opponent decks' card names
/// against known signature cards, scoring each archetype by the sum of
/// matching signature card weights.
pub async fn classify_matches(repo: &impl MetagameRepository, format: &str) -> Result<u64> {
    let signature_cards = repo.get_signature_cards(format).await?;
    if signature_cards.is_empty() {
        info!("No signature cards found for {format}. Run compute-signatures first.");
        return Ok(0);
    }

    // Build lookup: card_name -> Vec<(archetype_id, archetype_name, weight)>
    let mut card_to_archetypes: HashMap<String, Vec<(i32, String, f32)>> = HashMap::new();
    for sc in &signature_cards {
        card_to_archetypes.entry(sc.card_name.clone()).or_default().push((
            sc.archetype_id,
            sc.archetype_name.clone(),
            sc.weight,
        ));
    }

    let unclassified = repo.get_unclassified_matches(format).await?;
    info!("Found {} unclassified matches for {format}", unclassified.len());

    let mut classified_count = 0u64;

    for m in &unclassified {
        let match_id = m.match_id.to_string();

        // Classify controller deck
        let controller_cards = repo.get_match_deck_cards(&match_id).await?;
        if let Some(archetype) = score_deck(&controller_cards, &card_to_archetypes, 1.0) {
            repo.upsert_match_archetype(&MatchArchetype {
                match_id: match_id.clone(),
                side: "controller".to_string(),
                archetype_id: Some(archetype.0),
                archetype_name: archetype.1.clone(),
                confidence: archetype.2,
            })
            .await?;
        }

        // Classify opponent deck
        let opponent_cards = repo.get_match_opponent_cards(&match_id).await?;
        if !opponent_cards.is_empty() {
            // Scale confidence by how complete the opponent deck observation is
            #[expect(clippy::cast_precision_loss)]
            let completeness = (opponent_cards.len() as f32 / EXPECTED_DECK_SIZE).min(1.0);
            if let Some(archetype) = score_deck(&opponent_cards, &card_to_archetypes, completeness) {
                repo.upsert_match_archetype(&MatchArchetype {
                    match_id: match_id.clone(),
                    side: "opponent".to_string(),
                    archetype_id: Some(archetype.0),
                    archetype_name: archetype.1.clone(),
                    confidence: archetype.2,
                })
                .await?;
            }
        }

        classified_count += 1;
    }

    info!("Classified {classified_count} matches for {format}");
    Ok(classified_count)
}

/// Score a deck's cards against signature card data.
/// Returns the best matching archetype as `(archetype_id, archetype_name, confidence)`.
fn score_deck(
    card_names: &[String],
    card_to_archetypes: &HashMap<String, Vec<(i32, String, f32)>>,
    confidence_scale: f32,
) -> Option<(i32, String, f32)> {
    let mut archetype_scores: HashMap<i32, (String, f32)> = HashMap::new();

    for card_name in card_names {
        if let Some(archetypes) = card_to_archetypes.get(card_name) {
            for (archetype_id, archetype_name, weight) in archetypes {
                let entry = archetype_scores
                    .entry(*archetype_id)
                    .or_insert_with(|| (archetype_name.clone(), 0.0));
                entry.1 += weight;
            }
        }
    }

    archetype_scores
        .into_iter()
        .max_by(|a, b| a.1.1.partial_cmp(&b.1.1).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|(_, (_, score))| *score >= MIN_CLASSIFICATION_SCORE)
        .map(|(id, (name, score))| (id, name, score * confidence_scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_deck_empty() {
        let card_to_archetypes = HashMap::new();
        let result = score_deck(&[], &card_to_archetypes, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_score_deck_finds_best_match() {
        let mut card_to_archetypes: HashMap<String, Vec<(i32, String, f32)>> = HashMap::new();
        card_to_archetypes.insert("Lightning Bolt".to_string(), vec![(1, "Mono Red".to_string(), 0.8)]);
        card_to_archetypes.insert(
            "Monastery Swiftspear".to_string(),
            vec![(1, "Mono Red".to_string(), 0.9)],
        );
        card_to_archetypes.insert("Counterspell".to_string(), vec![(2, "Blue Control".to_string(), 0.7)]);

        let deck = vec![
            "Lightning Bolt".to_string(),
            "Monastery Swiftspear".to_string(),
            "Mountain".to_string(),
        ];

        let result = score_deck(&deck, &card_to_archetypes, 1.0);
        assert!(result.is_some());
        let (id, name, _score) = result.unwrap();
        assert_eq!(id, 1);
        assert_eq!(name, "Mono Red");
    }

    #[test]
    fn test_score_deck_below_threshold() {
        let mut card_to_archetypes: HashMap<String, Vec<(i32, String, f32)>> = HashMap::new();
        card_to_archetypes.insert("Mountain".to_string(), vec![(1, "Mono Red".to_string(), 0.1)]);

        let deck = vec!["Mountain".to_string()];

        let result = score_deck(&deck, &card_to_archetypes, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_score_deck_confidence_scaling() {
        let mut card_to_archetypes: HashMap<String, Vec<(i32, String, f32)>> = HashMap::new();
        card_to_archetypes.insert("Lightning Bolt".to_string(), vec![(1, "Mono Red".to_string(), 0.8)]);
        card_to_archetypes.insert(
            "Monastery Swiftspear".to_string(),
            vec![(1, "Mono Red".to_string(), 0.9)],
        );

        let deck = vec!["Lightning Bolt".to_string(), "Monastery Swiftspear".to_string()];

        let full = score_deck(&deck, &card_to_archetypes, 1.0).unwrap();
        let partial = score_deck(&deck, &card_to_archetypes, 0.25).unwrap();

        assert!(partial.2 < full.2);
    }
}
