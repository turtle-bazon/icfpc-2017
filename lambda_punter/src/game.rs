use std::collections::HashMap;

use super::types::{PunterId, SiteId};
use super::map::Map;
use super::proto::{Move, Setup};

#[allow(dead_code)]
pub struct GameState {
    pub punter: PunterId,
    punters_count: usize,
    map: Map,
    moves: HashMap<PunterId, Vec<Move>>,
}

impl GameState {
    pub fn new(setup: Setup) -> GameState {
        GameState {
            punter: setup.punter,
            punters_count: setup.punters,
            map: setup.map,
            moves: HashMap::new(),
        }
    }

    pub fn score_for(mut self, punter: PunterId) -> u64 {
        let is_path_reachable = |site1: SiteId, site2: SiteId| true;
        let shortest_path = |site1: SiteId, site2: SiteId| 1;
        let score_from_mine_to_site = |mine: SiteId, site: SiteId| {
            if is_path_reachable(mine, site) {
                let path_rang = shortest_path(mine, site);

                path_rang * path_rang
            } else {
                0
            }
        };
        let score_from_mine = |mine| 1;
        self.map.mines
            .iter()
            .map(score_from_mine)
            .sum()
    }

    pub fn play(mut self, moves: Vec<Move>) -> (Move, GameState) {
        for mv in moves {
            let punter = match mv {
                Move::Claim { punter, .. } => punter,
                Move::Pass { punter, } => punter,
            };
            self.moves
                .entry(punter)
                .or_insert_with(Vec::new)
                .push(mv);
        }

        (Move::Pass { punter: self.punter, }, self)
    }
}
