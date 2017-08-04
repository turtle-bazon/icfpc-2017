use std::collections::HashMap;

use super::types::PunterId;
use super::map::Map;
use super::proto::{Move, Setup};

#[allow(dead_code)]
pub struct GameState {
    punter: PunterId,
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
