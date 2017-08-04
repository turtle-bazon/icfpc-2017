
use super::types::PunterId;
use super::map::Map;
use super::proto::{Move, Setup};

#[allow(dead_code)]
pub struct GameState {
    punter: PunterId,
    punters_count: usize,
    map: Map,
}

impl GameState {
    pub fn new(setup: Setup) -> GameState {
        GameState {
            punter: setup.punter,
            punters_count: setup.punters,
            map: setup.map,
        }
    }

    pub fn play(self, _moves: &[Move]) -> (Move, GameState) {
        (Move::Pass { punter: self.punter, }, self)
    }
}
