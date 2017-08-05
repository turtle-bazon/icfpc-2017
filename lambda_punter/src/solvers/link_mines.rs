use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};

use super::super::types::{PunterId, SiteId};
use super::super::map::River;
use super::super::proto::{Move, Setup, Future};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::Graph;

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
                    if let Some(path) = rivers_graph.shortest_path(key.0, key.1, &mut gcache, |_| true) {
                        mine_pairs.insert(key, path.len());
                        break;
                    }
                }
            }
        } else {
            for &mine_a in setup.map.mines.iter() {
                for &mine_b in setup.map.mines.iter() {
                    let key = (min(mine_a, mine_b), max(mine_a, mine_b));
                    if (mine_a != mine_b) && !mine_pairs.contains_key(&key) {
                        if let Some(path) = rivers_graph.shortest_path(key.0, key.1, &mut gcache, |_| true) {
                            mine_pairs.insert(key, path.len());
                        }
                    }
                }
            }
        }
        let mut pairs: Vec<_> = mine_pairs.into_iter().collect();
        pairs.sort_by_key(|p| p.1);
        let futures =
            if let Some(&((source, target), _)) = pairs.last() {
                if setup.settings.futures {
                    debug!(" ;; declaring future: from {} to {}", source, target);
                    Some(vec![Future { source: source, target: target, }])
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
            goals_rev: Vec::new(),
            claimed_rivers: HashSet::new(),
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
    goals_rev: Vec<(SiteId, SiteId)>,
    claimed_rivers: HashSet<River>,
    futures: Option<Vec<Future>>,
    mines_connected_sites: HashSet<SiteId>,
}

impl GameState for LinkMinesGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);

        let mut gcache = Default::default();
        let mut pass = 0;
        while pass < 2 {
            while let Some((orig_source, source, orig_target, target)) = self.goals.pop() {
                debug!(" ;; found current goal: from {} (originally {}) to {} (originally {})",
                       source, orig_source, target, orig_target);
                let maybe_path = {
                    let claimed_rivers = &self.claimed_rivers;
                    let check_claimed = |(s, t)| !claimed_rivers
                        .contains(&River {
                            source: min(s, t),
                            target: max(s, t),
                        });
                    self.rivers_graph.shortest_path(source, target, &mut gcache, check_claimed)
                };
                if let Some(path) = maybe_path {
                    debug!(" ;; there is a path for goal from {} to {}: {:?}", source, target, path);
                    if let (Some(&ps), Some(&pt)) = (path.get(0), path.get(1)) {
                        let move_ = Move::Claim { punter: self.punter, source: ps, target: pt, };
                        self.goals.push((orig_target, target, orig_source, pt));
                        self.mines_connected_sites.insert(ps);
                        self.mines_connected_sites.insert(pt);
                        return Ok((move_, self));
                    }
                }
                // path is done, revert it for the future
                debug!(" ;; reverting path from {} to {}", orig_source, orig_target);
                self.goals_rev.push((orig_source, orig_target));
            }
            // all mines are connected somehow, try again
            let iter = self.goals_rev.drain(..).map(|(s, t)| (s, s, t, t));
            self.goals.extend(iter);
            pass += 1;
        }

        // choose other free river
        let mut maybe_move = None;
        for river in self.rivers.iter() {
            if !self.claimed_rivers.contains(river) {
                let claimed_rivers = &self.claimed_rivers;
                let check_claimed = |(s, t)| !claimed_rivers
                    .contains(&River {
                        source: min(s, t),
                        target: max(s, t),
                    });
                for &mine_site in self.mines_connected_sites.iter() {
                    if let Some(path) = self.rivers_graph.shortest_path(mine_site, river.source, &mut gcache, &check_claimed) {
                        debug!(" ;; fallback: there is a path from mines connected sites {} to free river end {}", mine_site, river.source);
                        if let (Some(&s), Some(&t)) = (path.get(0), path.get(1)) {
                            maybe_move = Some(Move::Claim { punter: self.punter, source: s, target: t, });
                            break;
                        }
                    }
                    if let Some(path) = self.rivers_graph.shortest_path(mine_site, river.target, &mut gcache, &check_claimed) {
                        debug!(" ;; fallback: there is a path from mines connected sites {} to free river end {}", mine_site, river.target);
                        if let (Some(&s), Some(&t)) = (path.get(0), path.get(1)) {
                            maybe_move = Some(Move::Claim { punter: self.punter, source: s, target: t, });
                            break;
                        }
                    }
                }
            }
            if maybe_move.is_some() {
                break;
            }
        }
        if let Some(move_) = maybe_move {
            Ok((move_, self))
        } else {
            Ok((Move::Pass { punter: self.punter, }, self))
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
            if let Move::Claim { source, target, .. } = move_ {
                self.claimed_rivers.insert(River {
                    source: min(source, target),
                    target: max(source, target),
                });
            }
        }
    }
}
