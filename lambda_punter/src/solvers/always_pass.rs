use std::collections::HashMap;
use super::super::types::{PunterId, SiteId};
use super::super::map::Map;
use super::super::proto::{Move, Setup};
use super::super::game::{GameState, GameStateBuilder};

pub struct AlwaysPassGameStateBuilder;

impl GameStateBuilder for AlwaysPassGameStateBuilder {
    type GameState = AlwaysPassGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        AlwaysPassGameState {
            punter: setup.punter,
            punters_count: setup.punters,
            map: setup.map,
            moves: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
pub struct AlwaysPassGameState {
    punter: PunterId,
    punters_count: usize,
    map: Map,
    moves: HashMap<PunterId, Vec<Move>>,
}

impl GameState for AlwaysPassGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);
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

impl AlwaysPassGameState {
    fn update_moves(&mut self, moves: Vec<Move>) {
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
    }

    pub fn score_for(&self, _punter: PunterId) -> u64 {
        let is_path_reachable = |_site1: SiteId, _site2: SiteId| true;
        let shortest_path = |_site1: SiteId, _site2: SiteId| 1;
        let _score_from_mine_to_site = |mine: SiteId, site: SiteId| {
            if is_path_reachable(mine, site) {
                let path_rang = shortest_path(mine, site);

                path_rang * path_rang
            } else {
                0
            }
        };
        let score_from_mine = |_mine| 1;
        self.map.mines
            .iter()
            .map(score_from_mine)
            .sum()
    }
}
