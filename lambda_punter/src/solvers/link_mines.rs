use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};

use super::super::types::{PunterId, SiteId};
use super::super::map::River;
use super::super::proto::{Move, Setup};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::Graph;

pub struct LinkMinesGameStateBuilder;

impl GameStateBuilder for LinkMinesGameStateBuilder {
    type GameState = LinkMinesGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        let rivers_graph = Graph::from_map(&setup.map);
        let mut gcache = Default::default();

        let mut mine_pairs = HashMap::new();
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
        let mut pairs: Vec<_> = mine_pairs.into_iter().collect();
        pairs.sort_by_key(|p| p.1);
        let goals = pairs.into_iter().map(|p| p.0).collect();
        LinkMinesGameState {
            punter: setup.punter,
            rivers_graph: rivers_graph,
            goals: goals,
            claimed_rivers: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LinkMinesGameState {
    punter: PunterId,
    rivers_graph: Graph,
    goals: Vec<(SiteId, SiteId)>,
    claimed_rivers: HashSet<River>,
}

impl GameState for LinkMinesGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);

        let mut gcache = Default::default();
        while let Some((source, target)) = self.goals.pop() {
            debug!(" ;; found current goal: from {} to {}", source, target);
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
                    self.goals.push((pt, target));
                    return Ok((move_, self));
                }
            }
        }

        // TODO: choose other river
        Ok((Move::Pass { punter: self.punter, }, self))
    }

    fn stop(mut self, moves: Vec<Move>) -> Result<Self, Self::Error> {
        self.update_moves(moves);
        Ok(self)
    }

    fn get_punter(&self) -> PunterId {
        self.punter
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
