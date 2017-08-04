use std::fmt;
use std::sync::Arc;
use std::collections::HashSet;
use super::map::{Coords, Tile, Room};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Placement {
    pub player: Coords,
    pub crates: Arc<Vec<Coords>>,
}

#[derive(Clone)]
pub struct GameState {
    pub room: Arc<Room>,
    pub placement: Placement,
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Room {}x{}, {} crates",
               self.room.width,
               self.room.height,
               self.room.crates_dsts.len())?;
        for (coord, tile) in self.room.content.iter().enumerate() {
            let row = (coord / self.room.width) as isize;
            let col = (coord % self.room.width) as isize;
            if col == 0 {
                writeln!(f, "")?;
            }
            let pos = (row, col);
            if self.placement.player == pos {
                write!(f, "I")?;
            } else if self.placement.crates.iter().any(|&coord| coord == pos) {
                if let &Tile::CrateDst = tile {
                    write!(f, "*")?;
                } else {
                    write!(f, "+")?;
                }
            } else {
                write!(f, "{}", match tile {
                    &Tile::Wall => '#',
                    &Tile::Floor => ' ',
                    &Tile::CrateDst => '@',
                })?;
            }
        }
        writeln!(f, "")
    }
}

pub struct Game {
    room: Arc<Room>,
    crates_pos_buf: HashSet<Arc<Vec<Coords>>>,
}

impl Game {
    pub fn new(room: Room) -> Game {
        Game {
            room: Arc::new(room),
            crates_pos_buf: HashSet::new(),
        }
    }

    pub fn make_placement(&mut self, player: Coords, crates: &Vec<Coords>) -> Placement {
        let (buffered_crates_pos, insert_p) =
            if let Some(bc) = self.crates_pos_buf.get(crates) {
                (bc.clone(), false)
            } else {
                let mut crates_clone = crates.clone();
                crates_clone.sort();
                let bc = Arc::new(crates_clone);
                (bc, true)
            };
        if insert_p {
            self.crates_pos_buf.insert(buffered_crates_pos.clone());
        }
        Placement {
            player: player,
            crates: buffered_crates_pos,
        }
    }

    pub fn make_game_state(&self, placement: Placement) -> GameState {
        GameState {
            room: self.room.clone(),
            placement: placement,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Move {
    North,
    East,
    South,
    West,
}

impl GameState {
    pub fn room_at(&self, coord: &Coords) -> Option<&Tile> {
        let width = self.room.width as isize;
        let height = self.room.height as isize;
        if (coord.0 < 0) || (coord.0 >= height) || (coord.1 < 0) || (coord.1 >= width) {
            None
        } else {
            let index = coord.0 * width + coord.1;
            Some(&self.room.content[index as usize])
        }
    }

    pub fn crate_at(&self, coord: &Coords) -> Option<usize> {
        self.placement
            .crates
            .iter()
            .position(|c| c == coord)
    }

    pub fn finished(&self) -> bool {
        self.placement
            .crates
            .iter()
            .all(|c| if let Some(&Tile::CrateDst) = self.room_at(c) {
                true
            } else {
                false
            })
    }

    pub fn transitions<'a, 'b>(&'a self, game: &'b mut Game) -> Transitions<'a, 'b> {
        Transitions {
            state: self,
            game: game,
            crates_pos: Vec::new(),
            counter: 0,
        }
    }

    pub fn has_blocked_crate(&self) -> bool {
        for coord in self.placement.crates.iter() {
            if let Some(&Tile::CrateDst) = self.room_at(coord) {
                continue;
            }
            let blocked = |c| if let Some(&Tile::Wall) = self.room_at(&c) {
                true
            } else {
                self.placement.crates.iter().any(|&cc| cc == c)
            };
            if (blocked((coord.0 - 1, coord.1)) && blocked((coord.0, coord.1 + 1)))
                || (blocked((coord.0, coord.1 + 1)) && blocked((coord.0 + 1, coord.1)))
                || (blocked((coord.0 + 1, coord.1)) && blocked((coord.0, coord.1 - 1)))
                || (blocked((coord.0, coord.1 - 1)) && blocked((coord.0 - 1, coord.1)))
            {
                return true;
            }
        }
        false
    }

}

pub struct Transitions<'a, 'b> {
    state: &'a GameState,
    game: &'b mut Game,
    crates_pos: Vec<Coords>,
    counter: usize,
}

impl<'a, 'b> Iterator for Transitions<'a, 'b> {
    type Item = (Move, GameState);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (move_, rd, cd, rdd, cdd) =
                match self.counter {
                    0 => (Move::North, -1, 0, -2, 0),
                    1 => (Move::East, 0, 1, 0, 2),
                    2 => (Move::South, 1, 0, 2, 0),
                    3 => (Move::West, 0, -1, 0, -2),
                    _ => return None,
                };
            self.counter += 1;
            let (player_row, player_col) = self.state.placement.player;
            let near_coord =
                (player_row + rd, player_col + cd);
            let far_coord =
                (player_row + rdd, player_col + cdd);
            let crates = &*self.state.placement.crates;
            if let Some(crate_index) = self.state.crate_at(&near_coord) {
                if let None = self.state.crate_at(&far_coord) {
                    let tile_after = self.state.room_at(&far_coord);
                    if tile_after.map(|t| [Tile::Floor, Tile::CrateDst].contains(t)).unwrap_or(false) {
                        self.crates_pos.clear();
                        let coords_iter = crates
                            .iter()
                            .enumerate()
                            .map(|(i, &coord)| if crate_index == i {
                                far_coord
                            } else {
                                coord
                            });
                        self.crates_pos.extend(coords_iter);
                        let placement =
                            self.game.make_placement(near_coord, &self.crates_pos);
                        return Some((move_, self.game.make_game_state(placement)));
                    }
                }
            } else if self.state.room_at(&near_coord).map(|t| [Tile::Floor, Tile::CrateDst].contains(t)).unwrap_or(false) {
                let placement = self.game.make_placement(near_coord, crates);
                return Some((move_, self.game.make_game_state(placement)));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use super::{Move, GameState};
    use super::super::parser;

    #[test]
    fn move_north() {
        let (move_, state) = one_step(" \nI\n*\n");
        assert_eq!(move_, Move::North);
        assert_eq!(state.placement.player, (0, 0));
    }

    #[test]
    fn move_east() {
        let (move_, state) = one_step("*I ");
        assert_eq!(move_, Move::East);
        assert_eq!(state.placement.player, (0, 2));
    }

    #[test]
    fn move_south() {
        let (move_, state) = one_step("*\nI\n ");
        assert_eq!(move_, Move::South);
        assert_eq!(state.placement.player, (2, 0));
    }

    #[test]
    fn move_west() {
        let (move_, state) = one_step(" I*");
        assert_eq!(move_, Move::West);
        assert_eq!(state.placement.player, (0, 0));
    }

    #[test]
    fn move_north_dst() {
        let (move_, state) = one_step("+\n@\nI\n*\n");
        assert_eq!(move_, Move::North);
        assert_eq!(state.placement.player, (1, 0));
    }

    #[test]
    fn move_east_dst() {
        let (move_, state) = one_step("*I@+");
        assert_eq!(move_, Move::East);
        assert_eq!(state.placement.player, (0, 2));
    }

    #[test]
    fn move_south_dst() {
        let (move_, state) = one_step("*\nI\n@\n+\n");
        assert_eq!(move_, Move::South);
        assert_eq!(state.placement.player, (2, 0));
    }

    #[test]
    fn move_west_dst() {
        let (move_, state) = one_step("+@I*");
        assert_eq!(move_, Move::West);
        assert_eq!(state.placement.player, (0, 1));
    }

    #[test]
    fn move_crate_north() {
        let (move_, state) = one_step("@\n \n+\nI\n");
        assert_eq!(move_, Move::North);
        assert_eq!(state.placement.player, (2, 0));
        assert_eq!(state.placement.crates, Arc::new(vec![(1, 0)]));
        assert!(!state.finished());
    }

    #[test]
    fn move_crate_east() {
        let (move_, state) = one_step("I+ @");
        assert_eq!(move_, Move::East);
        assert_eq!(state.placement.player, (0, 1));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 2)]));
        assert!(!state.finished());
    }

    #[test]
    fn move_crate_south() {
        let (move_, state) = one_step("I\n+\n \n@\n");
        assert_eq!(move_, Move::South);
        assert_eq!(state.placement.player, (1, 0));
        assert_eq!(state.placement.crates, Arc::new(vec![(2, 0)]));
        assert!(!state.finished());
    }

    #[test]
    fn move_crate_west() {
        let (move_, state) = one_step("@ +I");
        assert_eq!(move_, Move::West);
        assert_eq!(state.placement.player, (0, 2));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 1)]));
        assert!(!state.finished());
    }

    #[test]
    fn move_crate_north_finish() {
        let (move_, state) = one_step("@\n+\nI\n*\n");
        assert_eq!(move_, Move::North);
        assert_eq!(state.placement.player, (1, 0));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 0), (3, 0)]));
        assert!(state.finished());
    }

    #[test]
    fn move_crate_east_finish() {
        let (move_, state) = one_step("*I+@");
        assert_eq!(move_, Move::East);
        assert_eq!(state.placement.player, (0, 2));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 0), (0, 3)]));
        assert!(state.finished());
    }

    #[test]
    fn move_crate_south_finish() {
        let (move_, state) = one_step("*\nI\n+\n@");
        assert_eq!(move_, Move::South);
        assert_eq!(state.placement.player, (2, 0));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 0), (3, 0)]));
        assert!(state.finished());
    }

    #[test]
    fn move_crate_west_finish() {
        let (move_, state) = one_step("@+I*");
        assert_eq!(move_, Move::West);
        assert_eq!(state.placement.player, (0, 1));
        assert_eq!(state.placement.crates, Arc::new(vec![(0, 0), (0, 3)]));
        assert!(state.finished());
    }

    #[test]
    fn no_move_north() {
        let room_txt = "@\n#\n+\nI\n";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_east() {
        let room_txt = "I+#@";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_south() {
        let room_txt = "I\n+\n#\n@\n";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_west() {
        let room_txt = "@#+I";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_north2() {
        let room_txt = "@\n@\n+\n+\nI\n";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_east2() {
        let room_txt = "I++@@";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_south2() {
        let room_txt = "I\n+\n+\n@\n@\n";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    #[test]
    fn no_move_west2() {
        let room_txt = "@@++I";
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        assert!(init_state.transitions(&mut game).next().is_none());
    }

    fn one_step(room_txt: &'static str) -> (Move, GameState) {
        let (mut game, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        init_state.transitions(&mut game).next().unwrap()
    }

    #[test]
    fn blocked_crates() {
        assert!(room_state("####\n#+I@").has_blocked_crate());
        assert!(room_state("####\n@I+#").has_blocked_crate());
        assert!(room_state("#+I@\n####").has_blocked_crate());
        assert!(room_state("@I+#\n####").has_blocked_crate());
        assert!(room_state("###@\n++I@").has_blocked_crate());
        assert!(room_state("###@\n@I++").has_blocked_crate());
        assert!(room_state("++I@\n###@").has_blocked_crate());
        assert!(room_state("@I++\n###@").has_blocked_crate());
    }

    fn room_state(room_txt: &'static str) -> GameState {
        let (_, init_state) = parser::parse(room_txt.as_bytes()).unwrap();
        init_state
    }
}
