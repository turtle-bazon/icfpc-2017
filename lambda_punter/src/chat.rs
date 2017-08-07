
use super::proto::{Req, Rep, Score};
use super::game::{GameState, GameStateBuilder};

#[derive(PartialEq, Debug)]
pub enum Error<SR, RR, GE> {
    Send(SR),
    Recv(RR),
    GameState(GE),
    UnexpectedHandshakeRep(Rep),
    UnexpectedSetupRep(Rep),
    UnexpectedMoveRep(Rep),
    UnexpectedSetupOrMoveRep(Rep),
    UnexpectedStateArrived,
    NoExpectedStateArrived,
}

pub fn run_online<S, FS, SR, FR, RR, GB>(
    name: &str,
    mut fn_state: S,
    mut send_fn: FS,
    mut recv_fn: FR,
    gs_builder: GB)
    -> Result<(Vec<Score>, GB::GameState), Error<SR, RR, <GB::GameState as GameState>::Error>>
    where FS: FnMut(&mut S, Req, Option<GB::GameState>) -> Result<(), SR>,
          FR: FnMut(&mut S) -> Result<(Rep, Option<GB::GameState>), RR>,
          GB: GameStateBuilder
{
    // P → S {"me" : name}
    send_fn(&mut fn_state, Req::Handshake { name: name.to_string(), }, None)
        .map_err(Error::Send)?;
    // S → P {"you" : name}
    match recv_fn(&mut fn_state).map_err(Error::Recv)? {
        (Rep::Handshake { name: ref rep_name, }, _) if rep_name == name =>
            (),
        (other, _) =>
            return Err(Error::UnexpectedHandshakeRep(other)),
    }
    // S → P {"punter" : p, "punters" : n, "map" : map}
    let mut game_state =
        match recv_fn(&mut fn_state).map_err(Error::Recv)? {
            (Rep::Setup(setup), _) =>
                gs_builder.build(setup),
            (other, _) =>
                return Err(Error::UnexpectedSetupRep(other)),
        };
    // P → S {"ready" : p}
    send_fn(&mut fn_state, Req::Ready { punter: game_state.get_punter(), futures: game_state.get_futures(), }, None)
        .map_err(Error::Send)?;
    // gameplay
    loop {
        // S → P {"move" : {"moves" : moves}}
        // S → P {"stop" : {"moves" : moves,"scores" : scores}}
        match recv_fn(&mut fn_state).map_err(Error::Recv)? {
            (Rep::Move { moves, }, _) => {
                let (move_, next_game_state) = game_state.play(moves)
                    .map_err(Error::GameState)?;
                game_state = next_game_state;
                send_fn(&mut fn_state, Req::Move(move_), None).map_err(Error::Send)?;
            },
            (Rep::Stop { scores, moves, }, _) =>
                return Ok((scores, game_state.stop(moves).map_err(Error::GameState)?)),
            (other, _) =>
                return Err(Error::UnexpectedMoveRep(other)),
        }

    }
}

pub fn run_offline<S, FS, SR, FR, RR, GB>(
    name: &str,
    mut fn_state: S,
    mut send_fn: FS,
    mut recv_fn: FR,
    gs_builder: GB)
    -> Result<Option<(Vec<Score>, GB::GameState)>, Error<SR, RR, <GB::GameState as GameState>::Error>>
    where FS: FnMut(&mut S, Req, Option<GB::GameState>) -> Result<(), SR>,
          FR: FnMut(&mut S) -> Result<(Rep, Option<GB::GameState>), RR>,
          GB: GameStateBuilder
{
    // P → S {"me" : name}
    send_fn(&mut fn_state, Req::Handshake { name: name.to_string(), }, None)
        .map_err(Error::Send)?;
    // S → P {"you" : name}
    match recv_fn(&mut fn_state).map_err(Error::Recv)? {
        (Rep::Handshake { name: ref rep_name, }, None) if rep_name == name =>
            (),
        (Rep::Handshake { .. }, Some(..)) =>
            return Err(Error::UnexpectedStateArrived),
        (other, _) =>
            return Err(Error::UnexpectedHandshakeRep(other)),
    }
    match recv_fn(&mut fn_state).map_err(Error::Recv)? {
        // S → P {"punter" : p, "punters" : n, "map" : map}
        (Rep::Setup(setup), None) => {
            let mut game_state = gs_builder.build(setup);
            // P → S {"ready" : p, "state" : state}
            send_fn(&mut fn_state, Req::Ready { punter: game_state.get_punter(), futures: game_state.get_futures(), }, Some(game_state))
                .map_err(Error::Send)
                .map(|()| None)
        },
        (Rep::Setup(..), Some(..)) =>
            Err(Error::UnexpectedStateArrived),
        // S → P {"move" : {"moves" : moves},"state" : state}
        (Rep::Move { moves, }, Some(game_state)) => {
            let (move_, next_game_state) = game_state.play(moves)
                .map_err(Error::GameState)?;
            send_fn(&mut fn_state, Req::Move(move_), Some(next_game_state))
                .map_err(Error::Send)
                .map(|()| None)
        },
        (Rep::Move { .. }, None) =>
            Err(Error::NoExpectedStateArrived),
        // S → P {"stop" : {"moves" : moves,"scores" : scores},"state" : state}
        (Rep::Stop { scores, moves, }, Some(game_state)) =>
            Ok(Some((scores, game_state.stop(moves).map_err(Error::GameState)?))),
        (Rep::Stop { .. }, None) =>
            Err(Error::NoExpectedStateArrived),
        (other, _) =>
            return Err(Error::UnexpectedSetupOrMoveRep(other)),
    }
}

#[cfg(test)]
mod test {
    use super::{Error, run_online};
    use super::super::types::PunterId;
    use super::super::map::{Map, River};
    use super::super::proto::{Req, Rep, Move, Setup, Score};
    use super::super::game::{GameStateBuilder, GameState};
    use super::super::solvers::always_pass::AlwaysPassGameStateBuilder;

    #[test]
    fn handshake_err() {
        assert_eq!(
            run_online(
                "alice",
                (),
                |_, _req, _| Ok::<_, ()>(()),
                |_| Ok::<_, ()>((Rep::Handshake { name: "bob".to_string(), }, None)),
                AlwaysPassGameStateBuilder)
                .map(|v| v.0),
            Err(Error::UnexpectedHandshakeRep(Rep::Handshake { name: "bob".to_string(), })));
    }

    fn default_map() -> Map {
        let mut map = Map {
            sites: vec![4, 1, 3, 6, 5, 0, 7, 2],
            mines: vec![1, 5],
            ..Default::default()
        };
        map.rivers.push(River::new(3, 4));
        map.rivers.push(River::new(0, 1));
        map.rivers.push(River::new(2, 3));
        map.rivers.push(River::new(1, 3));
        map.rivers.push(River::new(5, 6));
        map.rivers.push(River::new(4, 5));
        map.rivers.push(River::new(3, 5));
        map.rivers.push(River::new(6, 7));
        map.rivers.push(River::new(5, 7));
        map.rivers.push(River::new(1, 7));
        map.rivers.push(River::new(0, 7));
        map.rivers.push(River::new(1, 2));
        map
    }

    #[test]
    fn sample_alice_script() {
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

        common_test_script(
            "Alice",
            vec![
                Req::Handshake { name: "Alice".to_string(), },
                Req::Ready { punter: 0, futures: None, },
                Req::Move(Move::Claim { punter: 0, source: 0, target: 1, }),
                Req::Move(Move::Claim { punter: 0, source: 2, target: 3, }),
                Req::Move(Move::Claim { punter: 0, source: 4, target: 5, }),
                Req::Move(Move::Claim { punter: 0, source: 6, target: 7, }),
                Req::Move(Move::Claim { punter: 0, source: 1, target: 3, }),
                Req::Move(Move::Claim { punter: 0, source: 5, target: 7, }),
            ],
            vec![
                Rep::Handshake { name: "Alice".to_string(), },
                Rep::Setup(Setup {
                    punter: 0,
                    punters: 2,
                    map: default_map(),
                    settings: Default::default(),
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
            ],
            vec![
                Move::Claim { punter: 0, source: 0, target: 1, },
                Move::Claim { punter: 0, source: 2, target: 3, },
                Move::Claim { punter: 0, source: 4, target: 5, },
                Move::Claim { punter: 0, source: 6, target: 7, },
                Move::Claim { punter: 0, source: 1, target: 3, },
                Move::Claim { punter: 0, source: 5, target: 7, },
            ],
            vec![Score { punter: 0, score: 6 }, Score { punter: 1, score: 6 }]
        );
    }

    #[test]
    fn sample_bob_script() {
        // -> {"me":"Bob"}
        // <- {"you":"Bob"}
        // <- {"punter":1, "punters":2,
        // "map":{"sites":[{"id":4},{"id":1},{"id":3},{"id":6},{"id":5},{"id":0},{"id":7},{"id":2}], "rivers":[{"source":3,"target":4},{"source":0,"target":1},{"source":2,"target":3}, {"source":1,"target":3},{"source":5,"target":6},{"source":4,"target":5}, {"source":3,"target":5},{"source":6,"target":7},{"source":5,"target":7},
        // {"source":1,"target":7},{"source":0,"target":7},{"source":1,"target":2}], "mines":[1,5]}}
        // -> {"ready":1}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":0,"target":1}},{"pass":{"punter":1}}]}}
        // -> {"claim":{"punter":1,"source":1,"target":2}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":2,"target":3}},{"claim":{"punter":1,"source":1,"target":2}}]}}
        // -> {"claim":{"punter":1,"source":3,"target":4}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":4,"target":5}},{"claim":{"punter":1,"source":3,"target":4}}]}}
        // -> {"claim":{"punter":1,"source":5,"target":6}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":6,"target":7}},{"claim":{"punter":1,"source":5,"target":6}}]}}
        // -> {"claim":{"punter":1,"source":7,"target":0}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":1,"target":3}},{"claim":{"punter":1,"source":7,"target":0}}]}}
        // -> {"claim":{"punter":1,"source":3,"target":5}}
        // <- {"move":{"moves":[{"claim":{"punter":0,"source":5,"target":7}},{"claim":{"punter":1,"source":3,"target":5}}]}}
        // -> {"claim":{"punter":1,"source":7,"target":1}}
        // <- {"stop":{"moves":[{"claim":{"punter":0,"source":5,"target":7}},{"claim":{"punter":1,"source":7,"target":1}}], "scores":[{"punter":0,"score":6},{"punter":1,"score":6}]}}

        common_test_script(
            "Bob",
            vec![
                Req::Handshake { name: "Bob".to_string(), },
                Req::Ready { punter: 1, futures: None, },
                Req::Move(Move::Claim { punter: 1, source: 1, target: 2, }),
                Req::Move(Move::Claim { punter: 1, source: 3, target: 4, }),
                Req::Move(Move::Claim { punter: 1, source: 5, target: 6, }),
                Req::Move(Move::Claim { punter: 1, source: 7, target: 0, }),
                Req::Move(Move::Claim { punter: 1, source: 3, target: 5, }),
                Req::Move(Move::Claim { punter: 1, source: 7, target: 1, }),
            ],
            vec![
                Rep::Handshake { name: "Bob".to_string(), },
                Rep::Setup(Setup {
                    punter: 1,
                    punters: 2,
                    map: default_map(),
                    settings: Default::default(),
                }),
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 0, target: 1, }, Move::Pass { punter: 1, },] },
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 2, target: 3, }, Move::Claim { punter: 1, source: 1, target: 2, },] },
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 4, target: 5, }, Move::Claim { punter: 1, source: 3, target: 4, },] },
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 6, target: 7, }, Move::Claim { punter: 1, source: 5, target: 6, },] },
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 1, target: 3, }, Move::Claim { punter: 1, source: 7, target: 0, },] },
                Rep::Move { moves: vec![Move::Claim { punter: 0, source: 5, target: 7, }, Move::Claim { punter: 1, source: 3, target: 5, },] },
                Rep::Stop {
                    moves: vec![Move::Claim { punter: 0, source: 5, target: 7, }, Move::Claim { punter: 1, source: 7, target: 1, },],
                    scores: vec![Score { punter: 0, score: 6, }, Score { punter: 1, score: 6, }],
                },
            ],
            vec![
                Move::Claim { punter: 1, source: 1, target: 2, },
                Move::Claim { punter: 1, source: 3, target: 4, },
                Move::Claim { punter: 1, source: 5, target: 6, },
                Move::Claim { punter: 1, source: 7, target: 0, },
                Move::Claim { punter: 1, source: 3, target: 5, },
                Move::Claim { punter: 1, source: 7, target: 1, },
            ],
            vec![Score { punter: 0, score: 6 }, Score { punter: 1, score: 6 }]
        );
    }

    fn common_test_script(name: &str, mut reqs: Vec<Req>, mut reps: Vec<Rep>, mut gs_script: Vec<Move>, expected_score: Vec<Score>) {
        reqs.reverse();
        reps.reverse();
        gs_script.reverse();

        #[derive(PartialEq, Debug)]
        struct Unexpected<T> { expected: Option<T>, provided: T, };
        #[derive(PartialEq, Debug)]
        struct RepsStackIsEmpty;

        #[derive(Serialize, Deserialize)]
        struct ScriptGameState {
            punter: PunterId,
            script: Vec<Move>,
        };

        struct ScriptGameStateBuilder(Vec<Move>);

        impl GameStateBuilder for ScriptGameStateBuilder {
            type GameState = ScriptGameState;

            fn build(self, setup: Setup) -> Self::GameState {
                ScriptGameState {
                    punter: setup.punter,
                    script: self.0,
                }
            }
        }

        #[derive(PartialEq, Debug)]
        struct MovesStackIsEmpty;

        impl GameState for ScriptGameState {
            type Error = MovesStackIsEmpty;

            fn play(mut self, _moves: Vec<Move>) -> Result<(Move, Self), Self::Error> {
                if let Some(move_) = self.script.pop() {
                    Ok((move_, self))
                } else {
                    Err(MovesStackIsEmpty)
                }
            }

            fn stop(self, _moves: Vec<Move>) -> Result<Self, Self::Error> {
                Ok(self)
            }

            fn get_punter(&self) -> PunterId {
                self.punter
            }
        }

        let (final_score, final_state) = run_online(
            name,
            (),
            |_, req, _| if let Some(expected_req) = reqs.pop() {
                if expected_req == req {
                    Ok(())
                } else {
                    Err(Unexpected { expected: Some(expected_req), provided: req, })
                }
            } else {
                Err(Unexpected { expected: None, provided: req, })
            },
            |_| if let Some(rep) = reps.pop() {
                Ok((rep, None))
            } else {
                Err(RepsStackIsEmpty)
            },
            ScriptGameStateBuilder(gs_script))
            .unwrap();
        assert_eq!(final_score, expected_score);
        assert_eq!(final_state.script, vec![]);
    }
}
