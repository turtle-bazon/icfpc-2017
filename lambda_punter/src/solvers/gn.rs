use std::time;
use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};
use rand::{self, Rng};

use super::super::types::{PunterId, SiteId};
use super::super::map::{River, RiversIndex};
use super::super::proto::{Move, Setup, Future};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::{Graph, GraphCache, EdgeAttr};
use super::super::prob;

pub struct GNGameStateBuilder;

impl GameStateBuilder for GNGameStateBuilder {
    type GameState = GNGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        let timeout_start = time::Instant::now();
        let max_timeout = time::Duration::from_secs(9);

        // make map graph
        let rivers_graph = Graph::from_map(&setup.map);
        let mut gcache = Default::default();

        // calculate betweenness coeffs
        let rivers_bw = RiversIndex::from_hash_map(
            rivers_graph.rivers_betweenness(&mut gcache));

        let mut futures = None;
        if setup.settings.futures {
            // in case there is futures support, try to estimate the best ones
            let mut mcache = Default::default();
            let mut futures_estimated = Vec::with_capacity(setup.map.mines.len());
            for &mine in setup.map.mines.iter() {
                if let Some(time_avail) = max_timeout.checked_sub(timeout_start.elapsed()) {
                    debug!("guessing a future for mine {}, {:?} time left", mine, time_avail);
                    let future_guess =
                        prob::estimate_best_future(
                            &rivers_graph,
                            mine,
                            &setup.map.mines,
                            &rivers_bw,
                            3,
                            4,
                            |path_rivers, claimed_rivers| {
                                path_rivers
                                    .iter()
                                    .filter(|&r| !claimed_rivers.contains_key(r))
                                    .max_by_key(|&r| rivers_bw.get(r).map(|bw| (bw * 1000.0) as u64).unwrap_or(0))
                            },
                            max(setup.map.rivers.len(), 128),
                            time_avail,
                            &mut mcache,
                            &mut gcache);
                    if let Some((source, target)) = future_guess {
                        debug!("guessed a future from {} to {}", source, target);
                        futures_estimated.push(Future { source: source, target: target, });
                    }
                } else {
                    debug!("no more futures guessing, time is expired");
                    break;
                }
            }

            if !futures_estimated.is_empty() {
                futures = Some(futures_estimated);
            }
        }

        let goals: Vec<_> = if let Some(ref futs) = futures {
            // build goals from futures
            futs.iter().map(|fut| (fut.source, fut.target)).collect()
        } else {
            // in case there is no futures support or we have failed to build a future, try to link mines pairwise
            let mut mine_pairs = HashMap::new();
            if setup.map.mines.len() < 2 {
                debug!("there is only one mine on this map");
                if let Some(&mine) = setup.map.mines.iter().next() {
                    if let Some(path) = rivers_graph.longest_jouney_from(mine, &mut gcache) {
                        if let Some(&longest_jouney_site) = path.last() {
                            debug!("longest jouney choosen from mine {} to {}", mine, longest_jouney_site);
                            let key = (min(mine, longest_jouney_site), max(mine, longest_jouney_site));
                            mine_pairs.insert(key, path.to_owned());
                        }
                    }
                }
            } else {
                debug!("there are {} mines on this map", setup.map.mines.len());
                for &mine_a in setup.map.mines.iter() {
                    for &mine_b in setup.map.mines.iter() {
                        let key = (min(mine_a, mine_b), max(mine_a, mine_b));
                        if (mine_a != mine_b) && !mine_pairs.contains_key(&key) {
                            if let Some(path) = rivers_graph.shortest_path_only(key.0, key.1, &mut gcache) {
                                mine_pairs.insert(key, path.to_owned());
                            }
                        }
                    }
                }
            }

            let mut pairs: Vec<_> = mine_pairs.into_iter().collect();
            pairs.sort_by_key(|p| (p.1).len());
            pairs.into_iter().map(|p| ((p.0).0, (p.0).1)).collect()
        };

        debug!("initially choosen {} goals", goals.len());

        GNGameState {
            punter: setup.punter,
            rivers: setup.map.rivers,
            rivers_graph: rivers_graph,
            goals: goals,
            claimed_rivers: Default::default(),
            futures: futures,
            mines_connected_sites: HashSet::new(),
            rivers_bw: rivers_bw,
        }
    }
}

type ClaimedRivers = RiversIndex<u64>;

#[derive(Serialize, Deserialize)]
pub struct GNGameState {
    punter: PunterId,
    rivers: Vec<River>,
    rivers_graph: Graph,
    goals: Vec<(SiteId, SiteId)>,
    claimed_rivers: ClaimedRivers,
    futures: Option<Vec<Future>>,
    mines_connected_sites: HashSet<SiteId>,
    rivers_bw: RiversIndex<f64>,
}

impl GameState for GNGameState {
    type Error = ();

    fn play(mut self, moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
        self.update_moves(moves);
        let mut gcache = Default::default();
        loop {
            while let Some((source, target)) = self.goals.pop() {
                debug!("found current goal: from {} to {}", source, target);
                let maybe_path = self.shortest_path(source, target, &mut gcache);
                if let Some(path) = maybe_path {
                    debug!("there is a path for goal from {} to {}: {:?}", source, target, path);
                    if let Some((ps, pt)) = self.choose_route_segment(path) {
                        let move_ = Move::Claim { punter: self.punter, source: ps, target: pt, };
                        self.goals.push((target, source));
                        self.mines_connected_sites.insert(ps);
                        self.mines_connected_sites.insert(pt);
                        return Ok((move_, self));
                    }
                }
                debug!("no route from {} to {}, proceeding with next", source, target);
            }
            debug!("no more goals to reach, choosing a new random one");

            // all current goals are reached for now, let's choose a random free river connected to our already existing path
            let mut rng = rand::thread_rng();
            let mut new_goal = None;
            rng.shuffle(&mut self.rivers);
            for river in self.rivers.iter() {
                if !self.claimed_rivers.contains_key(river) {
                    for &mine_site in self.mines_connected_sites.iter() {
                        if self.shortest_path(river.source, mine_site, &mut gcache).is_some() {
                            debug!("fallback: new goal is chosen: from {} (as a part of mine path) to {}", mine_site, river.target);
                            let move_ = Move::Claim { punter: self.punter, source: river.source, target: river.target, };
                            new_goal = Some((move_, mine_site, river.source, river.target));
                            break;
                        }
                    }
                }
                if new_goal.is_some() {
                    break;
                }
            }

            return Ok(if let Some((move_, ms, rs, rt)) = new_goal {
                // new goal is choosen
                self.goals.push((ms, rs));
                self.mines_connected_sites.insert(rs);
                self.mines_connected_sites.insert(rt);
                (move_, self)
            } else {
                // no new goals, claim some random river if any
                let move_ = {
                    let free_rivers: Vec<_> = self.rivers
                        .iter()
                        .filter(|r| !self.claimed_rivers.contains_key(r))
                        .collect();
                    if let Some(river) = rng.choose(&free_rivers) {
                        Move::Claim { punter: self.punter, source: river.source, target: river.target, }
                    } else {
                        // no more rivers to claim
                        Move::Pass { punter: self.punter, }
                    }
                };
                (move_, self)
            });
        }
    }

    fn stop(mut self, moves: Vec<Move>) -> Result<Self, Self::Error> {
        self.update_moves(moves);
        debug!("STOP command invoked");
        if let Some(ref futures) = self.futures {
            let mut gcache = Default::default();
            for &Future { source, target, } in futures.iter() {
                let completed = self.shortest_path(source, target, &mut gcache).is_some();
                debug!("future from {} to {}: {}", source, target, if completed { "SUCCESS" } else { "FAILED" });
            }
        }
        Ok(self)
    }

    fn get_punter(&self) -> PunterId {
        self.punter
    }

    fn get_futures(&mut self) -> Option<Vec<Future>> {
        self.futures.clone()
    }
}

impl GNGameState {
    fn update_moves(&mut self, moves: Vec<Move>) {
        for move_ in moves {
            match move_ {
                Move::Claim { punter, source, target, } => {
                    self.claimed_rivers.insert(River::new(source, target), (1 << punter));
                },
                Move::Pass { .. } =>
                    (),
                Move::Splurge { punter, route, } => {
                    let mut offset = 0;
                    while let (Some(&source), Some(&target)) = (route.get(offset), route.get(offset + 1)) {
                        self.claimed_rivers.insert(River::new(source, target), punter);
                        offset += 1;
                    }
                },
                Move::Option { punter, source, target, } => {
                    *self.claimed_rivers.entry(River::new(source, target)).or_insert(0) |= 1 << punter;
                },
            }
        }
    }

    fn shortest_path<'a>(&self, source: SiteId, target: SiteId, gcache: &'a mut GraphCache) -> Option<&'a [SiteId]> {
        let my_punter = self.punter;
        let claimed_rivers = &self.claimed_rivers;
        let probe_claimed = |(s, t)| claimed_rivers
            .get(&River::new(s, t))
            .map(|&river_owner| if river_owner & (1 << my_punter) != 0 {
                EdgeAttr::Accessible { edge_cost: 0, }
            } else {
                EdgeAttr::Blocked
            })
            .unwrap_or(EdgeAttr::Accessible { edge_cost: 1, });

        self.rivers_graph.shortest_path(source, target, gcache, probe_claimed)
    }

    fn choose_route_segment(&self, path: &[SiteId]) -> Option<(SiteId, SiteId)> {
        let mut best = None;
        let mut offset = 0;
        while let (Some(&ps), Some(&pt)) = (path.get(offset), path.get(offset + 1)) {
            let wanted_river = River::new(ps, pt);
            if self.claimed_rivers.get(&wanted_river).map(|&p| p & (1 << self.punter) != 0).unwrap_or(false) {
                debug!("  -- from {} to {}: already claimed by me", ps, pt);
            } else {
                let bw_coeff = self.rivers_bw
                    .get(&wanted_river)
                    .cloned()
                    .unwrap_or(0.0);
                debug!("  -- from {} to {}: bw_coeff = {}", ps, pt, bw_coeff);
                best = match best {
                    Some((best_river, best_bw_coeff)) => if bw_coeff < best_bw_coeff {
                        Some((best_river, best_bw_coeff))
                    } else {
                        Some((wanted_river, bw_coeff))
                    },
                    _ =>
                        Some((wanted_river, bw_coeff)),
                };
            }
            offset += 1;
        }

        if let Some((river, bw_coeff)) = best {
            debug!("choosing {:?} because of maximum bw_coeff: {}", river, bw_coeff);
            Some((river.source, river.target))
        } else {
            None
        }
    }
}
