use super::types::PunterId;
use super::proto::{Move, Setup};

pub trait GameStateBuilder {
    type GameState: GameState;

    fn build(self, setup: Setup) -> Self::GameState;
}

pub trait GameState: Sized {
    type Error;

    fn play(self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error>;
    fn stop(self, moves: Vec<Move>) -> Result<Self, Self::Error>;
    fn get_punter(&self) -> PunterId;
}
