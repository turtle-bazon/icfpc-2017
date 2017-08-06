extern crate env_logger;
extern crate lambda_punter;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::process;
use clap::{Arg, SubCommand};
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
    AlwaysPassSolver(client::Error<()>),
    NearestSolver(client::Error<()>),
    LinkMinesSolver(client::Error<()>),
    GNSolver(client::Error<()>),
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("hello-name")
             .display_order(1)
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
        .subcommand(SubCommand::with_name("gn")
                    .display_order(4)
                    .about("solvers::gn"))
        .get_matches();

    let hello_name = matches.value_of("hello-name")
        .ok_or(Error::MissingParameter("hello-name"))?;

    info!("initializing as [ {} ]", hello_name);
    if let Some(..) = matches.subcommand_matches("always_pass") {
        debug!("using solvers::always_pass");
        proceed_with_solver(hello_name, solvers::always_pass::AlwaysPassGameStateBuilder, Error::AlwaysPassSolver)
    } else if let Some(..) = matches.subcommand_matches("nearest") {
        debug!("using solvers::nearest");
        proceed_with_solver(hello_name, solvers::nearest::NearestGameStateBuilder, Error::NearestSolver)
    } else if let Some(..) = matches.subcommand_matches("link_mines") {
        debug!("using solvers::link_mines");
        proceed_with_solver(hello_name, solvers::link_mines::LinkMinesGameStateBuilder, Error::LinkMinesSolver)
    } else if let Some(..) = matches.subcommand_matches("gn") {
        debug!("using solvers::gn");
        proceed_with_solver(hello_name, solvers::gn::GNGameStateBuilder, Error::GNSolver)
    } else {
        debug!("using solvers::link_mines");
        proceed_with_solver(hello_name, solvers::link_mines::LinkMinesGameStateBuilder, Error::LinkMinesSolver)
    }
}

fn proceed_with_solver<GB, EF>(
    hello_name: &str,
    gs_builder: GB,
    err_map: EF)
    -> Result<(), Error>
    where GB: game::GameStateBuilder,
          EF: Fn(client::Error<<GB::GameState as game::GameState>::Error>) -> Error
{
    let maybe_results = client::run_offline(hello_name, gs_builder)
        .map_err(err_map)?;
    info!("all done");

    if let Some((scores, game_state)) = maybe_results {
        let my_punter = game_state.get_punter();
        info!("Game over! Total server scores:");
        for score in scores {
            info!("  Punter: {}{}, score: {}",
                  score.punter,
                  if score.punter == my_punter { " (it's me)" } else { "" },
                  score.score);
        }
    }

    Ok(())
}
