
use super::proto::{Req, Rep, Score};
use super::game::GameState;

#[derive(PartialEq, Debug)]
pub enum Error<SR, RR> {
    Send(SR),
    Recv(RR),
    UnexpectedHandshakeRep(Rep),
    UnexpectedSetupRep(Rep),
    UnexpectedMoveRep(Rep),
}

pub fn run_online<FS, SR, FR, RR>(name: &str, mut send_fn: FS, mut recv_fn: FR) -> Result<Vec<Score>, Error<SR, RR>>
    where FS: FnMut(Req) -> Result<(), SR>,
          FR: FnMut() -> Result<Rep, RR>,
{
    // P → S {"me" : name}
    send_fn(Req::Handshake { name: name.to_string(), })
        .map_err(Error::Send)?;
    // S → P {"you" : name}
    match recv_fn().map_err(Error::Recv)? {
        Rep::Handshake { name: ref rep_name, } if rep_name == name =>
            (),
        other =>
            return Err(Error::UnexpectedHandshakeRep(other)),
    }
    // S → P {"punter" : p, "punters" : n, "map" : map}
    let mut game_state =
        match recv_fn().map_err(Error::Recv)? {
            Rep::Setup(setup) =>
                GameState::new(setup),
            other =>
                return Err(Error::UnexpectedSetupRep(other)),
        };
    // P → S {"ready" : p}
    send_fn(Req::Ready { punter: game_state.punter, })
        .map_err(Error::Send)?;
    // gameplay
    loop {
        // S → P {"move" : {"moves" : moves}}
        // S → P {"stop" : {"moves" : moves,"scores" : scores}}
        match recv_fn().map_err(Error::Recv)? {
            Rep::Move { moves, } => {
                let (move_, next_game_state) = game_state.play(moves);
                game_state = next_game_state;
                send_fn(Req::Move(move_)).map_err(Error::Send)?;
            },
            Rep::Stop { scores, .. } =>
                return Ok(scores),
            other =>
                return Err(Error::UnexpectedMoveRep(other)),
        }

    }
}

#[cfg(test)]
mod test {
    use super::{Error, run_online};
    #[allow(unused_imports)]
    use super::super::proto::{Req, Rep};

    #[test]
    fn handshake_err() {
        assert_eq!(
            run_online(
                "alice",
                |_req| Ok::<_, ()>(()),
                || Ok::<_, ()>(Rep::Handshake { name: "bob".to_string(), })),
            Err(Error::UnexpectedHandshakeRep(Rep::Handshake { name: "bob".to_string(), })));
    }
}
