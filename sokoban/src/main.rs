#[macro_use] extern crate nom;

mod map;
mod parser;

use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let map_file = &args[1];

        let mut file = File::open (map_file).expect("File not found");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("something went wrong reading the file");

        println!("Result: {:?}", parser::parse_map(contents.as_bytes()));
    } else {
        panic!("args");
    }
}
