use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};

use super::super::types::{PunterId, SiteId};
use super::super::map::River;
use super::super::proto::{Move, Setup, Future};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::{Graph, GraphCache, EdgeAttr};

pub struct LinkMinesGameStateBuilder;

impl GameStateBuilder for LinkMinesGameStateBuilder {
    type GameState = LinkMinesGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        let rivers_graph = Graph::from_map(&setup.map);
        let mut gcache = Default::default();
        let mut mine_pairs = HashMap::new();
        if setup.map.mines.len() < 2 {
            if let Some(&mine) = setup.map.mines.iter().next() {
                for (&site, _) in setup.map.sites.iter() {
                    let key = (min(mine, site), max(mine, site));
                    if let Some(path) = rivers_graph.shortest_path_only(key.0, key.1, &mut gcache) {
                        mine_pairs.insert(key, path.to_owned());
                        break;
                    }
                }
            }
        } else {
            for &mine_a in setup.map.mines.iter() {
                for &mine_b in setup.map.mines.iter() {
                    let key = (min(mine_a, mine_b), max(mine_a, mine_b));
                    if (mine_a != mine_b) && !mine_pairs.contains_key(&key) {
                        if let Some(path) = rivers_graph.shortest_path_only(key.0, key.1, &mut gcache) {
                            mine_pairs.insert(key, path.to_owned());
                        }
                    }
                }
            }
        }
        let mut pairs: Vec<_> = mine_pairs.into_iter().collect();
        pairs.sort_by_key(|p| (p.1).len());
        let futures =
            if let Some(&(_, ref path)) = pairs.last() {
                let len = path.len();
                if setup.settings.futures && len >= 3 {
                    if let (Some(&s), Some(&sp), Some(&tp), Some(&t)) =
                        (path.get(0), path.get(1), path.get(len - 2), path.get(len - 1))
                    {
                        debug!(" ;; declaring direct future: from {} to {}, and reverse: from {} to {}",
                               s, tp, t, sp);
                        Some(vec![Future { source: s, target: tp, },
                                  Future { source: t, target: sp, }])
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
        let goals = pairs.into_iter().map(|p| ((p.0).0, (p.0).0, (p.0).1, (p.0).1)).collect();

        LinkMinesGameState {
            punter: setup.punter,
            rivers: setup.map.rivers,
            rivers_graph: rivers_graph,
            goals: goals,
            claimed_rivers: HashMap::new(),
            futures: futures,
            mines_connected_sites: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LinkMinesGameState {
    punter: PunterId,
    rivers: HashSet<River>,
    rivers_graph: Graph,
    goals: Vec<(SiteId, SiteId, SiteId, SiteId)>,
    claimed_rivers: HashMap<River, PunterId>,
    futures: Option<Vec<Future>>,
    mines_connected_sites: HashSet<SiteId>,
}

impl GameState for LinkMinesGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);
        let mut gcache = Default::default();
        loop {
            while let Some((orig_source, source, orig_target, target)) = self.goals.pop() {
                debug!(" ;; found current goal: from {} (originally {}) to {} (originally {})",
                       source, orig_source, target, orig_target);
                let maybe_path = self.shortest_path(source, target, &mut gcache);
                if let Some(path) = maybe_path {
                    debug!(" ;; there is a path for goal from {} to {}: {:?}", source, target, path);
                    let mut offset = 0;
                    while let (Some(&ps), Some(&pt)) = (path.get(offset), path.get(offset + 1)) {
                        let wanted_river = River::new(ps, pt);
                        if self.claimed_rivers.get(&wanted_river).map(|&p| p == self.punter).unwrap_or(false) {
                            // it is already mine river, skip it
                            debug!(" ;; skipping already mine river from {} to {}", ps, pt);
                            offset += 1;
                            continue;
                        } else {
                            // not yet mine river
                            let move_ = Move::Claim { punter: self.punter, source: ps, target: pt, };
                            self.goals.push((orig_target, target, orig_source, pt));
                            self.mines_connected_sites.insert(ps);
                            self.mines_connected_sites.insert(pt);
                            return Ok((move_, self));
                        }
                    }
                }
                debug!(" ;; no path from {} to {} for a goal", orig_source, orig_target);
            }

            // all current goals are reached for now, let's choose a free river connected to our already existing path
            let mut maybe_goal = None;
            for river in self.rivers.iter() {
                if !self.claimed_rivers.contains_key(river) {
                    for &mine_site in self.mines_connected_sites.iter() {
                        if let Some(path) = self.shortest_path(river.source, mine_site, &mut gcache) {
                            let journey_start = mine_site;
                            let journey_end = match path.get(1) {
                                Some(&p) if p == river.target =>
                                    river.source,
                                _ =>
                                    river.target,
                            };
                            debug!(" ;; fallback: new goal is chosen: from {} (as a part of mine path) to {}", journey_start, journey_end);
                            maybe_goal = Some((journey_start, journey_end));
                            break;
                        }
                    }
                }
                if maybe_goal.is_some() {
                    break;
                }
            }

            if let Some((source, target)) = maybe_goal {
                // new goal has been found
                self.goals.push((source, source, target, target));
                continue;
            } else {
                // no new goals, just pass for now
                return Ok((Move::Pass { punter: self.punter, }, self));
            }
        }
    }

    fn stop(mut self, moves: Vec<Move>) -> Result<Self, Self::Error> {
        self.update_moves(moves);
        Ok(self)
    }

    fn get_punter(&self) -> PunterId {
        self.punter
    }

    fn get_futures(&mut self) -> Option<Vec<Future>> {
        self.futures.take()
    }
}

impl LinkMinesGameState {
    fn update_moves(&mut self, moves: Vec<Move>) {
        for move_ in moves {
            if let Move::Claim { punter, source, target, } = move_ {
                self.claimed_rivers.insert(River::new(source, target), punter);
            }
        }
    }

    fn shortest_path<'a>(&self, source: SiteId, target: SiteId, gcache: &'a mut GraphCache) -> Option<&'a [SiteId]> {
        let my_punter = self.punter;
        let claimed_rivers = &self.claimed_rivers;
        let probe_claimed = |(s, t)| claimed_rivers
            .get(&River::new(s, t))
            .map(|&river_owner| if river_owner == my_punter {
                EdgeAttr::Accessible { edge_cost: 0, }
            } else {
                EdgeAttr::Blocked
            })
            .unwrap_or(EdgeAttr::Accessible { edge_cost: 1, });

        self.rivers_graph.shortest_path(source, target, gcache, probe_claimed)
    }
}
