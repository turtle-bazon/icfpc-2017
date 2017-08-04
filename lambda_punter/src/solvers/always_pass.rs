use super::super::types::PunterId;
use super::super::proto::{Move, Setup};
use super::super::game::{GameState, GameStateBuilder};

pub struct AlwaysPassGameStateBuilder;

impl GameStateBuilder for AlwaysPassGameStateBuilder {
    type GameState = AlwaysPassGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        AlwaysPassGameState {
            punter: setup.punter,
        }
    }
}

#[allow(dead_code)]
pub struct AlwaysPassGameState {
    punter: PunterId,
}

impl GameState for AlwaysPassGameState {
    type Error = ();

    fn play(self, _moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        Ok((Move::Pass { punter: self.punter, }, self))
    }

    fn stop(self, _moves: Vec<Move>) -> Result<Self, Self::Error> {
        Ok(self)
    }

    fn get_punter(&self) -> PunterId {
        self.punter
    }
}
