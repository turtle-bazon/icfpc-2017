use std::fmt;

#[derive(Debug)]
pub enum Tile {
    Wall,
    Floor,
    CrateDst,
}

pub struct Room {
    pub width: usize,
    pub height: usize,
    pub content: Vec<Tile>,
}

impl fmt::Debug for Room {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Room {}x{}", self.width, self.height)?;
        for (coord, tile) in self.content.iter().enumerate() {
            if coord % self.width == 0 {
                writeln!(f, "")?;
            }
            write!(f, "{}", match tile {
                &Tile::Wall => '#',
                &Tile::Floor => ' ',
                &Tile::CrateDst => '@',
            })?;
        }
        writeln!(f, "")
    }
}
