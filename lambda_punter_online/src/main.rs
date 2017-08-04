extern crate env_logger;
extern crate lambda_punter;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::process;
use clap::Arg;

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
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("server-host")
             .display_order(1)
             .short("h")
             .long("server-host")
             .value_name("HOST")
             .help("server tcp connect host")
             .default_value("punter.inf.ed.ac.uk")
             .takes_value(true))
        .arg(Arg::with_name("server-port")
             .display_order(2)
             .short("p")
             .long("server-port")
             .value_name("PORT")
             .help("server tcp connect port")
             .default_value("9001")
             .takes_value(true))
        .get_matches();

    let server_host = matches.value_of("server-host")
        .ok_or(Error::MissingParameter("server-host"))?;
    let server_port = matches.value_of("server-port")
        .ok_or(Error::MissingParameter("server-port"))?;

    Ok(())
}
