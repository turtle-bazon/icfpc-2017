use std::collections::{HashMap, HashSet};

use super::types::{PunterId, SiteId};
use super::map::{Map, Site, River};
use super::proto::{Move, Setup};

#[allow(dead_code)]
pub struct GameState {
    pub punter: PunterId,
    punters_count: usize,
    map: Map,
    claims: HashMap<River, PunterId>,
    moves: HashMap<PunterId, Vec<Move>>,
}

impl GameState {
    pub fn new(setup: Setup) -> GameState {
        GameState {
            punter: setup.punter,
            punters_count: setup.punters,
            map: setup.map,
            claims: HashMap::new(),
            moves: HashMap::new(),
        }
    }

    pub fn score_for(&self, punter: PunterId) -> u64 {
        let sites: HashMap<SiteId, Option<usize>> = self.map.sites
            .iter()
            .map(|(site, _)| (*site, None))
            .collect();
        let is_path_reachable = |site1: SiteId, site2: SiteId| {
            true
        };
        let site_rivers = |site: SiteId| -> Vec<&River> {
            self.map.rivers
                .iter()
                .filter(|river| river.source == site || river.target == site)
                .collect()
        };
        let shortest_path = |site1: SiteId, site2: SiteId| {

            1
        };
        let score_from_mine_to_site = |mine: SiteId, site: SiteId| {
            if is_path_reachable(mine, site) {
                let path_rang = shortest_path(mine, site);

                path_rang * path_rang
            } else {
                0
            }
        };
        let score_from_mine = |&mine| -> u64 {
            let mut sites_to_proceed: Vec<(SiteId, usize)> = Vec::new();
            let mut site_nodes: HashMap<SiteId, usize> = HashMap::new();
            let mut visited_nodes: HashSet<SiteId> = HashSet::new();

            let mut search_paths = |site: SiteId, initial: usize| -> Vec<SiteId> {
                if ! visited_nodes.contains(&site) {
                    println!("Node: {}, site_nodes: {:?}", site, &site_nodes);
                    let path = site_nodes.entry(site).or_insert(initial);
                    if *path > initial {
                        *path = initial;
                    }
                    
                    println!("Node: {}, site_nodes: {:?}", site, site_nodes);
                    println!("");
                    visited_nodes.insert(site);
                    let site_rivers = site_rivers(site);

                    site_rivers
                        .iter()
                        .flat_map(|river| Some(river.source).into_iter().chain(Some(river.target).into_iter()))
                        .collect()
                } else {
                    vec![]
                }
            };

            sites_to_proceed.push((mine, 0));

            while sites_to_proceed.len() > 0 {
                let (current_site, current_value) = sites_to_proceed.remove(0);
                let mut cs1 = current_site;
                let mut cv1 = current_value;

                for add_site in search_paths(cs1, cv1) {
                    sites_to_proceed.push((add_site, current_value + 1));
                }
            }

            self.map.sites.iter()
                .map(|(&site, _)| score_from_mine_to_site(mine, site))
                .sum()
        };
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

            match mv {
                Move::Claim { punter, source, target } => {
                    let river = River {source, target};
                    
                    self.claims.insert(river, punter);
                },
                Move::Pass {..} => {},
            }
            self.moves
                .entry(punter)
                .or_insert_with(Vec::new)
                .push(mv);
        }

        (Move::Pass { punter: self.punter, }, self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn simple_scoring() {
        let mut sites: HashMap<SiteId, Site> = HashMap::new();

        sites.insert(0, Site{id: 0});
        sites.insert(1, Site{id: 1});
        sites.insert(2, Site{id: 2});
        sites.insert(3, Site{id: 3});
        sites.insert(4, Site{id: 4});
        sites.insert(5, Site{id: 5});
        sites.insert(6, Site{id: 6});
        sites.insert(7, Site{id: 7});
        
        let mut rivers: HashSet<River> = HashSet::new();

        rivers.insert(River{source: 0, target: 1});
        rivers.insert(River{source: 1, target: 2});
        rivers.insert(River{source: 0, target: 7});
        rivers.insert(River{source: 7, target: 6});
        rivers.insert(River{source: 6, target: 5});
        rivers.insert(River{source: 5, target: 4});
        rivers.insert(River{source: 4, target: 3});
        rivers.insert(River{source: 3, target: 2});
        rivers.insert(River{source: 1, target: 7});
        rivers.insert(River{source: 1, target: 3});
        rivers.insert(River{source: 7, target: 5});
        rivers.insert(River{source: 5, target: 3});
        
        let mut mines: HashSet<SiteId> = HashSet::new();

        mines.insert(5);
        mines.insert(7);

        let map = Map {
            sites: sites,
            rivers: rivers,
            mines: mines,
        };

        let game_state = GameState {
            punter: 0,
            punters_count: 2,
            map: map,
            claims: HashMap::new(),
            moves: HashMap::new(),
        };
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 0, target: 1}, Move::Pass {punter: 1}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 2, target: 3}, Move::Claim {punter: 1, source: 1, target: 2}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 4, target: 5}, Move::Claim {punter: 1, source: 3, target: 4}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 6, target: 7}, Move::Claim {punter: 1, source: 5, target: 6}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 1, target: 3}, Move::Claim {punter: 1, source: 7, target: 0}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 5, target: 7}, Move::Claim {punter: 1, source: 3, target: 5}]);
        let (_, game_state) = game_state.play(vec![Move::Claim {punter: 0, source: 5, target: 7}, Move::Claim {punter: 1, source: 7, target: 1}]);

        assert_eq!(6, *&game_state.score_for(0));
        assert_eq!(6, *&game_state.score_for(1));
    }
}
