use std::collections::{HashMap, HashSet};
use super::types::SiteId;

pub struct Map {
    pub sites: HashMap<SiteId, Site>,
    pub rivers: HashSet<River>,
    pub mines: HashSet<SiteId>,
}

pub struct Site {
    pub id: SiteId,
}

#[derive(PartialEq, Eq, Hash)]
pub struct River {
    source: SiteId,
    target: SiteId,
}
