use rand;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

use super::types::{SiteId, PunterId};
use super::map::{River, RiversIndex};

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

    // play `games_count` times and gather stats
    let mut success_count = 0;
    for _ in 0 .. games_count {
        enum Outcome { Success, Fail, };

        fn play<F>(
            rivers_bw: &RiversIndex<f64>,
            my_punter: PunterId,
            punters_count: usize,
            make_move: F,
            cache: &mut MonteCarloCache,
        )
            -> Outcome
            where F: for<'a> Fn(&'a [River], &RiversIndex<PunterId>) -> Option<&'a River>
        {
            cache.claimed_rivers.clear();
            let mut rng = rand::thread_rng();
            let mut turn = 0;
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

                if turn == my_punter as usize {
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
                                    weight: (bw_coeff * 1000.0) as u32,
                                    item: river.clone(),
                                }));
                        let choice = WeightedChoice::new(&mut cache.weighted);
                        // pick a river
                        choice.ind_sample(&mut rng)
                    };
                    cache.claimed_rivers.insert(river, enemy_punter);
                }

                turn = (turn + 1) % punters_count;
            }
        }

        match play(rivers_bw, my_punter, punters_count, &make_move, cache) {
            Outcome::Success =>
                success_count += 1,
            Outcome::Fail =>
                (),
        }
    }
    Some(success_count as f64 / games_count as f64)
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;
    use super::super::types::PunterId;
    use super::super::graph::Graph;
    use super::super::map::{River, RiversIndex};
    use super::journey_success_simulate;

    fn sample_map() -> (Graph, RiversIndex<f64>) {
        let graph = Graph::from_iter(
            [(3, 4), (0, 1), (2, 3), (1, 3), (5, 6), (4, 5), (3, 5), (6, 7), (5, 7), (1, 7), (0, 7), (1, 2)]
                .iter()
                .cloned());
        let mut gcache = Default::default();
        let rivers_bw = RiversIndex::from_hash_map(graph.rivers_betweenness(&mut gcache));
        (graph, rivers_bw)
    }

    fn make_move<'a>(route: &'a [River], claimed_rivers: &RiversIndex<PunterId>) -> Option<&'a River> {
        route.iter().find(|river| !claimed_rivers.contains_key(river))
    }

    #[test]
    fn sample_map_simulation_always_success() {
        let (_, rivers_bw) = sample_map();
        let mut pcache = Default::default();

        let prob = journey_success_simulate(&[1, 0], &rivers_bw, 0, 2, make_move, 100, &mut pcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 2], &rivers_bw, 0, 2, make_move, 100, &mut pcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 3], &rivers_bw, 0, 2, make_move, 100, &mut pcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
        let prob = journey_success_simulate(&[1, 7], &rivers_bw, 0, 2, make_move, 100, &mut pcache);
        assert_eq!(prob.map(|v| (v * 100.0) as usize), Some(100));
    }

    #[test]
    fn sample_map_simulation() {
        let (graph, rivers_bw) = sample_map();
        let mut gcache = Default::default();
        let mut pcache = Default::default();

        let all_other_sites: HashSet<_> = rivers_bw
            .iter()
            .flat_map(|(river, _)| Some(river.source).into_iter().chain(Some(river.target).into_iter()))
            .collect();
        let mut results: Vec<_> = all_other_sites
            .into_iter()
            .filter(|&site| site != 1)
            .flat_map(|target| graph.shortest_path_only(1, target, &mut gcache).map(|v| v.to_owned()))
            .map(|route| {
                let prob = journey_success_simulate(&route, &rivers_bw, 1, 2, make_move, 10000, &mut pcache);
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
    }

}
