
use super::types::{PunterId, SiteId};
use super::map::Map;

#[derive(PartialEq, Debug)]
pub enum Req {
    Handshake { name: String, },
    Ready { punter: PunterId, },
    Move(Move),
}

#[derive(PartialEq, Debug)]
pub enum Rep {
    Handshake { name: String, },
    Setup(Setup),
    Move { moves: Vec<Move>, },
    Stop {
        moves: Vec<Move>,
        scores: Vec<Score>,
    },
}

#[derive(PartialEq, Debug)]
pub struct Setup {
    pub punter: PunterId,
    pub punters: usize,
    pub map: Map,
}

#[derive(PartialEq, Debug)]
pub enum Move {
    Claim { punter: PunterId, source: SiteId, target: SiteId, },
    Pass { punter: PunterId, },
}

#[derive(PartialEq, Debug)]
pub struct Score {
    pub punter: PunterId,
    pub score: usize,
}
