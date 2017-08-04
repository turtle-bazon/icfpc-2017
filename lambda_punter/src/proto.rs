
use super::types::{PunterId, SiteId};
use super::map::Map;

#[derive(Debug)]
pub enum OnlineReq {
    Handshake { name: String, },
    Ready { punter: PunterId, },
    Move(Move),
}

#[derive(Debug)]
pub enum OnlineRep {
    Handshake { name: String, },
    Setup {
        punter: PunterId,
        punters: usize,
        map: Map,
    },
    Move { moves: Vec<Move>, },
    Stop {
        moves: Vec<Move>,
        scores: Vec<Score>,
    },
}

#[derive(Debug)]
pub enum Move {
    Claim { punter: PunterId, source: SiteId, target: SiteId, },
    Pass { punter: PunterId, },
}

#[derive(Debug)]
pub struct Score {
    pub punter: PunterId,
    pub score: usize,
}
