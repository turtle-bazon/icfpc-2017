enum Map {
    Wall,
    Floor
}

struct Room {
    width: usize,
    height: usize,
    entry: Vec<Map>,
}
