use super::super::types::PunterId;
use super::super::proto::{Move, Setup};
use super::super::game::{GameState, GameStateBuilder};

pub struct LinkMinesGameStateBuilder;

impl GameStateBuilder for LinkMinesGameStateBuilder {
    type GameState = LinkMinesGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        LinkMinesGameState {
            punter: setup.punter,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LinkMinesGameState {
    punter: PunterId,
}

impl GameState for LinkMinesGameState {
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
