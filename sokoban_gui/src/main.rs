extern crate sokoban;
extern crate env_logger;
extern crate piston_window;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::{io, fs, process};
use std::io::Read;
use clap::Arg;
use piston_window::{PistonWindow, WindowSettings};

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
    Sokoban(sokoban::Error),
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

    Ok(())
}
