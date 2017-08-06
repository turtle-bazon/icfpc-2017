use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};
use rand::{self, Rng};

use super::super::types::{PunterId, SiteId};
use super::super::map::{River, RiversIndex};
use super::super::proto::{Move, Setup, Future};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::{Graph, GraphCache, EdgeAttr};

pub struct GNGameStateBuilder;

impl GameStateBuilder for GNGameStateBuilder {
    type GameState = GNGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        // make map graph
        let rivers_graph = Graph::from_map(&setup.map);
        let mut gcache = Default::default();

        // calculate betweenness coeffs
        let rivers_bw = RiversIndex::from_hash_map(
            rivers_graph.rivers_betweenness(&mut gcache));

        let mut mine_pairs = HashMap::new();
        if setup.map.mines.len() < 2 {
            debug!("there is only one mine on this map");
            if let Some(&mine) = setup.map.mines.iter().next() {
                if let Some(path) = rivers_graph.longest_jouney_from(mine, &mut gcache) {
                    if let Some(&longest_jouney_site) = path.last() {
                        debug!("longest jouney choosen from mine {} to {}", mine, longest_jouney_site);
                        let key = (min(mine, longest_jouney_site), max(mine, longest_jouney_site));
                        mine_pairs.insert(key, path.to_owned());
                    }
                }
            }
        } else {
            debug!("there are {} mines on this map", setup.map.mines.len());
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
        debug!("initially choosen {} goals", pairs.len());

        let futures =
            if let Some(&(_, ref path)) = pairs.last() {
                let len = path.len();
                if setup.settings.futures && len >= 3 {
                    if let (Some(&s), Some(&sp), Some(&tp), Some(&t)) =
                        (path.get(0), path.get(1), path.get(len - 2), path.get(len - 1))
                    {
                        debug!("declaring direct future: from {} to {} and reverse one: from {} to {}", s, tp, t, sp);
                        Some(vec![Future { source: s, target: tp, }, Future { source: t, target: sp, }])
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

        GNGameState {
            punter: setup.punter,
            rivers: setup.map.rivers,
            rivers_graph: rivers_graph,
            goals: pairs.into_iter().map(|p| ((p.0).0, (p.0).1)).collect(),
            claimed_rivers: ClaimedRivers::new(),
            futures: futures,
            mines_connected_sites: HashSet::new(),
            rivers_bw: rivers_bw,
        }
    }
}

type ClaimedRivers = RiversIndex<PunterId>;

#[derive(Serialize, Deserialize)]
pub struct GNGameState {
    punter: PunterId,
    rivers: Vec<River>,
    rivers_graph: Graph,
    goals: Vec<(SiteId, SiteId)>,
    claimed_rivers: ClaimedRivers,
    futures: Option<Vec<Future>>,
    mines_connected_sites: HashSet<SiteId>,
    rivers_bw: RiversIndex<f64>,
}

impl GameState for GNGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);
        let mut gcache = Default::default();
        loop {
            while let Some((source, target)) = self.goals.pop() {
                debug!("found current goal: from {} to {}", source, target);
                let maybe_path = self.shortest_path(source, target, &mut gcache);
                if let Some(path) = maybe_path {
                    debug!("there is a path for goal from {} to {}: {:?}", source, target, path);
                    let mut offset = 0;
                    while let (Some(&ps), Some(&pt)) = (path.get(offset), path.get(offset + 1)) {
                        let wanted_river = River::new(ps, pt);
                        if self.claimed_rivers.get(&wanted_river).map(|&p| p == self.punter).unwrap_or(false) {
                            // it is already mine river, skip it
                            debug!("skipping already claimed river from {} to {}", ps, pt);
                            offset += 1;
                            continue;
                        } else {
                            // not yet mine river
                            let move_ = Move::Claim { punter: self.punter, source: ps, target: pt, };
                            self.goals.push((target, source));
                            self.mines_connected_sites.insert(ps);
                            self.mines_connected_sites.insert(pt);
                            return Ok((move_, self));
                        }
                    }
                }
                debug!("no path from {} to {} for a goal: probably blocked by our foes", source, target);
            }

            // all current goals are reached for now, let's choose a random free river connected to our already existing path
            let mut new_goal = None;
            rand::thread_rng().shuffle(&mut self.rivers);
            for river in self.rivers.iter() {
                if !self.claimed_rivers.contains_key(river) {
                    for &mine_site in self.mines_connected_sites.iter() {
                        if self.shortest_path(river.source, mine_site, &mut gcache).is_some() {
                            debug!("fallback: new goal is chosen: from {} (as a part of mine path) to {}", mine_site, river.target);
                            let move_ = Move::Claim { punter: self.punter, source: river.source, target: river.target, };
                            new_goal = Some((move_, mine_site, river.source, river.target));
                            break;
                        }
                    }
                }
                if new_goal.is_some() {
                    break;
                }
            }

            return Ok(if let Some((move_, ms, rs, rt)) = new_goal {
                // new goal is choosen
                self.goals.push((ms, rs));
                self.mines_connected_sites.insert(rs);
                self.mines_connected_sites.insert(rt);
                (move_, self)
            } else {
                // no new goals, just pass for now
                (Move::Pass { punter: self.punter, }, self)
            });
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

impl GNGameState {
    fn update_moves(&mut self, moves: Vec<Move>) {
        for move_ in moves {
            match move_ {
                Move::Claim { punter, source, target, } => {
                    self.claimed_rivers.insert(River::new(source, target), punter);
                },
                Move::Pass { .. } =>
                    (),
                Move::Splurge { punter, route, } => {
                    let mut offset = 0;
                    while let (Some(&source), Some(&target)) = (route.get(offset), route.get(offset + 1)) {
                        self.claimed_rivers.insert(River::new(source, target), punter);
                        offset += 1;
                    }
                },
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
