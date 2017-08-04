use super::map::Coords;

pub fn sq_dist(&(ar, ac): &Coords, &(br, bc): &Coords) -> usize {
    (((ar - br) * (ar - br)) + ((ac - bc) * (ac - bc))) as usize
}

pub mod a_star {
    use std::cmp::Ordering;
    use std::collections::{HashSet, BinaryHeap};
    use super::sq_dist;
    use super::super::map::{Tile, Coords};
    use super::super::game::{Game, GameState, Move};

    pub fn solve(game: &mut Game, initial_state: GameState) -> Option<Vec<(Move, GameState)>> {
        solve_debug(game, initial_state, |_moves, _st, _finished, _rtc, _cod, _ncnd, _nc| { })
    }

    pub fn solve_debug<F>(game: &mut Game, initial_state: GameState, mut step_debug_fn: F) -> Option<Vec<(Move, GameState)>>
        where F: FnMut(&Vec<(Move, GameState)>, &GameState, bool, usize, usize, usize, usize)
    {
        struct Node {
            state: GameState,
            path: Vec<(Move, GameState)>,
            finished: bool,
            rtc: usize, // reachable_tiles_count
            cod: usize, // crates_on_dst_count
            ncnd: usize, // nearest_crate_nearest_dst_sq_dist
            nc: usize, // nearest_crate_sq_dist
        }

        impl Node {
            fn new(
                state: GameState,
                path: Vec<(Move, GameState)>,
                rtc_trans: &mut Vec<Coords>,
                rtc_visited: &mut HashSet<Coords>)
                -> Node
            {
                Node {
                    path: path,
                    finished: state.finished(),
                    rtc: reachable_tiles_count(&state, rtc_trans, rtc_visited),
                    cod: crates_on_dst_count(&state),
                    ncnd: nearest_crate_nearest_dst_sq_dist(&state),
                    nc: nearest_crate_sq_dist(&state),
                    state: state,
                }
            }
        }

        impl PartialEq for Node {
            fn eq(&self, other: &Node) -> bool {
                self.state.placement == other.state.placement
            }
        }

        impl Eq for Node {}

        impl Ord for Node {
            fn cmp(&self, other: &Node) -> Ordering {
                self.finished.cmp(&other.finished)
                    .then(self.rtc.cmp(&other.rtc))
                    .then(self.cod.cmp(&other.cod))
                    .then(other.ncnd.cmp(&self.ncnd))
                    .then(other.nc.cmp(&self.nc))
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Node) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut visited = HashSet::new();
        let mut rtc_trans = Vec::new();
        let mut rtc_visited = HashSet::new();

        let mut pq = BinaryHeap::new();
        pq.push(Node::new(initial_state, Vec::new(), &mut rtc_trans, &mut rtc_visited));

        while let Some(Node { state, path, finished, rtc, cod, ncnd, nc, }) = pq.pop() {
            step_debug_fn(&path, &state, finished, rtc, cod, ncnd, nc);

            if finished {
                return Some(path);
            }
            visited.insert(state.placement.clone());

            for (move_, trans_state) in state.transitions(game) {
                if visited.contains(&trans_state.placement) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push((move_, trans_state.clone()));
                pq.push(Node::new(trans_state, next_path, &mut rtc_trans, &mut rtc_visited));
            }
        }

        None
    }

    fn crates_on_dst_count(state: &GameState) -> usize {
        state.placement
            .crates
            .iter()
            .filter(|&c| if let Some(&Tile::CrateDst) = state.room_at(c) {
                true
            } else {
                false
            })
            .count()
    }

    fn nearest_crate_with_sq_dist(state: &GameState) -> Option<(&Coords, usize)> {
        let pcoord = &state.placement.player;
        state.placement
            .crates
            .iter()
            .filter(|&c| if let Some(&Tile::CrateDst) = state.room_at(c) {
                false
            } else {
                true
            })
            .map(|coord| (coord, sq_dist(coord, pcoord)))
            .min_by_key(|&(_, d)| d)
    }

    fn nearest_crate_nearest_dst_sq_dist(state: &GameState) -> usize {
        nearest_crate_with_sq_dist(state)
            .and_then(|(coord, _)| {
                state.room
                    .crates_dsts
                    .iter()
                    .filter(|&&dc| !state.placement.crates.iter().any(|&c| c == dc))
                    .map(|dcoord| sq_dist(dcoord, coord))
                    .min()
            })
            .unwrap_or(0)
    }

    fn nearest_crate_sq_dist(state: &GameState) -> usize {
        nearest_crate_with_sq_dist(state)
            .map(|(_, d)| d)
            .unwrap_or(0)
    }

    fn reachable_tiles_count(state: &GameState, rtc_trans: &mut Vec<Coords>, rtc_visited: &mut HashSet<Coords>) -> usize {
        rtc_visited.clear();
        rtc_trans.clear();
        rtc_trans.push(state.placement.player);
        let mut reachable = 0;
        while let Some(coord) = rtc_trans.pop() {
            rtc_visited.insert(coord);
            reachable += 1;
            for &(dr, dc) in [(-1, 0), (0, 1), (1, 0), (0, -1)].iter() {
                let trans_coord = (coord.0 + dr, coord.1 + dc);
                match state.room_at(&trans_coord) {
                    Some(&Tile::Floor) | Some(&Tile::CrateDst) => (),
                    _ => continue,
                }
                if rtc_visited.contains(&trans_coord) {
                    continue;
                }
                if state.placement.crates.iter().any(|&c| c == trans_coord) {
                    continue;
                }
                rtc_trans.push(trans_coord);
            }
        }
        reachable
    }
}
