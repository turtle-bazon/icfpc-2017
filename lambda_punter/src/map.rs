use std::collections::{HashMap, HashSet};
use super::types::SiteId;

#[derive(PartialEq, Debug)]
pub struct Map {
    pub sites: HashMap<SiteId, Site>,
    pub rivers: HashSet<River>,
    pub mines: HashSet<SiteId>,
}

#[derive(PartialEq, Debug)]
pub struct Site {
    pub id: SiteId,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct River {
    source: SiteId,
    target: SiteId,
}
