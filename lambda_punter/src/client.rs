use std::{io, net};
use std::net::ToSocketAddrs;
use super::game::GameStateBuilder;
use super::proto::Score;
use super::chat;

#[derive(Debug)]
pub enum Error {
    Chat,
    Connect(io::Error),
}

pub fn run_network<A, GB>(addr: A, gs_builder: GB) -> Result<(Vec<Score>, GB::GameState), Error>
    where A: ToSocketAddrs,
          GB: GameStateBuilder,
{
    let tcp = net::TcpStream::connect(addr)
        .map_err(Error::Connect)?;


    unimplemented!()
}
