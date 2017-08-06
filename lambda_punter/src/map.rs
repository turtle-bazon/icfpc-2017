use std::cmp::{min, max};
use std::collections::HashMap;
use super::types::SiteId;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub sites: Vec<SiteId>,
    pub rivers: Vec<River>,
    pub mines: Vec<SiteId>,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct River {
    pub source: SiteId,
    pub target: SiteId,
}

impl River {
    pub fn new(source: SiteId, target: SiteId) -> River {
        River {
            source: min(source, target),
            target: max(source, target),
        }
    }
}

pub struct RiversIndex<T>(HashMap<River, T>);

impl<T> RiversIndex<T> {
    pub fn new() -> RiversIndex<T> {
        RiversIndex(HashMap::new())
    }

    pub fn from_hash_map(map: HashMap<River, T>) -> RiversIndex<T> {
        RiversIndex(map)
    }
}

use std::ops::{Deref, DerefMut};

impl<T> Deref for RiversIndex<T> {
    type Target = HashMap<River, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RiversIndex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

use serde::{ser, de};

impl<T> ser::Serialize for RiversIndex<T> where T: ser::Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: ser::Serializer {
        let vec: Vec<_> = self.iter().collect();
        vec.serialize(serializer)
    }
}

impl<'de, T> de::Deserialize<'de> for RiversIndex<T> where T: de::Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: de::Deserializer<'de> {
        let vec: Vec<(River, T)> = de::Deserialize::deserialize(deserializer)?;
        Ok(RiversIndex(vec.into_iter().collect()))
    }
}
