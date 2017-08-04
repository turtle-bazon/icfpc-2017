use std::collections::{HashMap, HashSet};
use super::types::SiteId;

#[derive(Debug,PartialEq, Deserialize)]
pub struct Map {
    pub sites: HashMap<SiteId, Site>,
    pub rivers: HashSet<River>,
    pub mines: HashSet<SiteId>,
}

#[derive(Debug,PartialEq, Deserialize)]
pub struct Site {
    pub id: SiteId,
}

#[derive(PartialEq, Eq, Hash, Debug, Deserialize)]
pub struct River {
    pub source: SiteId,
    pub target: SiteId,
}
