use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, BinaryHeap};

use super::types::SiteId;
use super::map::{Map, River};

#[derive(Serialize, Deserialize)]
pub struct Graph {
    neighs: HashMap<SiteId, HashSet<SiteId>>,
}

enum Visit {
    Visited,
    NotYetVisited(usize),
}

#[derive(Default)]
pub struct GraphCache<S = ()> {
    pqueue: BinaryHeap<PQNode<S>>,
    visited: HashMap<SiteId, Visit>,
    path_buf: Vec<(SiteId, usize)>,
    path: Vec<SiteId>,
}

impl<S> GraphCache<S> {
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

impl EdgeAttr {
    pub fn standard(_: (SiteId, SiteId)) -> EdgeAttr {
        EdgeAttr::Accessible { edge_cost: 1, }
    }
}

pub enum StepCommand<S> {
    Continue(S),
    Stop,
    Terminate,
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

    pub fn shortest_path_only<'a, S>(&self, source: SiteId, target: SiteId, cache: &'a mut GraphCache<S>) -> Option<&'a [SiteId]>
        where S: Default + Clone
    {
        self.shortest_path(source, target, cache, EdgeAttr::standard)
    }

    pub fn shortest_path<'a, E, S>(
        &self,
        source: SiteId,
        target: SiteId,
        cache: &'a mut GraphCache<S>,
        probe_edge: E
    )
        -> Option<&'a [SiteId]>
        where E: Fn((SiteId, SiteId)) -> EdgeAttr,
              S: Default + Clone,
    {
        self.generic_bfs(source, Default::default(), |path, _cost, _seed| {
            if let Some(&pt) = path.last() {
                if pt == target {
                    StepCommand::Terminate
                } else {
                    StepCommand::Continue(Default::default())
                }
            } else {
                StepCommand::Stop
            }
        }, probe_edge, cache)
    }

    pub fn longest_jouney_from<S>(&self, source: SiteId, cache: &mut GraphCache<S>) -> Option<Vec<SiteId>> where S: Default + Clone {
        let mut best = None;
        self.generic_bfs(source, Default::default(), |path, cost, _seed| {
            best = Some(if let Some((best_cost, best_path)) = best.take() {
                if best_cost < cost {
                    (cost, path.to_owned())
                } else {
                    (best_cost, best_path)
                }
            } else {
                (cost, path.to_owned())
            });
            StepCommand::Continue(Default::default())
        }, EdgeAttr::standard, cache);
        best.map(|v| v.1)
    }

    pub fn generic_bfs<'a, S, F, E>(
        &self,
        source: SiteId,
        source_seed: S,
        mut step_fn: F,
        probe_edge: E,
        cache: &'a mut GraphCache<S>
    )
        -> Option<&'a [SiteId]>
        where F: FnMut(&[SiteId], usize, &S) -> StepCommand<S>,
              E: Fn((SiteId, SiteId)) -> EdgeAttr,
              S: Clone,
    {
        cache.clear();
        cache.path_buf.push((source, 0));
        cache.pqueue.push(PQNode { site: source, cost: 0, phead: 1, seed: source_seed, });
        while let Some(PQNode { site, cost: current_cost, phead: current_phead, seed, }) = cache.pqueue.pop() {
            // check if node is visited
            match cache.visited.get(&site) {
                Some(&Visit::NotYetVisited(prev_cost)) if current_cost > prev_cost =>
                    continue,
                _ =>
                    (),
            }
            cache.visited.insert(site, Visit::Visited);

            // restore full path
            cache.path.clear();
            let mut phead = current_phead;
            while phead != 0 {
                let (site_hop, next_phead) = cache.path_buf[phead - 1];
                cache.path.push(site_hop);
                phead = next_phead;
            }
            cache.path.reverse();

            // check if we should stop here
            let next_seed = match step_fn(&cache.path, current_cost, &seed) {
                StepCommand::Terminate =>
                    return Some(&cache.path),
                StepCommand::Stop =>
                    continue,
                StepCommand::Continue(next_seed) =>
                    next_seed,
            };

            // proceed with neighbours
            if let Some(neighs) = self.neighs.get(&site) {
                let next_cost = current_cost + 1;
                for &reachable_site in neighs.iter() {
                    match cache.visited.get(&reachable_site) {
                        None =>
                            (),
                        Some(&Visit::NotYetVisited(prev_cost)) if next_cost < prev_cost =>
                            (),
                        _ =>
                            continue,
                    }
                    match probe_edge((site, reachable_site)) {
                        EdgeAttr::Blocked =>
                            continue,
                        EdgeAttr::Accessible { edge_cost, } => {
                            cache.visited.insert(reachable_site, Visit::NotYetVisited(next_cost));
                            cache.path_buf.push((reachable_site, current_phead));
                            cache.pqueue.push(PQNode {
                                site: reachable_site,
                                cost: current_cost + edge_cost,
                                phead: cache.path_buf.len(),
                                seed: next_seed.clone(),
                            });
                        },
                    }
                }
            }
        }
        None
    }

    // The Girvan-Newman Algorithm
    pub fn rivers_betweenness<S>(&self, cache: &mut GraphCache<S>) -> HashMap<River, f64> where S: Default {
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

    fn rivers_betweenness_pass<S>(
        &self,
        start_node: SiteId,
        rivers: &mut HashMap<River, f64>,
        visit_cache: &mut HashMap<SiteId, BssVisit>,
        visit_rev: &mut BinaryHeap<(usize, SiteId)>,
        cache: &mut GraphCache<S>)
        where S: Default
    {
        visit_cache.clear();
        visit_rev.clear();
        cache.clear();
        cache.pqueue.push(PQNode { site: start_node, cost: 0, phead: 0, ..Default::default() });
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
                            ..Default::default()
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

#[derive(Default)]
struct PQNode<S = ()> {
    site: SiteId,
    cost: usize,
    phead: usize,
    seed: S,
}

impl<S> PartialEq for PQNode<S> {
    fn eq(&self, other: &PQNode<S>) -> bool {
        self.site == other.site && self.cost == other.cost && self.phead == other.phead
    }
}

impl<S> Eq for PQNode<S> {}

impl<S> Ord for PQNode<S> {
    fn cmp(&self, other: &PQNode<S>) -> Ordering {
        other.cost.cmp(&self.cost)
            .then_with(|| self.site.cmp(&other.site))
            .then_with(|| self.phead.cmp(&other.phead))
    }
}

impl<S> PartialOrd for PQNode<S> {
    fn partial_cmp(&self, other: &PQNode<S>) -> Option<Ordering> {
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
    use super::super::test_common::*;
    use super::{Graph, EdgeAttr};

    #[test]
    fn shortest_path() {
        let mut cache = Default::default();
        let graph = sample_map_graph();
        let path14: &[_] = &[1, 3, 4]; assert_eq!(graph.shortest_path_only::<()>(1, 4, &mut cache), Some(path14));
        let path15: &[_] = &[1, 7, 5]; assert_eq!(graph.shortest_path_only::<()>(1, 5, &mut cache), Some(path15));
        let path04: &[_] = &[0, 7, 5, 4]; assert_eq!(graph.shortest_path_only::<()>(0, 4, &mut cache), Some(path04));
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

        let path14: &[_] = &[1, 7, 5, 4]; assert_eq!(graph.shortest_path::<_, ()>(1, 4, &mut cache, edge_probe), Some(path14));
        let path15: &[_] = &[1, 7, 5]; assert_eq!(graph.shortest_path::<_, ()>(1, 5, &mut cache, edge_probe), Some(path15));
        let path04: &[_] = &[0, 7, 5, 4]; assert_eq!(graph.shortest_path::<_, ()>(0, 4, &mut cache, edge_probe), Some(path04));
    }

    #[test]
    fn betweenness_mmds() {
        let mut cache = Default::default();
        let graph = Graph::from_iter(
            [(0, 1), (0, 2), (1, 2), (1, 3), (3, 4), (3, 5), (3, 6), (4, 5), (5, 6)]
                .iter()
                .cloned());
        let b_rivers = graph.rivers_betweenness::<()>(&mut cache);
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
    fn longest_jouney() {
        let mut cache = Default::default();
        let graph = sample_map_graph();
        assert_eq!(graph.longest_jouney_from::<()>(6, &mut cache).and_then(|p| p.last().cloned()), Some(2));
    }

    #[test]
    fn betweenness_sample_map() {
        let mut cache = Default::default();
        let graph = sample_map_graph();
        let b_rivers = graph.rivers_betweenness::<()>(&mut cache);
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

    #[test]
    fn betweenness_random_medium_map() {
        let mut cache = Default::default();
        let graph = random_medium_map_graph();
        let b_rivers = graph.rivers_betweenness::<()>(&mut cache);
        let mut vb_rivers: Vec<_> = b_rivers
            .into_iter()
            .map(|(r, v)| ((r.source, r.target), (v * 1000.0) as usize))
            .collect();
        vb_rivers.sort_by_key(|p| p.0);
        assert_eq!(vb_rivers, vec![
            ((0, 21), 36484),
            ((0, 34), 220904),
            ((0, 39), 131120),
            ((0, 49), 27579),
            ((0, 56), 13938),
            ((0, 96), 37109),
            ((1, 30), 247739),
            ((1, 65), 70850),
            ((1, 67), 253536),
            ((1, 82), 363584),
            ((2, 24), 234571),
            ((2, 88), 121565),
            ((2, 89), 89020),
            ((3, 30), 154325),
            ((3, 67), 50923),
            ((3, 78), 77506),
            ((4, 24), 186155),
            ((4, 27), 408135),
            ((4, 55), 23583),
            ((4, 87), 243576),
            ((5, 32), 343580),
            ((5, 49), 92094),
            ((5, 74), 366297),
            ((6, 14), 352342),
            ((6, 45), 201604),
            ((6, 88), 168611),
            ((7, 10), 11644),
            ((7, 15), 25176),
            ((7, 47), 44170),
            ((7, 73), 20695),
            ((8, 20), 158732),
            ((8, 35), 78532),
            ((9, 18), 152409),
            ((9, 57), 223853),
            ((10, 15), 158058),
            ((10, 26), 117106),
            ((10, 47), 103208),
            ((10, 52), 306768),
            ((10, 73), 7524),
            ((10, 94), 35535),
            ((11, 12), 114871),
            ((11, 29), 114871),
            ((11, 75), 374451),
            ((11, 95), 545349),
            ((12, 29), 1000),
            ((12, 58), 146438),
            ((12, 75), 31233),
            ((13, 31), 75666),
            ((13, 41), 2833),
            ((13, 66), 77666),
            ((13, 80), 8333),
            ((13, 84), 64833),
            ((14, 23), 332733),
            ((14, 60), 163663),
            ((14, 64), 356754),
            ((14, 70), 478481),
            ((15, 26), 46740),
            ((15, 47), 415391),
            ((15, 59), 529230),
            ((15, 94), 24509),
            ((16, 38), 100946),
            ((16, 54), 149813),
            ((17, 50), 2000),
            ((17, 82), 94000),
            ((18, 38), 105446),
            ((19, 58), 65357),
            ((19, 64), 30642),
            ((20, 71), 393307),
            ((20, 77), 474785),
            ((21, 39), 8682),
            ((21, 49), 43567),
            ((21, 56), 168164),
            ((21, 85), 259352),
            ((21, 96), 1625),
            ((22, 51), 123267),
            ((22, 53), 210670),
            ((22, 72), 64984),
            ((22, 79), 81586),
            ((23, 58), 177929),
            ((23, 60), 12246),
            ((23, 64), 79156),
            ((23, 86), 279922),
            ((24, 55), 10700),
            ((24, 87), 96474),
            ((25, 40), 223333),
            ((25, 44), 137666),
            ((26, 46), 72406),
            ((26, 62), 33146),
            ((26, 94), 3873),
            ((27, 40), 407335),
            ((28, 72), 167759),
            ((28, 88), 118896),
            ((28, 89), 83062),
            ((29, 58), 146438),
            ((29, 75), 31233),
            ((30, 85), 434026),
            ((31, 36), 328166),
            ((31, 41), 122333),
            ((31, 66), 2500),
            ((31, 80), 50333),
            ((32, 52), 204834),
            ((32, 68), 161405),
            ((33, 39), 106951),
            ((33, 43), 85101),
            ((33, 76), 85101),
            ((33, 83), 84069),
            ((33, 85), 302056),
            ((33, 92), 84069),
            ((34, 37), 174896),
            ((34, 47), 162959),
            ((34, 56), 125378),
            ((34, 73), 122607),
            ((35, 51), 87432),
            ((36, 40), 575333),
            ((36, 66), 164166),
            ((37, 47), 479266),
            ((37, 86), 647162),
            ((39, 49), 53561),
            ((39, 85), 116609),
            ((39, 96), 9613),
            ((40, 61), 282805),
            ((40, 95), 482773),
            ((41, 84), 31166),
            ((42, 45), 72491),
            ((42, 53), 90480),
            ((42, 72), 16817),
            ((43, 48), 11070),
            ((43, 74), 67383),
            ((43, 76), 1000),
            ((43, 83), 7000),
            ((43, 92), 7000),
            ((44, 80), 54666),
            ((45, 69), 111613),
            ((46, 62), 9494),
            ((46, 63), 12449),
            ((46, 91), 16751),
            ((47, 73), 55975),
            ((48, 74), 63717),
            ((48, 76), 11070),
            ((48, 83), 17071),
            ((48, 92), 17071),
            ((49, 56), 46262),
            ((50, 82), 188000),
            ((50, 93), 96000),
            ((52, 56), 325566),
            ((52, 68), 238386),
            ((52, 73), 103641),
            ((53, 70), 438481),
            ((53, 71), 417654),
            ((53, 72), 144711),
            ((54, 77), 236146),
            ((55, 87), 61716),
            ((56, 96), 169289),
            ((57, 68), 336958),
            ((57, 81), 130477),
            ((58, 60), 192591),
            ((58, 64), 351216),
            ((58, 75), 669602),
            ((58, 86), 396573),
            ((59, 62), 54557),
            ((59, 63), 70334),
            ((59, 77), 511867),
            ((59, 90), 80895),
            ((59, 91), 139196),
            ((60, 64), 9642),
            ((61, 67), 268406),
            ((61, 78), 40399),
            ((62, 63), 19979),
            ((62, 91), 20587),
            ((62, 94), 84211),
            ((63, 90), 2559),
            ((63, 91), 7695),
            ((64, 87), 443934),
            ((65, 67), 25150),
            ((69, 79), 46780),
            ((73, 94), 96727),
            ((74, 76), 67383),
            ((74, 82), 323584),
            ((75, 85), 886139),
            ((76, 83), 7000),
            ((76, 92), 7000),
            ((78, 95), 65425),
            ((81, 91), 127210),
            ((83, 92), 1000),
            ((85, 96), 224326),
            ((88, 89), 36375),
            ((90, 91), 12545)]);
    }
}
