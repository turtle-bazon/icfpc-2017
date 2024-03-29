use std::time;
use rand;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

use super::types::{SiteId, PunterId};
use super::map::{River, RiversIndex};
use super::graph::{Graph, GraphCache, EdgeAttr, StepCommand};

#[derive(Default)]
pub struct MonteCarloCache {
    claimed_rivers: RiversIndex<PunterId>,
    weighted: Vec<Weighted<River>>,
    route_rivers: Vec<River>,
}

impl MonteCarloCache {
    pub fn clear(&mut self) {
        self.claimed_rivers.clear();
        self.weighted.clear();
        self.route_rivers.clear();
    }
}

pub fn journey_success_simulate<F>(
    journey: &[SiteId],
    rivers_bw: &RiversIndex<f64>,
    my_punter: PunterId,
    punters_count: usize,
    start_turn: usize,
    make_move: F,
    games_count: usize,
    cache: &mut MonteCarloCache,
)
    -> Option<f64>
    where F: for<'a> Fn(&'a [River], &RiversIndex<PunterId>) -> Option<&'a River>
{
    let journey_len = journey.len();
    if (journey_len < 2) || (punters_count < 2) {
        return None;
    }

    // collect journey rivers
    cache.route_rivers.clear();
    let mut offset = 0;
    while let (Some(&ps), Some(&pt)) = (journey.get(offset), journey.get(offset + 1)) {
        cache.route_rivers.push(River::new(ps, pt));
        offset += 1;
    }

    // calculate scale coeff for bw values
    let bw_sum: f64 = rivers_bw.values().sum();
    let bw_scale = (u32::max_value() as f64) / bw_sum;

    // play `games_count` times and gather stats
    let mut success_count = 0;
    for _ in 0 .. games_count {
        enum Outcome { Success, Fail, };

        fn play<F>(
            rivers_bw: &RiversIndex<f64>,
            my_punter: PunterId,
            punters_count: usize,
            start_turn: usize,
            make_move: F,
            bw_scale: f64,
            cache: &mut MonteCarloCache,
        )
            -> Outcome
            where F: for<'a> Fn(&'a [River], &RiversIndex<PunterId>) -> Option<&'a River>
        {
            cache.claimed_rivers.clear();
            let mut rng = rand::thread_rng();
            let mut turn_counter = 0;
            loop {
                // check if journey is finished or blocked
                let mut finished = true;
                for river in cache.route_rivers.iter() {
                    match cache.claimed_rivers.get(river) {
                        Some(&river_owner) if river_owner == my_punter =>
                            continue,
                        Some(..) =>
                            return Outcome::Fail,
                        None => {
                            finished = false;
                            break;
                        }
                    }
                }
                if finished {
                    return Outcome::Success;
                }

                let turn = turn_counter % punters_count;
                if (turn_counter >= start_turn) && (turn == my_punter as usize) {
                    // it's a my turn, perform a move
                    if let Some(river) = make_move(&cache.route_rivers, &cache.claimed_rivers) {
                        cache.claimed_rivers.insert(river.clone(), my_punter);
                    }
                } else {
                    // it's an enemy turn, simulate a move
                    let enemy_punter = turn as PunterId;

                    // prepare distribution set from non-claimed rivers
                    let river = {
                        cache.weighted.clear();
                        let claimed_rivers = &cache.claimed_rivers;
                        cache.weighted.extend(
                            rivers_bw
                                .iter()
                                .filter(|&(river, _)| !claimed_rivers.contains_key(river))
                                .map(|(river, bw_coeff)| Weighted {
                                    weight: (bw_coeff * bw_scale) as u32,
                                    item: river.clone(),
                                }));
                        let choice = WeightedChoice::new(&mut cache.weighted);
                        // pick a river
                        choice.ind_sample(&mut rng)
                    };
                    cache.claimed_rivers.insert(river, enemy_punter);
                }

                turn_counter += 1;
            }
        }

        match play(rivers_bw, my_punter, punters_count, start_turn, &make_move, bw_scale, cache) {
            Outcome::Success =>
                success_count += 1,
            Outcome::Fail =>
                (),
        }
    }
    Some(success_count as f64 / games_count as f64)
}

pub fn estimate_best_future<F>(
    graph: &Graph,
    mine: SiteId,
    mines: &[SiteId],
    rivers_bw: &RiversIndex<f64>,
    my_punter: PunterId,
    punters_count: usize,
    start_turn: usize,
    make_move: F,
    games_count: usize,
    time_limit: time::Duration,
    mcache: &mut MonteCarloCache,
    gcache: &mut GraphCache<f64>,
)
    -> Option<(SiteId, SiteId, usize)>
    where F: for<'a> Fn(&'a [River], &RiversIndex<PunterId>) -> Option<&'a River>
{
    let mut best = None;
    let timeout_start = time::Instant::now();
    graph.generic_bfs(mine, 0.0, |path, cost, prev_reward| {
        if timeout_start.elapsed() > time_limit {
            return StepCommand::Terminate;
        }
        if let (Some(&source), Some(&target)) = (path.first(), path.last()) {
            if mines.iter().any(|&m| m == target) {
                StepCommand::Continue(0.0)
            } else {
                let maybe_prob = journey_success_simulate(
                    path,
                    rivers_bw,
                    my_punter,
                    punters_count,
                    start_turn,
                    &make_move,
                    games_count,
                    mcache);
                if let Some(prob) = maybe_prob {
                    let regular_reward = cost * cost;
                    let future_reward = cost * cost * cost;
                    let expected_reward = future_reward as f64 * prob;
                    // check if it is worth to take this future
                    if expected_reward < regular_reward as f64 {
                        StepCommand::Continue(0.0)
                    } else {
                        // track the best future candidate
                        best = Some(if let Some((best_reward, best_fut)) = best.take() {
                            if best_reward < expected_reward {
                                (expected_reward, (source, target, path.len()))
                            } else {
                                (best_reward, best_fut)
                            }
                        } else {
                            (expected_reward, (source, target, path.len()))
                        });
                        // check if there is no sense to move futher
                        if &expected_reward > prev_reward {
                            StepCommand::Continue(expected_reward)
                        } else {
                            StepCommand::Stop
                        }
                    }
                } else {
                    StepCommand::Stop
                }
            }
        } else {
            StepCommand::Stop
        }
    }, EdgeAttr::standard, gcache);

    best.map(|v| v.1)
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use std::collections::HashSet;
    use super::super::types::PunterId;
    use super::super::graph::Graph;
    use super::super::map::{River, RiversIndex};
    use super::super::test_common::*;
    use super::{journey_success_simulate, estimate_best_future};

    fn sample_map() -> (Graph, RiversIndex<f64>) {
        let graph = Graph::from_iter(
            [(3, 4), (0, 1), (2, 3), (1, 3), (5, 6), (4, 5), (3, 5), (6, 7), (5, 7), (1, 7), (0, 7), (1, 2)]
                .iter()
                .cloned());
        let mut gcache = Default::default();
        let rivers_bw = RiversIndex::from_hash_map(graph.rivers_betweenness::<()>(&mut gcache));
        (graph, rivers_bw)
    }

    fn make_move<'a>(route: &'a [River], claimed_rivers: &RiversIndex<PunterId>) -> Option<&'a River> {
        route.iter().find(|river| !claimed_rivers.contains_key(river))
    }

    #[test]
    fn sample_map_simulation_always_success() {
        let (_, rivers_bw) = sample_map();
        let mut mcache = Default::default();

        let prob = journey_success_simulate(&[1, 0], &rivers_bw, 0, 2, 0, make_move, 100, &mut mcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 2], &rivers_bw, 0, 2, 0, make_move, 100, &mut mcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 3], &rivers_bw, 0, 2, 0, make_move, 100, &mut mcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 7], &rivers_bw, 0, 2, 0, make_move, 100, &mut mcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
    }

    #[test]
    fn sample_map_simulation() {
        let (graph, rivers_bw) = sample_map();
        let mut gcache = Default::default();
        let mut mcache = Default::default();

        let all_other_sites: HashSet<_> = rivers_bw
            .iter()
            .flat_map(|(river, _)| Some(river.source).into_iter().chain(Some(river.target).into_iter()))
            .collect();
        let mut results: Vec<_> = all_other_sites
            .into_iter()
            .filter(|&site| site != 1)
            .flat_map(|target| graph.shortest_path_only::<()>(1, target, &mut gcache).map(|v| v.to_owned()))
            .map(|route| {
                let prob = journey_success_simulate(&route, &rivers_bw, 1, 2, 0, make_move, 10000, &mut mcache);
                (route, prob)
            })
            .collect();
        results.sort_by_key(|&(_, v)| v.map(|w| (w * 10000.0) as usize));
        assert_eq!(results[0].0.last().unwrap(), &5);
        assert!((results[1].0.last().unwrap() == &4) || (results[1].0.last().unwrap() == &6));
        assert!((results[2].0.last().unwrap() == &4) || (results[2].0.last().unwrap() == &6));
        assert!((results[3].0.last().unwrap() == &3) || (results[3].0.last().unwrap() == &7));
        assert!((results[4].0.last().unwrap() == &3) || (results[4].0.last().unwrap() == &7));
        assert!((results[5].0.last().unwrap() == &0) || (results[5].0.last().unwrap() == &2));
        assert!((results[6].0.last().unwrap() == &0) || (results[6].0.last().unwrap() == &2));
        // results are something like these:
        // [
        //     ([1, 3, 5], Some(0.7036)),
        //     ([1, 3, 4], Some(0.7481)),
        //     ([1, 7, 6], Some(0.752)),
        //     ([1, 7], Some(0.9015)),
        //     ([1, 3], Some(0.9028)),
        //     ([1, 2], Some(0.9221)),
        //     ([1, 0], Some(0.9236)),
        // ]
    }

    #[test]
    fn sample_map_best_future() {
        let (graph, rivers_bw) = sample_map();
        let mut gcache = Default::default();
        let mut mcache = Default::default();

        let future =
            estimate_best_future(&graph, 1, &[1, 5], &rivers_bw, 1, 2, 0, make_move,
                                 10000, Duration::from_millis(5000), &mut mcache, &mut gcache).unwrap();
        assert!((future.1 == 4) || (future.1 == 6));
    }

    #[test]
    fn random_medium_map_best_future() {
        let graph = random_medium_map_graph();
        let mut gcache = Default::default();
        let mut mcache = Default::default();
        let rivers_bw = RiversIndex::from_hash_map(graph.rivers_betweenness(&mut gcache));

        estimate_best_future(&graph, 60, &[60, 10, 31, 33], &rivers_bw, 3, 4, 0, make_move,
                             187, Duration::from_millis(5000), &mut mcache, &mut gcache).unwrap();
    }
}
