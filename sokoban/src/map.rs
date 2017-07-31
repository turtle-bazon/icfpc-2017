#[derive (Debug)]
pub enum Tile {
    Wall,
    Floor,
}

#[derive (Debug)]
pub struct Room {
    pub width: usize,
    pub height: usize,
    pub content: Vec<Tile>,
}
