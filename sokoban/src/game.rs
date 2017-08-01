use std::fmt;
use super::map::{Tile, Room};

pub type Coords = (isize, isize);
pub type GameStateId = usize;

pub struct GameState<'a> {
    pub room: &'a Room,
    pub player: &'a Coords,
    pub crates: &'a [Coords],
}

impl<'a> fmt::Display for GameState<'a> {
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
            if self.player == &pos {
                write!(f, "I")?;
            } else if self.crates.iter().any(|&coord| coord == pos) {
                write!(f, "+")?;
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
    room: Room,
    coords_buf: Vec<Coords>,
}

#[derive(Debug)]
pub enum Error {
    InvalidGameStateId(GameStateId),
}

impl Game {
    pub fn new(room: Room) -> Game {
        Game {
            room: room,
            coords_buf: Vec::new(),
        }
    }

    pub fn add_state<I>(&mut self, player: Coords, crates: I) -> GameStateId
        where I: Iterator<Item = Coords>
    {
        let id = self.coords_buf.len();
        self.coords_buf.push(player);
        self.coords_buf.extend(crates);
        id
    }

    pub fn get_state<'a>(&'a self, state_id: GameStateId) -> Result<GameState<'a>, Error> {
        let state_len = 1 + self.room.crates_count;
        println!("{}", self.coords_buf.len());
        if state_id + state_len > self.coords_buf.len() {
            Err(Error::InvalidGameStateId(state_id))
        } else {
            Ok(GameState {
                room: &self.room,
                player: &self.coords_buf[state_id],
                crates: &self.coords_buf[state_id + 1 .. state_id + state_len],
            })
        }
    }
}

impl<'a> GameState<'a> {
    pub fn room_at(&self, coord: Coords) -> Option<&'a Tile> {
        let width = self.room.width as isize;
        let height = self.room.height as isize;
        if (coord.0 < 0) || (coord.0 >= height) || (coord.1 < 0) || (coord.1 >= width) {
            None
        } else {
            let index = coord.0 * width + coord.1;
            Some(&self.room.content[index as usize])
        }
    }


}
