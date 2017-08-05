use std::{io, net, str, iter, num};
use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use super::game::{GameStateBuilder, GameState};
use super::proto::{self, Score, Req, Rep};
use super::chat;

type ChatError<GE> = chat::Error<SendError, RecvError, GE>;

#[derive(Debug)]
pub enum Error<GE> {
    Chat(ChatError<GE>),
    Connect(io::Error),
}

#[derive(Debug)]
pub enum SendError {
    WriteLen(io::Error),
    WritePacket(io::Error),
    PacketEncode(proto::Error),
}

#[derive(Debug)]
pub enum RecvError {
    ReadLen(io::Error),
    ReadLenTooBig(usize),
    ReadUnexpectedClose,
    LenEmpty,
    LenString(str::Utf8Error),
    LenParse(num::ParseIntError),
    ReadPacket(io::Error),
    ReadPacketNotEnough { want_bytes: usize, received_bytes: usize, },
    PacketString(str::Utf8Error),
    PacketDecode(proto::Error),
    UnexpectedStateArrived,
}

pub fn run_network<A, GB>(addr: A, name: &str, gs_builder: GB) -> Result<(Vec<Score>, GB::GameState), Error<<GB::GameState as GameState>::Error>>
    where A: ToSocketAddrs,
          GB: GameStateBuilder,
{
    let tcp = net::TcpStream::connect(addr)
        .map_err(Error::Connect)?;

    chat::run_online(name, tcp, generic_write, generic_read, gs_builder)
        .map_err(Error::Chat)
}

pub fn run_offline<GB>(name: &str, gs_builder: GB) -> Result<Option<(Vec<Score>, GB::GameState)>, Error<<GB::GameState as GameState>::Error>>
    where GB: GameStateBuilder,
{
    struct Stdio;

    impl Read for Stdio {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let s = io::stdin();
            let mut l = s.lock();
            l.read(buf)
        }

        fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
            let s = io::stdin();
            let mut l = s.lock();
            l.read_to_end(buf)
        }
    }

    impl Write for Stdio {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let s = io::stdout();
            let mut l = s.lock();
            let b = l.write(buf)?;
            l.flush()?;
            Ok(b)
        }

        fn flush(&mut self) -> io::Result<()> {
            let s = io::stdout();
            let mut l = s.lock();
            l.flush()
        }
    }

    chat::run_offline(name, Stdio, generic_write, generic_read, gs_builder)
        .map_err(Error::Chat)
}

fn generic_write<W, S>(writer: &mut W, req: Req, maybe_state: Option<S>) -> Result<(), SendError>
    where W: Write, S: Serialize
{
    let encoded_req = req.to_json(maybe_state)
        .map_err(SendError::PacketEncode)?;
    let length_req = format!("{}:", encoded_req.as_bytes().len());
    debug!("P -> S | {}{}", length_req, encoded_req);
    writer.write_all(length_req.as_bytes())
        .map_err(SendError::WriteLen)?;
    writer.write_all(encoded_req.as_bytes())
        .map_err(SendError::WritePacket)
}

fn generic_read<R, S>(reader: &mut R) -> Result<(Rep, Option<S>), RecvError>
    where R: Read, S: DeserializeOwned
{
    let mut packet = Vec::with_capacity(9);
    loop {
        let mut byte = [0; 1];
        match reader.read(&mut byte) {
            Ok(0) =>
                return Err(RecvError::ReadUnexpectedClose),
            Ok(1) if byte[0] == b':' =>
                break,
            Ok(1) =>
                packet.push(byte[0]),
            Ok(_) =>
                unreachable!(),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted =>
                continue,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
                continue,
            Err(e) =>
                return Err(RecvError::ReadLen(e)),
        }
    }
    if packet.is_empty() {
        Err(RecvError::LenEmpty)
    } else {
        let len: usize = str::from_utf8(&packet)
            .map_err(RecvError::LenString)?
        .parse()
            .map_err(RecvError::LenParse)?;
        packet.clear();
        packet.extend(iter::repeat(0).take(len));
        let received_bytes = {
            let mut buf: &mut [_] = &mut packet;
            while !buf.is_empty() {
                match reader.read(buf) {
                    Ok(0) =>
                        break,
                    Ok(n) => {
                        let tmp = buf;
                        buf = &mut tmp[n..];
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted =>
                        (),
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
                        (),
                    Err(e) =>
                        return Err(RecvError::ReadPacket(e)),
                }
            }
            len - buf.len()
        };
        if received_bytes != len {
            Err(RecvError::ReadPacketNotEnough {
                want_bytes: len,
                received_bytes: received_bytes,
            })
        } else {
            let packet_str = str::from_utf8(&packet)
                .map_err(RecvError::PacketString)?;
            debug!("S -> P | {}:{}", len, packet_str);
            let (rep, maybe_state) = Rep::from_json(&packet_str)
                .map_err(RecvError::PacketDecode)?;
            Ok((rep, maybe_state))
        }
    }
}
