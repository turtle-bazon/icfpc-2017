use std::fmt;
use std::rc::Rc;
use std::collections::HashSet;
use super::map::{Tile, Room};

pub type Coords = (isize, isize);

#[derive(Hash)]
pub struct Placement {
    pub player: Coords,
    pub crates: Rc<Vec<Coords>>,
}

pub struct GameState {
    pub room: Rc<Room>,
    pub placement: Placement,
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Room {}x{}, {} crates",
               self.room.width,
               self.room.height,
               self.room.crates_count)?;
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
    room: Rc<Room>,
    crates_pos_buf: HashSet<Rc<Vec<Coords>>>,
}

#[derive(Debug)]
pub enum Error {
}

impl Game {
    pub fn new(room: Room) -> Game {
        Game {
            room: Rc::new(room),
            crates_pos_buf: HashSet::new(),
        }
    }

    pub fn make_placement(&mut self, player: Coords, crates: &Vec<Coords>) -> Placement {
        let (buffered_crates_pos, insert_p) =
            if let Some(bc) = self.crates_pos_buf.get(crates) {
                (bc.clone(), false)
            } else {
                let bc = Rc::new(crates.clone());
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

impl GameState {
    pub fn room_at(&self, coord: Coords) -> Option<&Tile> {
        let width = self.room.width as isize;
        let height = self.room.height as isize;
        if (coord.0 < 0) || (coord.0 >= height) || (coord.1 < 0) || (coord.1 >= width) {
            None
        } else {
            let index = coord.0 * width + coord.1;
            Some(&self.room.content[index as usize])
        }
    }

    pub fn crate_at(&self, coord: Coords, crates: &[Coords]) -> Option<usize> {
        crates.iter().position(|c| c == &coord)
    }


}
