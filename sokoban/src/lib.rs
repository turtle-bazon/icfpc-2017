#[macro_use] extern crate log;
#[macro_use] extern crate nom;

use std::{io, fs};
use std::io::Read;
use std::path::Path;

pub mod map;
pub mod game;
pub mod parser;
pub mod solver;

#[derive(Debug)]
pub enum Error {
    MissingParameter(&'static str),
    RoomFileOpen(io::Error),
    RoomFileRead(io::Error),
    RoomParse(parser::Error),
}

pub fn init_room<P>(room_file: P) -> Result<(game::Game, game::GameState), Error> where P: AsRef<Path> {
    let mut file = fs::File::open(room_file)
        .map_err(Error::RoomFileOpen)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(Error::RoomFileRead)?;

    parser::parse(contents.as_bytes())
        .map_err(Error::RoomParse)
}
