
use std::cmp::{min,max};
use std::collections::{HashMap,HashSet};

use super::super::types::{PunterId, SiteId};
use super::super::map::Map;
use super::super::proto::{Move, Setup};
use super::super::game::{GameState,GameStateBuilder};

pub struct NearestGameStateBuilder;

impl GameStateBuilder for NearestGameStateBuilder {
    type GameState = NearestGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        let mut all_rivs = HashMap::new();
        for r in &setup.map.rivers {
            let src = min(r.source,r.target);
            let dst = max(r.source,r.target);
                
            if setup.map.mines.contains(&src)||setup.map.mines.contains(&dst) {
                all_rivs.entry(0)
                    .or_insert_with(HashSet::new)
                    .insert((src,dst));
            } else {
                all_rivs.entry(1)
                    .or_insert_with(HashSet::new)
                    .insert((src,dst));
            }        
        }
        
        NearestGameState {
            punter: setup.punter,
            punters_count: setup.punters,
            map: setup.map,
            moves: HashMap::new(),

            all_rivers: all_rivs,
        }
    }
}


#[allow(dead_code)]
pub struct NearestGameState {
    punter: PunterId,
    punters_count: usize,
    map: Map,
    moves: HashMap<PunterId, Vec<Move>>,

    all_rivers: HashMap<usize,HashSet<(SiteId,SiteId)>>,
}

impl GameState for NearestGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);
        Ok((match self.get_next_move() {
            Ok((src,dst)) => {
                Move::Claim { punter: self.punter, source: src, target: dst} 
            },
            Err(..) => {
                Move::Pass { punter: self.punter, }
            }
        }, self))
    }

    fn stop(mut self, moves: Vec<Move>) -> Result<Self, Self::Error> {
        self.update_moves(moves);
        Ok(self)
    }

    fn get_punter(&self) -> PunterId {
        self.punter
    }
}

impl NearestGameState {
    fn get_next_move(&mut self) -> Result<(SiteId,SiteId),()> {
        match self.all_rivers.entry(0)
            .or_insert_with(HashSet::new)
            .iter().next() {
                Some(r) => {
                    return Ok(r.clone())
                },
                None => {}
            }
        match self.all_rivers.entry(1)
            .or_insert_with(HashSet::new)
            .iter().next() {
                Some(r) => Ok(r.clone()),
                None => Err(())
            }
    }

    fn update_moves(&mut self, moves: Vec<Move>) {
        for mv in moves {
            let punter = match mv {
                Move::Claim { punter, source, target } => {
                    let src = min(source,target);
                    let dst = max(source,target);
                    self.all_rivers.entry(0)
                        .or_insert_with(HashSet::new)
                        .remove(&(src,dst));
                    self.all_rivers.entry(1)
                        .or_insert_with(HashSet::new)
                        .remove(&(src,dst));
                    punter
                },
                Move::Pass { punter, } => punter,
            };
            self.moves
                .entry(punter)
                .or_insert_with(Vec::new)
                .push(mv);
        }
    }

}

