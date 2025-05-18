use std::{
    cmp::Ordering, collections::BTreeMap, fmt::Display, fs::File, io::Read, path::Path,
    str::FromStr,
};

use prost::Message;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::proto::{Card as ProtoCard, CardCollection, CardFace as ProtoCardFace};

use crate::models::mana::Cost;

#[derive(Debug)]
pub struct CardsDatabase {
    pub db: BTreeMap<String, Card>,
}

impl Default for CardsDatabase {
    fn default() -> Self {
        let default_path = Path::new("data/cards.json");
        Self::new(default_path).unwrap_or_else(|e| {
            error!("Error loading default cards database: {:?}", e);
            Self {
                db: BTreeMap::new(),
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardFace {
    pub name: String,
    pub type_line: String,
    pub mana_cost: Option<String>,
    pub image_uri: Option<String>,
    pub colors: Option<Vec<String>>,
}

impl CardFace {
    fn from_proto(proto_face: ProtoCardFace) -> Self {
        Self {
            name: proto_face.name,
            type_line: proto_face.type_line,
            mana_cost: if proto_face.mana_cost.is_empty() { None } else { Some(proto_face.mana_cost) },
            image_uri: proto_face.image_uri,
            colors: if proto_face.colors.is_empty() { None } else { Some(proto_face.colors) },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: i32,
    pub set: String,
    pub name: String,
    pub lang: String,
    pub image_uri: Option<String>,
    pub mana_cost: Option<String>,
    pub cmc: u8,
    pub type_line: String,
    pub layout: String,
    pub colors: Option<Vec<String>>,
    pub color_identity: Vec<String>,
    pub card_faces: Option<Vec<CardFace>>,
}

impl Card {
    fn from_proto(proto_card: ProtoCard) -> Self {
        Self {
            id: proto_card.id as i32, // Convert i64 to i32
            set: proto_card.set,
            name: proto_card.name,
            lang: proto_card.lang,
            image_uri: if proto_card.image_uri.is_empty() { None } else { Some(proto_card.image_uri) },
            mana_cost: if proto_card.mana_cost.is_empty() { None } else { Some(proto_card.mana_cost) },
            cmc: proto_card.cmc as u8, // Convert i32 to u8
            type_line: proto_card.type_line,
            layout: proto_card.layout,
            colors: if proto_card.colors.is_empty() { None } else { Some(proto_card.colors) },
            color_identity: proto_card.color_identity,
            card_faces: if proto_card.card_faces.is_empty() { 
                None 
            } else { 
                Some(proto_card.card_faces.into_iter().map(CardFace::from_proto).collect()) 
            },
        }
    }
}

impl Card {
    pub fn image_uri(&self) -> &str {
        if let Some(image_uri) = &self.image_uri {
            image_uri
        } else {
            self.card_faces
                .as_ref()
                .and_then(|faces| faces.first().map(|face| face.image_uri.as_ref()))
                .flatten()
                .map_or("", |v| v)
        }
    }

    pub fn cost(&self) -> Option<Cost> {
        self.mana_cost.as_ref().and_then(|s| s.parse::<Cost>().ok())
    }

    #[inline]
    pub fn mana_value(&self) -> u8 {
        self.cmc
    }

    fn types(&self) -> Vec<CardType> {
        let mut faces = self.type_line.split("//");
        let Some(front) = faces.next() else {
            return vec![];
        };
        front
            .trim()
            .split("—")
            .filter_map(|t| CardType::from_str(t).ok())
            .collect()
    }

    pub fn dominant_type(&self) -> CardType {
        self.types().first().copied().unwrap_or(CardType::Unknown)
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -- {}", self.name, self.set)
    }
}

impl Eq for Card {}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        let mana_value_ordering = self.mana_value().cmp(&other.mana_value());
        if mana_value_ordering == Ordering::Equal {
            self.name.cmp(&other.name)
        } else {
            mana_value_ordering
        }
    }
}

impl PartialEq<Self> for Card {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl PartialOrd<Self> for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CardsDatabase {
    /// # Errors
    ///
    /// Will return an error if the database file cannot be opened or if the database file is not a valid protobuf
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut cards_db_file = File::open(path)?;
        let mut buffer = Vec::new();
        cards_db_file.read_to_end(&mut buffer)?;
        
        let card_collection = CardCollection::decode(buffer.as_slice())?;
        let cards_db = card_collection.cards.into_iter()
            .map(|proto_card| {
                let card = Card::from_proto(proto_card);
                (card.id.to_string(), card)
            })
            .collect();

        Ok(Self { db: cards_db })
    }

    /// # Errors
    ///
    /// Will return an error if the card cannot be found in the database
    pub fn get_pretty_name<T>(&self, grp_id: &T) -> anyhow::Result<String>
    where
        T: Display + ?Sized,
    {
        let grp_id = grp_id.to_string();
        let card = self
            .db
            .get(&grp_id)
            .ok_or_else(|| anyhow::anyhow!("Card not found in database"))?;
        Ok(card.name.clone())
    }

    pub fn get_pretty_name_defaulted<T>(&self, grp_id: &T) -> String
    where
        T: Display + ?Sized,
    {
        self.get_pretty_name(grp_id)
            .unwrap_or_else(|_| grp_id.to_string())
    }

    pub fn get<T>(&self, grp_id: &T) -> Option<&Card>
    where
        T: Display + ?Sized,
    {
        let grp_id = grp_id.to_string();
        self.db.get(&grp_id)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardType {
    Creature,
    Land,
    Artifact,
    Enchantment,
    Planeswalker,
    Instant,
    Sorcery,
    Battle,
    #[default]
    Unknown,
}
impl FromStr for CardType {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "Creature" => Ok(CardType::Creature),
            "Land" | "Basic Land" => Ok(CardType::Land),
            "Artifact" => Ok(CardType::Artifact),
            "Enchantment" => Ok(CardType::Enchantment),
            "Planeswalker" => Ok(CardType::Planeswalker),
            "Instant" => Ok(CardType::Instant),
            "Sorcery" => Ok(CardType::Sorcery),
            "Battle" => Ok(CardType::Battle),
            _ => Err(Self::Err::Unknown),
        }
    }
}

impl Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CardType::Creature => "Creature",
                CardType::Land => "Land",
                CardType::Artifact => "Artifact",
                CardType::Enchantment => "Enchantment",
                CardType::Planeswalker => "Planeswalker",
                CardType::Instant => "Instant",
                CardType::Sorcery => "Sorcery",
                CardType::Battle => "Battle",
                CardType::Unknown => "Unknown",
            }
        )
    }
}
