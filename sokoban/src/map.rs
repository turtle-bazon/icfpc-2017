pub type Coords = (isize, isize);

#[derive(Debug)]
pub enum Tile {
    Wall,
    Floor,
    CrateDst,
}

#[derive(Debug)]
pub struct Room {
    pub width: usize,
    pub height: usize,
    pub crates_dsts: Vec<Coords>,
    pub content: Vec<Tile>,
}
