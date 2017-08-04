use std::{io, net, str, iter, num};
use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use super::game::{GameStateBuilder, GameState};
use super::proto::{self, Score, Rep};
use super::chat;

type ChatError<GE> = chat::Error<SendError, RecvError, GE>;

#[derive(Debug)]
pub enum Error<GE> {
    Chat(ChatError<GE>),
    Connect(io::Error),
}

#[derive(Debug)]
pub enum SendError {
    TcpWrite(io::Error),
}

#[derive(Debug)]
pub enum RecvError {
    TcpReadLen(io::Error),
    TcpReadLenTooBig(usize),
    TcpReadUnexpectedClose,
    TcpLenEmpty,
    TcpLenString(str::Utf8Error),
    TcpLenParse(num::ParseIntError),
    TcpReadPacket(io::Error),
    TcpReadPacketNotEnough { want_bytes: usize, received_bytes: usize, },
    TcpPacketString(str::Utf8Error),
    PacketDecode(proto::Error),
}

pub fn run_network<A, GB>(addr: A, name: &str, gs_builder: GB) -> Result<(Vec<Score>, GB::GameState), Error<<GB::GameState as GameState>::Error>>
    where A: ToSocketAddrs,
          GB: GameStateBuilder,
{
    let tcp = net::TcpStream::connect(addr)
        .map_err(Error::Connect)?;

    chat::run_online(
        name,
        tcp,
        move |tcp, req| {
            let encoded_req = format!("{:?}", req);
            tcp.write_all(encoded_req.as_bytes())
                .map_err(SendError::TcpWrite)
        },
        |tcp| {
            let mut packet = Vec::with_capacity(8);
            loop {
                let mut byte = [0; 1];
                let bytes_read = tcp.read(&mut byte)
                    .map_err(RecvError::TcpReadLen)?;
                if bytes_read == 0 {
                    return Err(RecvError::TcpReadUnexpectedClose);
                } else if byte[0] == b':' {
                    break;
                } else {
                    packet.push(byte[0]);
                }
            }
            if packet.is_empty() {
                Err(RecvError::TcpLenEmpty)
            } else {
                let len: usize = str::from_utf8(&packet)
                    .map_err(RecvError::TcpLenString)?
                    .parse()
                    .map_err(RecvError::TcpLenParse)?;
                packet.clear();
                packet.extend(iter::repeat(0).take(len));
                let bytes_read = tcp.read(&mut packet)
                    .map_err(RecvError::TcpReadPacket)?;
                if bytes_read == 0 {
                    Err(RecvError::TcpReadUnexpectedClose)
                } else if bytes_read != len {
                    Err(RecvError::TcpReadPacketNotEnough {
                        want_bytes: len,
                        received_bytes: bytes_read,
                    })
                } else {
                    let packet_str = str::from_utf8(&packet)
                        .map_err(RecvError::TcpPacketString)?;
                    let rep = Rep::from_json(&packet_str)
                        .map_err(RecvError::PacketDecode)?;
                    Ok(rep)
                }
            }
        },
        gs_builder).map_err(Error::Chat)
}
