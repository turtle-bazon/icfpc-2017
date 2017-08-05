use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, BinaryHeap};

use super::types::SiteId;
use super::map::Map;

#[derive(Serialize, Deserialize)]
pub struct Graph {
    neighs: HashMap<SiteId, HashSet<SiteId>>,
}

#[derive(Default)]
pub struct GraphCache {
    pqueue: BinaryHeap<PQNode>,
    visited: HashSet<SiteId>,
    path_buf: Vec<(SiteId, usize)>,
    path: Vec<SiteId>,
}

impl GraphCache {
    fn clear(&mut self) {
        self.pqueue.clear();
        self.visited.clear();
        self.path_buf.clear();
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

    pub fn shortest_path<'a, F>(&self, source: SiteId, target: SiteId, cache: &'a mut GraphCache, mut accessible: F) -> Option<&'a [SiteId]>
        where F: FnMut((SiteId, SiteId)) -> bool
    {
        cache.clear();
        cache.path_buf.push((source, 0));
        cache.pqueue.push(PQNode { site: source, cost: 0, phead: 1, });
        while let Some(PQNode { site, cost: current_cost, mut phead, }) = cache.pqueue.pop() {
            if site == target {
                while phead != 0 {
                    let (site_hop, next_phead) = cache.path_buf[phead - 1];
                    cache.path.push(site_hop);
                    phead = next_phead;
                }
                cache.path.reverse();
                return Some(&cache.path);
            }
            cache.visited.insert(site);
            if let Some(neighs) = self.neighs.get(&site) {
                for &reachable_site in neighs.iter() {
                    if cache.visited.contains(&reachable_site) {
                        continue;
                    }
                    if !accessible((site, reachable_site)) {
                        continue;
                    }
                    cache.path_buf.push((reachable_site, phead));
                    cache.pqueue.push(PQNode { site: reachable_site, cost: current_cost + 1, phead: cache.path_buf.len(), });
                }
            }
        }
        None
    }
}

#[derive(PartialEq, Eq)]
struct PQNode {
    site: SiteId,
    cost: usize,
    phead: usize,
}

impl Ord for PQNode {
    fn cmp(&self, other: &PQNode) -> Ordering {
        other.cost.cmp(&self.cost)
            .then_with(|| self.site.cmp(&other.site))
            .then_with(|| self.phead.cmp(&other.phead))
    }
}

impl PartialOrd for PQNode {
    fn partial_cmp(&self, other: &PQNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use super::Graph;

    #[test]
    fn shortest_path() {
        let mut cache = Default::default();
        let graph = Graph::from_iter(
            [(3, 4), (0, 1), (2, 3), (1, 3), (5, 6), (4, 5), (3, 5), (6, 7), (5, 7), (1, 7), (0, 7), (1, 2)]
                .iter()
                .cloned());
        let path14: &[_] = &[1, 3, 4]; assert_eq!(graph.shortest_path(1, 4, &mut cache, |_| true), Some(path14));
        let path15: &[_] = &[1, 3, 5]; assert_eq!(graph.shortest_path(1, 5, &mut cache, |_| true), Some(path15));
        let path04: &[_] = &[0, 1, 3, 4]; assert_eq!(graph.shortest_path(0, 4, &mut cache, |_| true), Some(path04));
    }
}
