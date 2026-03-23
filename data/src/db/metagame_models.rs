use chrono::NaiveDate;
use sqlx::FromRow;

#[derive(Debug, Clone)]
pub struct MetagameTournament {
    pub goldfish_id: i32,
    pub name: String,
    pub format: String,
    pub date: NaiveDate,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct MetagameArchetype {
    pub name: String,
    pub format: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MetagameDeck {
    pub goldfish_id: i32,
    pub archetype_name: Option<String>,
    pub player_name: Option<String>,
    pub placement: Option<String>,
    pub format: String,
    pub date: Option<NaiveDate>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct MetagameDeckCard {
    pub card_name: String,
    pub quantity: i32,
    pub is_sideboard: bool,
}

/// Parse a deck download text file into card entries.
/// Format: `{quantity} {card name}` lines, blank line separates mainboard from sideboard.
pub fn parse_deck_download(text: &str) -> Vec<MetagameDeckCard> {
    let mut cards = Vec::new();
    let mut is_sideboard = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !cards.is_empty() {
                is_sideboard = true;
            }
            continue;
        }

        let Some((qty_str, card_name)) = line.split_once(' ') else {
            continue;
        };

        let Ok(quantity) = qty_str.parse::<i32>() else {
            continue;
        };

        cards.push(MetagameDeckCard {
            card_name: card_name.to_string(),
            quantity,
            is_sideboard,
        });
    }

    cards
}

#[derive(Debug, FromRow)]
pub struct MetagameTournamentRow {
    pub id: i32,
    pub goldfish_id: i32,
    pub name: String,
    pub format: String,
    pub date: NaiveDate,
    pub url: String,
}

#[derive(Debug, FromRow)]
pub struct MetagameArchetypeRow {
    pub id: i32,
    pub name: String,
    pub format: String,
    pub url: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct MetagameDeckRow {
    pub id: i32,
    pub goldfish_id: i32,
    pub tournament_id: Option<i32>,
    pub archetype_id: Option<i32>,
    pub player_name: Option<String>,
    pub placement: Option<String>,
    pub format: String,
    pub date: Option<NaiveDate>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct SignatureCard {
    pub archetype_id: i32,
    pub archetype_name: String,
    pub card_name: String,
    pub weight: f32,
    pub format: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct SignatureCardRow {
    pub archetype_id: i32,
    pub archetype_name: String,
    pub card_name: String,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct MatchArchetype {
    pub match_id: String,
    pub side: String,
    pub archetype_id: Option<i32>,
    pub archetype_name: String,
    pub confidence: f32,
}

/// A match that hasn't been classified yet, with its card data.
#[derive(Debug, Clone, FromRow)]
pub struct UnclassifiedMatchRow {
    pub match_id: sqlx::types::Uuid,
    pub format: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CardFrequencyRow {
    pub archetype_id: i32,
    pub archetype_name: String,
    pub card_name: String,
    pub archetype_deck_count: i64,
    pub total_archetype_decks: i64,
    pub total_decks_with_card: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deck_download() {
        let text = "4 Lightning Bolt\n3 Counterspell\n2 Island\n\n2 Negate\n1 Spell Pierce\n";
        let cards = parse_deck_download(text);
        assert_eq!(cards.len(), 5);

        assert_eq!(cards[0].card_name, "Lightning Bolt");
        assert_eq!(cards[0].quantity, 4);
        assert!(!cards[0].is_sideboard);

        assert_eq!(cards[3].card_name, "Negate");
        assert_eq!(cards[3].quantity, 2);
        assert!(cards[3].is_sideboard);
    }

    #[test]
    fn test_parse_deck_download_empty() {
        let cards = parse_deck_download("");
        assert!(cards.is_empty());
    }

    #[test]
    fn test_parse_deck_download_no_sideboard() {
        let text = "4 Lightning Bolt\n3 Counterspell\n";
        let cards = parse_deck_download(text);
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().all(|c| !c.is_sideboard));
    }
}
