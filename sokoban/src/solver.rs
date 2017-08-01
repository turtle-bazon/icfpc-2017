
pub mod a_star {
    use super::super::game::{Game, GameState, Move};

    pub fn solve(game: &mut Game, initial_state: GameState) -> Option<Vec<(Move, GameState)>> {
        unimplemented!()

            // for (move_, trans_state) in initial_state.transitions(&mut game) {
            //     println!("Transition found for {:?} move:", move_);
            //     println!("{}", trans_state);
            // }
    }
}
