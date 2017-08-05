extern crate rand;
extern crate env_logger;
extern crate lambda_punter;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::{io, thread, process};
use std::sync::mpsc;

use rand::Rng;
use clap::{Arg, AppSettings, SubCommand};
use lambda_punter::{client, game, solvers, proto};
use lambda_punter::types::PunterId;
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
    InvalidSlavesCount(clap::Error),
    InvalidGamesCount(clap::Error),
    NoSubcommandProvided,
    AlwaysPassSolver(client::Error<()>),
    NearestSolver(client::Error<()>),
    LinkMinesSolver(client::Error<()>),
    GameThreadSpawn(io::Error),
    GameThreadJoin(Box<std::any::Any + Send + 'static>),
}

#[derive(Clone, Copy)]
enum Solver {
    AlwaysPass,
    Nearest,
    LinkMines,
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
        .arg(Arg::with_name("server-start-port")
             .display_order(2)
             .short("s")
             .long("server-start-port")
             .value_name("PORT")
             .help("server tcp connect port range (left bound)")
             .default_value("9001")
             .takes_value(true))
        .arg(Arg::with_name("server-end-port")
             .display_order(3)
             .short("e")
             .long("server-end-port")
             .value_name("PORT")
             .help("server tcp connect port range (right bound)")
             .default_value("9001")
             .takes_value(true))
        .arg(Arg::with_name("parallel-slaves-count")
             .display_order(4)
             .short("w")
             .long("parallel-slaves-count")
             .value_name("COUNT")
             .help("parallel games count")
             .default_value("1")
             .takes_value(true))
        .arg(Arg::with_name("total-games-count")
             .display_order(5)
             .short("t")
             .long("total-games-count")
             .value_name("COUNT")
             .help("total successfull games expected")
             .default_value("1")
             .takes_value(true))
        .arg(Arg::with_name("hello-name")
             .display_order(6)
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
    let server_start_port = value_t!(matches, "server-start-port", u16)
        .map_err(Error::InvalidServerPort)?;
    let server_end_port = value_t!(matches, "server-end-port", u16)
        .map_err(Error::InvalidServerPort)?;
    let slaves_count = value_t!(matches, "parallel-slaves-count", usize)
        .map_err(Error::InvalidSlavesCount)?;
    let total_games = value_t!(matches, "total-games-count", usize)
        .map_err(Error::InvalidGamesCount)?;
    let hello_name = matches.value_of("hello-name")
        .ok_or(Error::MissingParameter("hello-name"))?;

    let solver =
        if let Some(..) = matches.subcommand_matches("always_pass") {
            debug!("using solvers::always_pass");
            Solver::AlwaysPass
        } else if let Some(..) = matches.subcommand_matches("nearest") {
            debug!("using solvers::nearest");
            Solver::Nearest
        } else if let Some(..) = matches.subcommand_matches("link_mines") {
            debug!("using solvers::link_mines");
            Solver::LinkMines
        } else {
            return Err(Error::NoSubcommandProvided);
        };

    let (tx, rx) = mpsc::channel();
    let mut games_played = 0;
    let mut slaves = Vec::new();
    let mut slave_id_counter = 0;
    let mut rng = rand::thread_rng();
    while games_played < total_games {
        while slaves.len() < slaves_count {
            let tx = tx.clone();
            let server_host = server_host.to_string();
            let server_port = rng.gen_range(server_start_port, server_end_port + 1);
            let hello_name = hello_name.to_string();
            slave_id_counter += 1;
            debug!("running slave {} for game on port {}", slave_id_counter, server_port);
            let slave = thread::Builder::new()
                .name(format!("game slave {}", slaves.len()))
                .spawn(move || {
                    tx.send(match solver {
                        Solver::AlwaysPass =>
                            proceed_with_solver(
                                slave_id_counter,
                                &server_host,
                                server_port,
                                &hello_name,
                                solvers::always_pass::AlwaysPassGameStateBuilder,
                                Error::AlwaysPassSolver),
                        Solver::Nearest =>
                            proceed_with_solver(
                                slave_id_counter,
                                &server_host,
                                server_port,
                                &hello_name,
                                solvers::nearest::NearestGameStateBuilder,
                                Error::NearestSolver),
                        Solver::LinkMines =>
                            proceed_with_solver(
                                slave_id_counter,
                                &server_host,
                                server_port,
                                &hello_name,
                                solvers::link_mines::LinkMinesGameStateBuilder,
                                Error::LinkMinesSolver),
                    }).ok();
                })
                .map_err(Error::GameThreadSpawn)?;
            slaves.push((slave_id_counter, slave));
        }

        let slave_id =
            match rx.recv().unwrap() {
                Ok((slave_id, port, my_punter, scores)) => {
                    println!("SUCCESS for game port {}:", port);
                    for score in scores {
                        println!("  Punter: {}{}, score: {}",
                                 score.punter,
                                 if score.punter == my_punter { " (it's me)" } else { "" },
                                 score.score);
                    }
                    games_played += 1;
                    slave_id
                },
                Err((slave_id, port, err)) => {
                    println!("ERROR for game port {}: {:?}", port, err);
                    slave_id
                },
            };
        let slave_i = slaves.iter().position(|s| s.0 == slave_id).unwrap();
        let (_, slave) = slaves.swap_remove(slave_i);
        let () = slave.join().map_err(Error::GameThreadJoin)?;
    }

    Ok(())
}

fn proceed_with_solver<GB, EF>(
    slave_id: usize,
    server_host: &str,
    server_port: u16,
    hello_name: &str,
    gs_builder: GB,
    err_map: EF)
    -> Result<(usize, u16, PunterId, Vec<proto::Score>), (usize, u16, Error)>
    where GB: game::GameStateBuilder,
          EF: Fn(client::Error<<GB::GameState as GameState>::Error>) -> Error
{
    info!("playing game on {}:{} as {} (slave {}) ", server_host, server_port, hello_name, slave_id);
    let (scores, game_state) = client::run_network((server_host, server_port), hello_name, gs_builder)
        .map_err(err_map)
        .map_err(|e| (slave_id, server_port, e))?;
    Ok((slave_id, server_port, game_state.get_punter(), scores))
}
