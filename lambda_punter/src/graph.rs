use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, BinaryHeap};

use super::types::SiteId;
use super::map::Map;

pub struct Graph {
    neighs: HashMap<SiteId, HashSet<SiteId>>,
}

#[derive(Default)]
pub struct GraphCache {
    pqueue: BinaryHeap<PQNode>,
    visited: HashSet<SiteId>,
    path: Vec<SiteId>,
}

impl GraphCache {
    fn clear(&mut self) {
        self.pqueue.clear();
        self.visited.clear();
        self.path.clear();
    }
}

impl Graph {
    pub fn from_map(map: &Map) -> Graph {
        Graph::from_iter(map.rivers.iter().map(|r| (r.source, r.target)))
    }

    pub fn from_iter<I>(iter: I) -> Graph where I: Iterator<Item = (SiteId, SiteId)> {
        let mut neighs = HashMap::new();
        {
            let mut add = |k, v| neighs
                .entry(k)
                .or_insert_with(HashSet::new)
                .insert(v);
            for (src, dst) in iter {
                add(src, dst);
                add(dst, src);
            }
        }
        Graph {
            neighs: neighs,
        }
    }

    pub fn shortest_path<'a>(&self, source: SiteId, target: SiteId, cache: &'a mut GraphCache) -> &'a [SiteId] {
        unimplemented!()
    }
}

#[derive(PartialEq, Eq)]
struct PQNode {
    site: SiteId,
    cost: usize,
}

impl Ord for PQNode {
    fn cmp(&self, other: &PQNode) -> Ordering {
        other.cost.cmp(&self.cost)
            .then_with(|| self.site.cmp(&other.site))
    }
}

impl PartialOrd for PQNode {
    fn partial_cmp(&self, other: &PQNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
