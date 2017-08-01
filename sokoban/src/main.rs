extern crate env_logger;
#[macro_use] extern crate log;
#[macro_use] extern crate nom;
#[macro_use] extern crate clap;

use std::{io, fs, process};
use std::io::Read;
use clap::Arg;

mod map;
mod parser;

fn main() {
    env_logger::init().unwrap();
    match run() {
        Ok(()) =>
            info!("graceful shutdown"),
        Err(e) => {
            error!("fatal error: {:?}", e);
            process::exit(1);
        },
    }
}

#[derive(Debug)]
enum Error {
    MissingParameter(&'static str),
    RoomFileOpen(io::Error),
    RoomFileRead(io::Error),
    RoomParse(parser::Error),
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("room-file")
             .display_order(1)
             .short("f")
             .long("room-file")
             .value_name("FILER")
             .help("Input room map file")
             .takes_value(true))
        .get_matches();

    let map_file = matches.value_of("room-file")
        .ok_or(Error::MissingParameter("room-file"))?;

    let mut file = fs::File::open(map_file)
        .map_err(Error::RoomFileOpen)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(Error::RoomFileRead)?;

    let parsed_room = parser::parse_map(contents.as_bytes())
        .map_err(Error::RoomParse)?;

    println!("Parsed room: {:?}", parsed_room);
    Ok(())
}
