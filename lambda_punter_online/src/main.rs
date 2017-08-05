extern crate env_logger;
extern crate lambda_punter;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::process;
use clap::{Arg, AppSettings, SubCommand};
use lambda_punter::{client, game, solvers};
use lambda_punter::game::GameState;

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
    InvalidServerPort(clap::Error),
    NoSubcommandProvided,
    AlwaysPassSolver(client::Error<()>),
    NearestSolver(client::Error<()>),
    LinkMinesSolver(client::Error<()>),
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .setting(AppSettings::SubcommandRequired)
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
        .arg(Arg::with_name("hello-name")
             .display_order(3)
             .short("n")
             .long("hello-name")
             .value_name("NAME")
             .help("welcome name for Handshake packet")
             .default_value("skobochka")
             .takes_value(true))
        .subcommand(SubCommand::with_name("always_pass")
                    .display_order(1)
                    .about("solvers::always_pass"))
        .subcommand(SubCommand::with_name("nearest")
                    .display_order(2)
                    .about("solvers::nearest"))
        .subcommand(SubCommand::with_name("link_mines")
                    .display_order(3)
                    .about("solvers::link_mines"))
        .get_matches();

    let server_host = matches.value_of("server-host")
        .ok_or(Error::MissingParameter("server-host"))?;
    let server_port = value_t!(matches, "server-port", u16)
        .map_err(Error::InvalidServerPort)?;
    let hello_name = matches.value_of("hello-name")
        .ok_or(Error::MissingParameter("hello-name"))?;

    info!("connecting to {}:{} as [ {} ]", server_host, server_port, hello_name);
    if let Some(..) = matches.subcommand_matches("always_pass") {
        debug!("using solvers::always_pass");
        proceed_with_solver(server_host, server_port, hello_name, solvers::always_pass::AlwaysPassGameStateBuilder, Error::AlwaysPassSolver)
    } else if let Some(..) = matches.subcommand_matches("nearest") {
        debug!("using solvers::nearest");
        proceed_with_solver(server_host, server_port, hello_name, solvers::nearest::NearestGameStateBuilder, Error::NearestSolver)
    } else if let Some(..) = matches.subcommand_matches("link_mines") {
        debug!("using solvers::link_mines");
        proceed_with_solver(server_host, server_port, hello_name, solvers::link_mines::LinkMinesGameStateBuilder, Error::LinkMinesSolver)
    } else {
        Err(Error::NoSubcommandProvided)
    }
}

fn proceed_with_solver<GB, EF>(
    server_host: &str,
    server_port: u16,
    hello_name: &str,
    gs_builder: GB,
    err_map: EF)
    -> Result<(), Error>
    where GB: game::GameStateBuilder,
          EF: Fn(client::Error<<GB::GameState as GameState>::Error>) -> Error
{
    let (scores, game_state) = client::run_network((server_host, server_port), hello_name, gs_builder)
        .map_err(err_map)?;
    info!("all done");

    println!("Game over! Total server scores:");
    for score in scores {
        println!("  Punter: {}{}, score: {}",
                 score.punter,
                 if game_state.get_punter() == score.punter { " (it's me)" } else { "" },
                 score.score);
    }

    Ok(())
}
