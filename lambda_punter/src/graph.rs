use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, BinaryHeap};

use super::types::SiteId;
use super::map::{Map, River};

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

pub enum EdgeAttr {
    Blocked,
    Accessible { edge_cost: usize, },
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

    pub fn shortest_path_only<'a>(&self, source: SiteId, target: SiteId, cache: &'a mut GraphCache) -> Option<&'a [SiteId]> {
        self.shortest_path(source, target, cache, |_| EdgeAttr::Accessible { edge_cost: 1, })
    }

    pub fn shortest_path<'a, E>(&self, source: SiteId, target: SiteId, cache: &'a mut GraphCache, mut probe_edge: E) -> Option<&'a [SiteId]>
        where E: FnMut((SiteId, SiteId)) -> EdgeAttr
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
                    match probe_edge((site, reachable_site)) {
                        EdgeAttr::Blocked =>
                            continue,
                        EdgeAttr::Accessible { edge_cost, } => {
                            cache.path_buf.push((reachable_site, phead));
                            cache.pqueue.push(PQNode {
                                site: reachable_site,
                                cost: current_cost + edge_cost,
                                phead: cache.path_buf.len(),
                            });
                        },
                    }
                }
            }
        }
        None
    }

    // The Girvan-Newman Algorithm
    pub fn rivers_betweenness(&self, cache: &mut GraphCache) -> HashMap<River, f64> {
        let mut rivers = HashMap::new();
        let mut visit_cache = HashMap::new();
        let mut visit_rev = BinaryHeap::new();
        for (&node, _) in self.neighs.iter() {
            self.rivers_betweenness_pass(node, &mut rivers, &mut visit_cache, &mut visit_rev, cache);
        }
        for betweenness2 in rivers.values_mut() {
            *betweenness2 /= 2.0;
        }
        rivers
    }

    fn rivers_betweenness_pass(
        &self,
        start_node: SiteId,
        rivers: &mut HashMap<River, f64>,
        visit_cache: &mut HashMap<SiteId, BssVisit>,
        visit_rev: &mut BinaryHeap<(usize, SiteId)>,
        cache: &mut GraphCache)
    {
        visit_cache.clear();
        visit_rev.clear();
        cache.clear();
        cache.pqueue.push(PQNode { site: start_node, cost: 0, phead: 0, });
        // forward pass
        while let Some(PQNode { site, cost: parent_cost, .. }) = cache.pqueue.pop() {
            let parent_count = {
                let site_visit = visit_cache.entry(site)
                    .or_insert_with(|| BssVisit {
                        visited: false,
                        cost: parent_cost,
                        paths_count: 1,
                        credits: 1.0,
                    });
                if site_visit.visited {
                    continue;
                } else {
                    site_visit.visited = true;
                    visit_rev.push((parent_cost, site));
                    site_visit.paths_count
                }
            };
            if let Some(neighs) = self.neighs.get(&site) {
                let children_cost = parent_cost + 1;
                for &reachable_site in neighs.iter() {
                    let visit = visit_cache.entry(reachable_site)
                        .or_insert_with(|| BssVisit {
                            visited: false,
                            cost: children_cost,
                            paths_count: 0,
                            credits: 1.0,
                        });
                    if visit.cost > parent_cost {
                        visit.paths_count += parent_count;
                    }
                    if !visit.visited {
                        cache.pqueue.push(PQNode {
                            site: reachable_site,
                            cost: children_cost,
                            phead: 0,
                        })
                    }
                }
            }
        }

        // backward pass
        while let Some((cost, node)) = visit_rev.pop() {
            let credits = if let Some(visit) = visit_cache.get(&node) {
                visit.credits
            } else {
                continue;
            };
            if let Some(neighs) = self.neighs.get(&node) {
                let mut parents_paths_sum = 0;
                for neigh in neighs.iter() {
                    if let Some(parent) = visit_cache.get(neigh) {
                        if parent.cost >= cost {
                            // skip non DAG nodes
                            continue;
                        }
                        parents_paths_sum += parent.paths_count;
                    }
                }
                if parents_paths_sum == 0 {
                    continue;
                }
                for &neigh in neighs.iter() {
                    if let Some(parent) = visit_cache.get_mut(&neigh) {
                        if parent.cost >= cost {
                            // skip non DAG nodes
                            continue;
                        }
                        let river = River::new(node, neigh);
                        let river_credit = credits * parent.paths_count as f64 / parents_paths_sum as f64;
                        *rivers.entry(river).or_insert(0.0) += river_credit;
                        parent.credits += river_credit;
                    }
                }
            }
        }
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

#[derive(Debug)]
struct BssVisit {
    visited: bool,
    cost: usize,
    paths_count: usize,
    credits: f64,
}

#[cfg(test)]
mod test {
    use super::super::types::SiteId;
    use super::{Graph, EdgeAttr};

    fn sample_map() -> Graph {
        Graph::from_iter(
            [(3, 4), (0, 1), (2, 3), (1, 3), (5, 6), (4, 5), (3, 5), (6, 7), (5, 7), (1, 7), (0, 7), (1, 2)]
                .iter()
                .cloned())
    }

    #[test]
    fn shortest_path() {
        let mut cache = Default::default();
        let graph = sample_map();
        let path14: &[_] = &[1, 3, 4]; assert_eq!(graph.shortest_path_only(1, 4, &mut cache), Some(path14));
        let path15: &[_] = &[1, 3, 5]; assert_eq!(graph.shortest_path_only(1, 5, &mut cache), Some(path15));
        let path04: &[_] = &[0, 1, 3, 4]; assert_eq!(graph.shortest_path_only(0, 4, &mut cache), Some(path04));
    }

    #[test]
    fn shortest_path_with_custom_costs() {
        let mut cache = Default::default();
        let graph = Graph::from_iter(
            [(3, 4), (0, 1), (2, 3), (1, 3), (5, 6), (4, 5), (3, 5), (6, 7), (5, 7), (1, 7), (0, 7), (1, 2)]
                .iter()
                .cloned());
        fn edge_probe((s, t): (SiteId, SiteId)) -> EdgeAttr {
            if ((s == 1) && (t == 3)) || ((s == 3) && (t == 1)) {
                EdgeAttr::Accessible { edge_cost: 3, }
            } else if ((s == 0) && (t == 1)) || ((s == 1) && (t == 0)) {
                EdgeAttr::Blocked
            } else {
                EdgeAttr::Accessible { edge_cost: 1, }
            }
        }

        let path14: &[_] = &[1, 2, 3, 4]; assert_eq!(graph.shortest_path(1, 4, &mut cache, edge_probe), Some(path14));
        let path15: &[_] = &[1, 7, 5]; assert_eq!(graph.shortest_path(1, 5, &mut cache, edge_probe), Some(path15));
        let path04: &[_] = &[0, 7, 5, 4]; assert_eq!(graph.shortest_path(0, 4, &mut cache, edge_probe), Some(path04));
    }

    #[test]
    fn betweenness_mmds() {
        let mut cache = Default::default();
        let graph = Graph::from_iter(
            [(0, 1), (0, 2), (1, 2), (1, 3), (3, 4), (3, 5), (3, 6), (4, 5), (5, 6)]
                .iter()
                .cloned());
        let b_rivers = graph.rivers_betweenness(&mut cache);
        let mut vb_rivers: Vec<_> = b_rivers
            .into_iter()
            .map(|(r, v)| ((r.source, r.target), v))
            .collect();
        vb_rivers.sort_by_key(|p| p.0);
        assert_eq!(vb_rivers, vec![((0, 1), 5.0),
                                   ((0, 2), 1.0),
                                   ((1, 2), 5.0),
                                   ((1, 3), 12.0),
                                   ((3, 4), 4.5),
                                   ((3, 5), 4.0),
                                   ((3, 6), 4.5),
                                   ((4, 5), 1.5),
                                   ((5, 6), 1.5)]);
    }

    #[test]
    fn betweenness_sample_map() {
        let mut cache = Default::default();
        let graph = sample_map();
        let b_rivers = graph.rivers_betweenness(&mut cache);
        let mut vb_rivers: Vec<_> = b_rivers
            .into_iter()
            .map(|(r, v)| ((r.source, r.target), v))
            .collect();
        vb_rivers.sort_by_key(|p| p.0);
        assert_eq!(vb_rivers, vec![((0, 1), 3.5),
                                   ((0, 7), 3.5),
                                   ((1, 2), 3.5),
                                   ((1, 3), 4.5),
                                   ((1, 7), 4.5),
                                   ((2, 3), 3.5),
                                   ((3, 4), 3.5),
                                   ((3, 5), 4.5),
                                   ((4, 5), 3.5),
                                   ((5, 6), 3.5),
                                   ((5, 7), 4.5),
                                   ((6, 7), 3.5)]);
    }
}
