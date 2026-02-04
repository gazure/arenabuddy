use std::{collections::BTreeMap, fmt::Display, fs::File, io::Read, path::Path};

use tracingx::debug;

use crate::models::{Card, CardCollection};

const CARDS: &[u8] = include_bytes!("../data/cards-full.pb");

#[derive(Debug)]
pub struct CardsDatabase {
    pub db: BTreeMap<String, Card>,
}

impl CardsDatabase {
    /// # Errors
    ///
    /// Will return an error if the database file cannot be opened or if the database file is not a valid protobuf
    pub fn new(path: impl AsRef<Path>) -> crate::Result<Self> {
        // TODO make async
        let mut cards_db_file = File::open(path)?;
        let mut buffer = Vec::new();
        cards_db_file.read_to_end(&mut buffer)?;
        Self::from_bytes(buffer.as_slice())
    }

    /// # Errors
    ///
    /// Will error if the provided bytes do not contain a `CardsCollection` proto
    pub fn from_bytes(cards: &[u8]) -> crate::Result<Self> {
        let collection = CardCollection::decode(cards)?;
        let cards_db: BTreeMap<String, Card> = collection
            .cards
            .into_iter()
            .map(|card| (card.id.to_string(), card))
            .collect();
        debug!("loaded {} cards", cards_db.len());
        Ok(Self { db: cards_db })
    }

    /// # Errors
    ///
    /// Will return an error if the card cannot be found in the database
    pub fn get_pretty_name<T>(&self, grp_id: &T) -> Option<String>
    where
        T: AsRef<str>,
    {
        self.db.get(grp_id.as_ref()).map(|c| c.name.clone())
    }

    pub fn get<T>(&self, grp_id: &T) -> Option<&Card>
    where
        T: Display + ?Sized,
    {
        let grp_id = grp_id.to_string();
        self.db.get(&grp_id)
    }

    pub fn len(&self) -> usize {
        self.db.len()
    }

    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }
}

impl Default for CardsDatabase {
    fn default() -> Self {
        Self::from_bytes(CARDS).expect("library should ship with correct cards database")
    }
}
