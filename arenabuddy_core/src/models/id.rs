use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Copy, Eq, Ord)]
pub struct ArenaId(i32);

impl ArenaId {
    pub fn new(v: i32) -> Self {
        Self(v)
    }

    pub fn inner(&self) -> i32 {
        self.0
    }
}

impl From<i32> for ArenaId {
    fn from(v: i32) -> Self {
        Self(v)
    }
}

impl Serialize for ArenaId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.0)
    }
}

impl<'de> Deserialize<'de> for ArenaId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;
        Ok(ArenaId(value))
    }
}

impl Display for ArenaId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
