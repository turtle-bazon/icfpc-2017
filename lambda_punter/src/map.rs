use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};
use super::types::SiteId;


#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub sites: HashMap<SiteId, Site>,
    pub rivers: HashSet<River>,
    pub mines: HashSet<SiteId>,
}


#[derive(Debug,PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub id: SiteId,
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
