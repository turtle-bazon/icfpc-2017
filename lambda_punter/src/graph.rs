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

    pub fn longest_jouney_from<'a>(&self, source: SiteId, cache: &'a mut GraphCache) -> Option<&'a [SiteId]> {
        cache.clear();
        cache.path_buf.push((source, 0));
        cache.pqueue.push(PQNode { site: source, cost: 0, phead: 1, });
        let mut best = None;
        while let Some(PQNode { site, cost: current_cost, phead, }) = cache.pqueue.pop() {
            best = match best {
                Some((best_phead, best_cost)) if current_cost < best_cost =>
                    Some((best_phead, best_cost)),
                _ =>
                    Some((phead, current_cost)),
            };
            cache.visited.insert(site);
            if let Some(neighs) = self.neighs.get(&site) {
                for &reachable_site in neighs.iter() {
                    if cache.visited.contains(&reachable_site) {
                        continue;
                    }
                    cache.path_buf.push((reachable_site, phead));
                    cache.pqueue.push(PQNode {
                        site: reachable_site,
                        cost: current_cost + 1,
                        phead: cache.path_buf.len(),
                    });
                }
            }
        }

        if let Some((mut phead, _)) = best {
            while phead != 0 {
                let (site_hop, next_phead) = cache.path_buf[phead - 1];
                cache.path.push(site_hop);
                phead = next_phead;
            }
            cache.path.reverse();
            Some(&cache.path)
        } else {
            None
        }
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
    fn longest_jouney() {
        let mut cache = Default::default();
        let graph = sample_map();
        assert_eq!(graph.longest_jouney_from(6, &mut cache).and_then(|p| p.last()), Some(&2));
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

    #[test]
    fn betweenness_random_medium_map() {
        let mut cache = Default::default();
        let graph = Graph::from_iter(
            [(81,91),(57,68),(32,68),(52,68),(32,52),(5,49),(5,74),(8,35),(53,71),(59,77),(22,53),(2,24),(27,40),(25,40),(36,40),(40,61),(25,44),
             (3,78),(30,85),(3,67),(61,67),(65,67),(1,30),(1,65),(22,51),(42,72),(42,53),(53,72),(22,72),(6,45),(64,87),(17,50),(50,82),(1,82),
             (1,67),(54,77),(16,38),(9,18),(7,10),(11,12),(6,14),(7,15),(10,15),(8,20),(0,21),(14,23),(4,24),(10,26),(15,26),(4,27),(11,29),(12,29),
             (3,30),(13,31),(5,32),(0,34),(31,36),(34,37),(18,38),(0,39),(21,39),(33,39),(13,41),(31,41),(33,43),(42,45),(26,46),(7,47),(10,47),
             (15,47),(34,47),(37,47),(43,48),(0,49),(21,49),(39,49),(35,51),(10,52),(16,54),(4,55),(24,55),(0,56),(21,56),(34,56),(49,56),(52,56),
             (9,57),(12,58),(19,58),(23,58),(29,58),(15,59),(14,60),(23,60),(58,60),(26,62),(46,62),(59,62),(46,63),(59,63),(62,63),(14,64),
             (19,64),(23,64),(58,64),(60,64),(13,66),(31,66),(36,66),(45,69),(14,70),(53,70),(20,71),(28,72),(7,73),(10,73),(34,73),(47,73),
             (52,73),(43,74),(48,74),(11,75),(12,75),(29,75),(58,75),(33,76),(43,76),(48,76),(74,76),(20,77),(61,78),(22,79),(69,79),(13,80),
             (31,80),(44,80),(57,81),(17,82),(74,82),(33,83),(43,83),(48,83),(76,83),(13,84),(41,84),(21,85),(33,85),(39,85),(75,85),(23,86),
             (37,86),(58,86),(4,87),(24,87),(55,87),(2,88),(6,88),(28,88),(2,89),(28,89),(88,89),(59,90),(63,90),(46,91),(59,91),(62,91),(63,91),
             (90,91),(33,92),(43,92),(48,92),(76,92),(83,92),(50,93),(10,94),(15,94),(26,94),(62,94),(73,94),(11,95),(40,95),(78,95),(0,96),
             (21,96),(39,96),(56,96),(85,96)].iter().cloned());
        let b_rivers = graph.rivers_betweenness(&mut cache);
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
