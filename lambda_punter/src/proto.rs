use serde_json::value::Value;
use serde_json;

use super::types::{PunterId, SiteId};
use super::map::{Site,River,Map};

#[derive(PartialEq, Debug)]
pub enum Req {
    Handshake { name: String, },
    Ready { punter: PunterId, },
    Move(Move),
}

#[derive(PartialEq, Debug)]
pub enum Rep {
    Handshake { name: String, },
    Setup(Setup),
    Move { moves: Vec<Move>, },
    Stop {
        moves: Vec<Move>,
        scores: Vec<Score>,
    },
}

#[derive(PartialEq, Debug,Deserialize)]
pub struct Setup {
    pub punter: PunterId,
    pub punters: usize,
    pub map: Map,
}

#[derive(Debug,PartialEq,Deserialize)]
pub enum Move {
    Claim { punter: PunterId, source: SiteId, target: SiteId, },
    Pass { punter: PunterId, },
}

#[derive(Debug,PartialEq,Deserialize)]
pub struct Score {
    pub punter: PunterId,
    pub score: usize,
}

#[derive(Debug)]
pub enum Error {
    Json(serde_json::Error),
    UnexpectedJson,

}



#[derive(Debug, Deserialize)]
#[allow(non_camel_case_types)]
enum ServerMove {
    claim { punter: PunterId, source: SiteId, target: SiteId, },
    pass { punter: PunterId, },
}
#[derive(Debug, Deserialize)]
struct ServerMoves {
    moves: Vec<ServerMove>,
}
#[derive(Debug, Deserialize)]
struct ServerStop {
    moves: Vec<ServerMove>,
    scores: Vec<Score>,
}
#[derive(Debug, Deserialize)]
struct ServerMap {
    sites: Vec<Site>,
    rivers: Vec<River>,
    mines: Vec<SiteId>,
}

impl Rep {
    pub fn from_json(s: &str) -> Result<Rep,Error> {
        match serde_json::from_str::<Value>(s).map_err(Error::Json)? {
            Value::Object(mut map) => {
                if map.contains_key("move") {
                    let smove = serde_json::from_value::<ServerMoves>(map.remove("move").unwrap()).map_err(Error::Json)?;
                    Ok(Rep::Move {
                        moves: smove.moves.into_iter().map(|m| {
                            match m {
                                ServerMove::claim { punter, source, target } => Move::Claim { punter: punter, source: source, target: target},
                                ServerMove::pass { punter } => Move::Pass { punter: punter },
                            }
                        }).collect(),
                    })
                }
                else if map.contains_key("stop") {
                    let stop=serde_json::from_value::<ServerStop>(map.remove("stop").unwrap()).map_err(Error::Json)?;
                    Ok(Rep::Stop {
                        moves: stop.moves.into_iter().map(|m| {
                            match m {
                                ServerMove::claim { punter, source, target } => Move::Claim { punter: punter, source: source, target: target},
                                ServerMove::pass { punter } => Move::Pass { punter: punter },
                            }
                        }).collect(),
                        scores: stop.scores,
                    })
                }
                else if map.contains_key("punter") && map.contains_key("punters") && map.contains_key("map") {
                    let smap = serde_json::from_value::<ServerMap>(map.remove("map").unwrap()).map_err(Error::Json)?;
                    Ok(Rep::Setup(Setup{
                        punter: serde_json::from_value::<PunterId>(map.remove("punter").unwrap()).map_err(Error::Json)?,
                        punters: serde_json::from_value::<usize>(map.remove("punters").unwrap()).map_err(Error::Json)?,
                        map: Map {
                            sites: smap.sites.into_iter().map(|s| (s.id,s)).collect(),
                            rivers: smap.rivers.into_iter().collect(),
                            mines: smap.mines.into_iter().collect(),
                        }
                    }))
                }
                /*else if map.contains_key("timeout") {

                }*/
                else if map.contains_key("you") {
                    Ok(Rep::Handshake {
                        name: serde_json::from_value::<String>(map.remove("you").unwrap()).map_err(Error::Json)?,
                    })
                } else {
                    Err(Error::UnexpectedJson)
                }
            },
            _ => {
                Err(Error::UnexpectedJson)
            }
        }

    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_handshake() {
        let object = Rep::from_json("{\"you\": \"test_name\"}").unwrap();
        let result = Rep::Handshake { name: "test_name".to_string() };
        assert_eq!(object,result);
    }

    #[test]
    fn test_move_1() {
        let object = Rep::from_json("{\"move\":{\"moves\":[{\"claim\":{\"punter\":0,\"source\":0,\"target\":1}},{\"claim\":{\"punter\":1,\"source\":1,\"target\":2}}]}}").unwrap();
        let result = Rep::Move {
            moves: vec![
                Move::Claim { punter: 0, source: 0, target: 1, },
                Move::Claim { punter: 1, source: 1, target: 2, },
                ]
        };
        assert_eq!(object,result);
    }

    #[test]
    fn test_move_2() {
        let object = Rep::from_json("{\"move\":{\"moves\":[{\"pass\":{\"punter\":0}},{\"pass\":{\"punter\":1}}]}}").unwrap();
        let result = Rep::Move {
            moves: vec![
                Move::Pass { punter: 0 },
                Move::Pass { punter: 1 },
                ]
        };
        assert_eq!(object,result);
    }

    #[test]
    fn test_stop() {
        let object = Rep::from_json("{\"stop\":{\"moves\":[{\"claim\":{\"punter\":0,\"source\":5,\"target\":7}},{\"claim\":{\"punter\":1,\"source\":7,\"target\":1}}], \"scores\":[{\"punter\":0,\"score\":6},{\"punter\":1,\"score\":6}]}}").unwrap();
        let result = Rep::Stop {
            moves: vec![
                Move::Claim { punter: 0, source: 5, target: 7, },
                Move::Claim { punter: 1, source: 7, target: 1, },
                ],
            scores: vec![
                Score { punter: 0, score: 6 },
                Score { punter: 1, score: 6 },
                ],
        };
        assert_eq!(object,result);
    }

    #[test]
    fn test_setup() {
        let object = Rep::from_json("{\"punter\":0, \"punters\":2,
\"map\":{\"sites\":[{\"id\":4},{\"id\":1},{\"id\":3},{\"id\":6},{\"id\":5},{\"id\":0},{\"id\":7},{\"id\":2}], \"rivers\":[{\"source\":3,\"target\":4},{\"source\":0,\"target\":1},{\"source\":2,\"target\":3}, {\"source\":1,\"target\":3},{\"source\":5,\"target\":6},{\"source\":4,\"target\":5}, {\"source\":3,\"target\":5},{\"source\":6,\"target\":7},{\"source\":5,\"target\":7},{\"source\":1,\"target\":7},{\"source\":0,\"target\":7},{\"source\":1,\"target\":2}], \"mines\":[1,5]}}").unwrap();
        let result = Rep::Setup(Setup {
            punter: 0,
            punters: 2,
            map: Map {
                sites: vec![Site {id:4}, Site {id:1}, Site {id:3}, Site {id:6}, Site {id:5}, Site {id:0}, Site {id:7}, Site {id:2}]
                    .into_iter()
                    .map(|s| {
                        (s.id, s)
                    }).collect(),
                rivers: vec![ River {source:3, target:4},
                         River {source:0, target:1},
                         River {source:2, target:3},
                         River {source:1, target:3},
                         River {source:5, target:6},
                         River {source:4, target:5},
                         River {source:3, target:5},
                         River {source:6, target:7},
                         River {source:5, target:7},
                         River {source:1, target:7},
                         River {source:0, target:7},
                         River {source:1, target:2} ].into_iter().collect(),
                mines: vec![1,5].into_iter().collect(),
            },
        });
        assert_eq!(object,result);
    }

}
