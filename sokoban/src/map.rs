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
    pub crates_count: usize,
    pub content: Vec<Tile>,
}
