use std::{time, thread};
use std::cmp::{min, max};
use std::sync::{mpsc, Arc};
use std::collections::HashMap;
use rand::{self, Rng};

use super::super::types::{PunterId, SiteId};
use super::super::map::{River, RiversIndex};
use super::super::proto::{Move, Setup, Future};
use super::super::game::{GameState, GameStateBuilder};
use super::super::graph::{Graph, GraphCache, EdgeAttr, StepCommand};
use super::super::prob;

pub struct GNGameStateBuilder;

impl GameStateBuilder for GNGameStateBuilder {
    type GameState = GNGameState;

    fn build(self, setup: Setup) -> Self::GameState {
        let timeout_start = time::Instant::now();
        let max_timeout = time::Duration::from_secs(8);

        // make map graph
        let rivers_graph = Arc::new(Graph::from_map(&setup.map));
        let mut gcache = Default::default();

        // calculate betweenness coeffs
        let rivers_bw = Arc::new(RiversIndex::from_hash_map(
            rivers_graph.rivers_betweenness::<()>(&mut gcache)));

        let mut futures = None;
        if setup.settings.futures {
            // in case there is futures support, try to estimate the best ones
            let mines = Arc::new(setup.map.mines.to_owned());
            let mut futures_estimated = Vec::with_capacity(setup.map.mines.len());
            let mut start_turn = 0;
            for &mine in setup.map.mines.iter() {
                if let Some(time_avail) = max_timeout.checked_sub(timeout_start.elapsed()) {
                    debug!("guessing a future for mine {}, {:?} time left", mine, time_avail);
                    let (tx, rx) = mpsc::channel();
                    let rivers_graph = rivers_graph.clone();
                    let rivers_bw = rivers_bw.clone();
                    let mines = mines.clone();
                    let punter = setup.punter;
                    let punters = setup.punters;
                    let rivers_count = setup.map.rivers.len();
                    thread::spawn(move || {
                        tx.send(prob::estimate_best_future(
                            &rivers_graph,
                            mine,
                            &mines,
                            &rivers_bw,
                            punter,
                            punters,
                            start_turn,
                            |path_rivers, claimed_rivers| {
                                path_rivers
                                    .iter()
                                    .filter(|&r| !claimed_rivers.contains_key(r))
                                    .max_by_key(|&r| rivers_bw.get(r).map(|bw| (bw * 1000.0) as u64).unwrap_or(0))
                            },
                            min(max(rivers_count, 128), 1024),
                            time_avail,
                            &mut Default::default(),
                            &mut Default::default())).ok();
                    });
                    match rx.recv_timeout(time_avail) {
                        Ok(Some((source, target, path_len))) => {
                            debug!("guessed a future from {} to {} (path len = {})", source, target, path_len);
                            futures_estimated.push(Future { source: source, target: target, });
                            start_turn += path_len * setup.punters;
                        },
                        Ok(None) => {
                            debug!("cannot estimate any future for mine {}, proceeding with next", mine);
                        },
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            debug!("no more futures guessing, bg thread is timed out");
                            break;
                        },
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            error!("bg thread suddenly disconnected");
                        },
                    }
                } else {
                    debug!("no more futures guessing, time is expired");
                    break;
                }
            }

            if !futures_estimated.is_empty() {
                futures_estimated.reverse();
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
            rivers_graph: ArcSerDe(rivers_graph),
            goals: goals,
            claimed_rivers: Default::default(),
            futures: futures,
            mines: setup.map.mines.to_owned(),
            rivers_bw: ArcSerDe(rivers_bw),
            options_left: if setup.settings.options { setup.map.mines.len() } else { 0 },
        }
    }
}

type ClaimedRivers = RiversIndex<u64>;

#[derive(Serialize, Deserialize)]
pub struct GNGameState {
    punter: PunterId,
    rivers: Vec<River>,
    rivers_graph: ArcSerDe<Graph>,
    goals: Vec<(SiteId, SiteId)>,
    claimed_rivers: ClaimedRivers,
    futures: Option<Vec<Future>>,
    mines: Vec<SiteId>,
    rivers_bw: ArcSerDe<RiversIndex<f64>>,
    options_left: usize,
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
                    if let Some(move_) = self.choose_route_segment(path) {
                        if let Move::Option { .. } = move_ {
                            self.options_left -= 1;
                        }
                        self.goals.push((target, source));
                        return Ok((move_, self));
                    }
                }
                debug!("no route from {} to {}, proceeding with next", source, target);
            }
            debug!("no more goals to reach, choosing a new random one");

            // all current goals are reached for now, let's choose a fallback move
            let new_goal_path = self.choose_fallback(&mut gcache);
            if let Some((path, source, target)) = new_goal_path {
                // new goal is choosen
                if let Some(move_) = self.choose_route_segment(&path) {
                    if let Move::Option { .. } = move_ {
                        self.options_left -= 1;
                    }
                    self.goals.push((source, target));
                    return Ok((move_, self));
                }
            }

            // no new goals, claim some random river if any
            let move_ = {
                let free_rivers: Vec<_> = self.rivers
                    .iter()
                    .filter(|r| !self.claimed_rivers.contains_key(r))
                    .collect();
                let mut rng = rand::thread_rng();
                if let Some(river) = rng.choose(&free_rivers) {
                    Move::Claim { punter: self.punter, source: river.source, target: river.target, }
                } else {
                    // no more rivers to claim
                    Move::Pass { punter: self.punter, }
                }
            };
            return Ok((move_, self))
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

    fn shortest_path<'a>(&self, source: SiteId, target: SiteId, gcache: &'a mut GraphCache<usize>) -> Option<&'a [SiteId]> {
        let my_punter = self.punter;
        let claimed_rivers = &self.claimed_rivers;
        let options_left = self.options_left;
        self.rivers_graph.generic_bfs(source, options_left, |path, _cost, &options_left| {
            if let Some(&pt) = path.last() {
                if pt == target {
                    // reached the target
                    StepCommand::Terminate
                } else if path.len() > 1 {
                    // maybe we could use an option
                    let len = path.len();
                    if let Some(&ps) = path.get(len - 2) {
                        if claimed_rivers
                            .get(&River::new(ps, pt))
                            .map(|river_owner| river_owner & (1 << my_punter) == 0)
                            .unwrap_or(false)
                        {
                            if options_left > 0 {
                                StepCommand::Continue(options_left - 1)
                            } else {
                                StepCommand::Stop
                            }
                        } else {
                            StepCommand::Continue(options_left)
                        }
                    } else {
                        StepCommand::Continue(options_left)
                    }
                } else {
                    StepCommand::Continue(options_left)
                }
            } else {
                StepCommand::Stop
            }
        }, |(s, t)| {
            claimed_rivers
                .get(&River::new(s, t))
                .map(|&river_owner| if river_owner & (1 << my_punter) != 0 {
                    EdgeAttr::Accessible { edge_cost: 0, }
	        } else if river_owner.count_ones() > 1 {
                    EdgeAttr::Blocked
                } else if options_left > 0 {
                    // it is an enemy river, but there is a chance to buy an option for it
                    EdgeAttr::Accessible { edge_cost: 1, }
                } else {
                    // no options -- no chance
                    EdgeAttr::Blocked
                })
                .unwrap_or(EdgeAttr::Accessible { edge_cost: 1, })
        }, gcache)
    }

    fn choose_route_segment(&self, path: &[SiteId]) -> Option<Move> {
        let mut best = None;
        let mut offset = 0;
        while let (Some(&ps), Some(&pt)) = (path.get(offset), path.get(offset + 1)) {
            let wanted_river = River::new(ps, pt);
            let river_owner = self.claimed_rivers.get(&wanted_river).cloned();
            if river_owner.map(|p| p & (1 << self.punter) != 0).unwrap_or(false) {
                debug!("  -- from {} to {}: already claimed by me", ps, pt);
            } else {
                let bw_coeff = self.rivers_bw
                    .get(&wanted_river)
                    .cloned()
                    .unwrap_or(0.0);
                debug!("  -- from {} to {}{}: bw_coeff = {}",
                       ps, pt, if river_owner.is_some() { " (NEED OPTION)" } else { "" }, bw_coeff);
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
            if self.claimed_rivers.contains_key(&river) {
                debug!("choosing OPTION {:?} ({} left) because of maximum bw_coeff: {}", river, self.options_left, bw_coeff);
                if self.options_left > 0 {
                    Some(Move::Option { punter: self.punter, source: river.source, target: river.target, })
                } else {
                    error!("something wrong with my solver: choosing OPTION while no options left");
                    None
                }
            } else {
                debug!("choosing CLAIM {:?} because of maximum bw_coeff: {}", river, bw_coeff);
                Some(Move::Claim { punter: self.punter, source: river.source, target: river.target, })
            }
        } else {
            None
        }
    }

    fn choose_fallback(&mut self, gcache: &mut GraphCache<usize>) -> Option<(Vec<SiteId>, SiteId, SiteId)> {
        let mut rng = rand::thread_rng();
        rng.shuffle(&mut self.mines);
        for &mine in self.mines.iter() {
            debug!("fallback: trying to upgrade route from mine {}", mine);
            let my_punter = self.punter;
            let claimed_rivers = &self.claimed_rivers;
            let options_left = self.options_left;
            let mut best = None;
            self.rivers_graph.generic_bfs(mine, options_left, |path, cost, &options_left| {
                if let Some(&pt) = path.last() {
                    let cmd = if path.len() > 1 {
                        // maybe we could use an option
                        let len = path.len();
                        if let Some(&ps) = path.get(len - 2) {
                            if claimed_rivers
                                .get(&River::new(ps, pt))
                                .map(|river_owner| river_owner & (1 << my_punter) == 0)
                                .unwrap_or(false)
                            {
                                if options_left > 0 {
                                    StepCommand::Continue(options_left - 1)
                                } else {
                                    StepCommand::Stop
                                }
                            } else {
                                StepCommand::Continue(options_left)
                            }
                        } else {
                            StepCommand::Continue(options_left)
                        }
                    } else {
                        StepCommand::Continue(options_left)
                    };

                    if let StepCommand::Continue(..) = cmd {
                        best = Some(if let Some((best_cost, best_path, best_target)) = best.take() {
                            if best_cost < cost {
                                (cost, path.to_owned(), pt)
                            } else {
                                (best_cost, best_path, best_target)
                            }
                        } else {
                            (cost, path.to_owned(), pt)
                        });
                    }
                    cmd
                } else {
                    StepCommand::Stop
                }
            }, |(s, t)| {
                claimed_rivers
                    .get(&River::new(s, t))
                    .map(|&river_owner| if river_owner & (1 << my_punter) != 0 {
                        EdgeAttr::Accessible { edge_cost: 0, }
	            } else if river_owner.count_ones() > 1 {
                        EdgeAttr::Blocked
                    } else if options_left > 0 {
                        // it is an enemy river, but there is a chance to buy an option for it
                        EdgeAttr::Accessible { edge_cost: 1, }
                    } else {
                        // no options -- no chance
                        EdgeAttr::Blocked
                    })
                    .unwrap_or(EdgeAttr::Accessible { edge_cost: 1, })
            }, gcache);
            if let Some((best_cost, best_path, best_target)) = best {
                if best_path.len() > 1 {
                    debug!("fallback: going path {:?} with best cost = {}", best_path, best_cost);
                    return Some((best_path, mine, best_target));
                }
            }
        }
        debug!("fallback: none found");
        None
    }
}

struct ArcSerDe<T>(Arc<T>);

use std::ops::Deref;

impl<T> Deref for ArcSerDe<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

use serde::{ser, de};

impl<T> ser::Serialize for ArcSerDe<T> where T: ser::Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: ser::Serializer {
        self.0.serialize(serializer)
    }
}

impl<'de, T> de::Deserialize<'de> for ArcSerDe<T> where T: de::Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: de::Deserializer<'de> {
        let item: T = de::Deserialize::deserialize(deserializer)?;
        Ok(ArcSerDe(Arc::new(item)))
    }
}
