#[derive (Debug)]
pub enum Map {
    Wall,
    Floor,
}

#[derive (Debug)]
pub struct Room {
    pub width: usize,
    pub height: usize,
    pub content: Vec<Map>,
}
