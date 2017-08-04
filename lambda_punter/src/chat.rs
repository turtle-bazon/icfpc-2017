
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
    use super::super::map::{Map, Site, River};
    use super::super::proto::{Req, Rep, Move, Setup, Score};

    #[test]
    fn handshake_err() {
        assert_eq!(
            run_online(
                "alice",
                |_req| Ok::<_, ()>(()),
                || Ok::<_, ()>(Rep::Handshake { name: "bob".to_string(), })),
            Err(Error::UnexpectedHandshakeRep(Rep::Handshake { name: "bob".to_string(), })));
    }

    #[test]
    fn alice_script() {
        // -> {"me":"Alice"}
        // <- {"you":"Alice"}
        // <- {"punter":0, "punters":2,
        // "map":{"sites":[{"id":4},{"id":1},{"id":3},{"id":6},{"id":5},{"id":0},{"id":7},{"id":2}], "rivers":[{"source":3,"target":4},{"source":0,"target":1},{"source":2,"target":3}, {"source":1,"target":3},{"source":5,"target":6},{"source":4,"target":5}, {"source":3,"target":5},{"source":6,"target":7},{"source":5,"target":7}, {"source":1,"target":7},{"source":0,"target":7},{"source":1,"target":2}], "mines":[1,5]}}
        // -> {"ready":0}
        // <- {"move":{"moves":[{"pass":{"punter":0}},{"pass":{"punter":1}}]}}
        // -> {"claim":{"punter":0,"source":0,"target":1}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":0,"target":1}},{"claim":{"punter":1,"source":1,"target":2}}]}}
        // -> {"claim":{"punter":0,"source":2,"target":3}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":2,"target":3}},{"claim":{"punter":1,"source":3,"target":4}}]}}
        // -> {"claim":{"punter":0,"source":4,"target":5}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":4,"target":5}},{"claim":{"punter":1,"source":5,"target":6}}]}}
        // -> {"claim":{"punter":0,"source":6,"target":7}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":6,"target":7}},{"claim":{"punter":1,"source":7,"target":0}}]}}
        // -> {"claim":{"punter":0,"source":1,"target":3}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":1,"target":3}},{"claim":{"punter":1,"source":3,"target":5}}]}}
        // -> {"claim":{"punter":0,"source":5,"target":7}}
        // <- {"stop":{"moves":[{"claim":{"punter":0,"source":5,"target":7}},{"claim":{"punter":1,"source":7,"target":1}}], "scores":[{"punter":0,"score":6},{"punter":1,"score":6}]}}

        let mut map: Map = Default::default();
        map.sites.insert(4, Site { id: 4 });
        map.sites.insert(1, Site { id: 1 });
        map.sites.insert(3, Site { id: 3 });
        map.sites.insert(6, Site { id: 6 });
        map.sites.insert(5, Site { id: 5 });
        map.sites.insert(0, Site { id: 0 });
        map.sites.insert(7, Site { id: 7 });
        map.sites.insert(2, Site { id: 2 });
        map.rivers.insert(River { source: 3, target: 4, });
        map.rivers.insert(River { source: 0, target: 1, });
        map.rivers.insert(River { source: 2, target: 3, });
        map.rivers.insert(River { source: 1, target: 3, });
        map.rivers.insert(River { source: 5, target: 6, });
        map.rivers.insert(River { source: 4, target: 5, });
        map.rivers.insert(River { source: 3, target: 5, });
        map.rivers.insert(River { source: 6, target: 7, });
        map.rivers.insert(River { source: 5, target: 7, });
        map.rivers.insert(River { source: 1, target: 7, });
        map.rivers.insert(River { source: 0, target: 7, });
        map.rivers.insert(River { source: 1, target: 2, });
        map.mines.insert(1);
        map.mines.insert(5);

        let mut reqs = vec![
            Req::Handshake { name: "alice".to_string(), },
            Req::Ready { punter: 0, },
            Req::Move(Move::Claim { punter: 0, source: 0, target: 1, }),
            Req::Move(Move::Claim { punter: 0, source: 2, target: 3, }),
            Req::Move(Move::Claim { punter: 0, source: 4, target: 5, }),
            Req::Move(Move::Claim { punter: 0, source: 6, target: 7, }),
            Req::Move(Move::Claim { punter: 0, source: 1, target: 3, }),
            Req::Move(Move::Claim { punter: 0, source: 5, target: 7, }),
        ];
        reqs.reverse();

        let mut reps = vec![
            Rep::Handshake { name: "alice".to_string(), },
            Rep::Setup(Setup {
                punter: 0,
                punters: 2,
                map: map,
            }),
            Rep::Move { moves: vec![Move::Pass { punter: 0, }, Move::Pass { punter: 1, },] },
            Rep::Move { moves: vec![Move::Claim { punter: 0, source: 0, target: 1, }, Move::Claim { punter: 1, source: 1, target: 2, },] },
            Rep::Move { moves: vec![Move::Claim { punter: 0, source: 2, target: 3, }, Move::Claim { punter: 1, source: 3, target: 4, },] },
            Rep::Move { moves: vec![Move::Claim { punter: 0, source: 4, target: 5, }, Move::Claim { punter: 1, source: 5, target: 6, },] },
            Rep::Move { moves: vec![Move::Claim { punter: 0, source: 6, target: 7, }, Move::Claim { punter: 1, source: 7, target: 0, },] },
            Rep::Move { moves: vec![Move::Claim { punter: 0, source: 1, target: 3, }, Move::Claim { punter: 1, source: 3, target: 5, },] },
            Rep::Stop {
                moves: vec![Move::Claim { punter: 0, source: 5, target: 7, }, Move::Claim { punter: 1, source: 7, target: 1, },],
                scores: vec![Score { punter: 0, score: 6, }, Score { punter: 1, score: 6, }],
            },
        ];
    }
}
