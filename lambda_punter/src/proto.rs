use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use serde_json::value::Value;
use serde_json;

use super::types::{PunterId, SiteId};
use super::map::{Site,River,Map};
use std::collections::BTreeMap;

#[derive(PartialEq, Debug)]
pub enum Req {
    Handshake { name: String, },
    Ready { punter: PunterId, futures: Option<Vec<Future>>, },
    Move(Move),
}

#[derive(PartialEq, Debug)]
pub enum Rep {
    Handshake { name: String, },
    Timeout(usize),
    Setup(Setup),
    Move { moves: Vec<Move>, },
    Stop {
        moves: Vec<Move>,
        scores: Vec<Score>,
    },
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Future {
    pub source: SiteId,
    pub target: SiteId,
}

#[derive(PartialEq, Debug, Deserialize)]
pub struct Setup {
    pub punter: PunterId,
    pub punters: usize,
    pub map: Map,
    pub settings: Settings,
}

#[derive(PartialEq, Default, Debug, Deserialize)]
pub struct Settings {
    pub futures: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Move {
    Claim { punter: PunterId, source: SiteId, target: SiteId, },
    Pass { punter: PunterId, },
}

#[derive(Debug, PartialEq, Deserialize)]
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
    pub fn from_json<S>(s: &str) -> Result<(Rep, Option<S>), Error> where S: DeserializeOwned {
        match serde_json::from_str::<Value>(s).map_err(Error::Json)? {
            Value::Object(mut map) => {
                let maybe_state = if let Some(value) = map.remove("state") {
                    if map.contains_key("move") || map.contains_key("stop") {
                        Some(serde_json::from_value::<S>(value).map_err(Error::Json)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if map.contains_key("move") {
                    let move_node = map.remove("move").unwrap();
                    let smove = serde_json::from_value::<ServerMoves>(move_node).map_err(Error::Json)?;
                    Ok((Rep::Move {
                        moves: smove.moves.into_iter().map(|m| {
                            match m {
                                ServerMove::claim { punter, source, target } => Move::Claim { punter: punter, source: source, target: target},
                                ServerMove::pass { punter } => Move::Pass { punter: punter },
                            }
                        }).collect(),
                    }, maybe_state))
                } else if map.contains_key("stop") {
                    let stop_node = map.remove("stop").unwrap();
                    let stop = serde_json::from_value::<ServerStop>(stop_node).map_err(Error::Json)?;
                    Ok((Rep::Stop {
                        moves: stop.moves.into_iter().map(|m| {
                            match m {
                                ServerMove::claim { punter, source, target } => Move::Claim { punter: punter, source: source, target: target},
                                ServerMove::pass { punter } => Move::Pass { punter: punter },
                            }
                        }).collect(),
                        scores: stop.scores,
                    }, maybe_state))
                } else if map.contains_key("punter") && map.contains_key("punters") && map.contains_key("map") {
                    let smap = serde_json::from_value::<ServerMap>(map.remove("map").unwrap()).map_err(Error::Json)?;
                    Ok((Rep::Setup(Setup{
                        punter: serde_json::from_value::<PunterId>(map.remove("punter").unwrap()).map_err(Error::Json)?,
                        punters: serde_json::from_value::<usize>(map.remove("punters").unwrap()).map_err(Error::Json)?,
                        map: Map {
                            sites: smap.sites.into_iter().map(|s| (s.id,s)).collect(),
                            rivers: smap.rivers.into_iter().collect(),
                            mines: smap.mines.into_iter().collect(),
                        },
                        settings: if let Some(Value::Object(mut settings_obj)) = map.remove("settings") {
                            Settings {
                                futures: match settings_obj.remove("futures") {
                                    Some(Value::Bool(true)) => true,
                                    _ => false,
                                }
                            }
                        } else {
                            Default::default()
                        },
                    }), None))
                } else if map.contains_key("timeout") {
                    Ok((Rep::Timeout(serde_json::from_value::<usize>(map.remove("timeout").unwrap()).map_err(Error::Json)?), None))
                } else if map.contains_key("you") {
                    Ok((Rep::Handshake {
                        name: serde_json::from_value::<String>(map.remove("you").unwrap()).map_err(Error::Json)?,
                    }, None))
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

impl Req {
    pub fn to_json<S>(self, maybe_state: Option<S>) -> Result<String, Error> where S: Serialize {
        let mut res = BTreeMap::new();
        match self {
            Req::Handshake { name } => {
                res.insert("me".to_string(), serde_json::to_value(name).map_err(Error::Json)?);
            },
            Req::Ready { punter, futures, } => {
                res.insert("ready".to_string(), serde_json::to_value(punter).map_err(Error::Json)?);
                if let Some(futs) = futures {
                    let fut_values = futs.into_iter()
                        .map(|fut| {
                            serde_json::to_value(
                                vec![("source".to_string(), serde_json::to_value(fut.source).map_err(Error::Json)?),
                                     ("target".to_string(), serde_json::to_value(fut.target).map_err(Error::Json)?)]
                                    .into_iter()
                                    .collect::<BTreeMap<String, Value>>()).map_err(Error::Json)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    res.insert("futures".to_string(), Value::Array(fut_values));
                }
                if let Some(state) = maybe_state {
                    res.insert("state".to_string(), serde_json::to_value(state).map_err(Error::Json)?);
                }
            },
            Req::Move(mv) => {
                match mv {
                    Move::Claim { punter, source, target } => {
                        let m = vec![("punter",serde_json::to_value(punter).map_err(Error::Json)?),
                                     ("source",serde_json::to_value(source).map_err(Error::Json)?),
                                     ("target",serde_json::to_value(target).map_err(Error::Json)?),
                                     ].into_iter()
                            .map(|x| (x.0.to_string(),x.1))
                            .collect::<BTreeMap<String,Value>>();
                        res.insert("claim".to_string(),serde_json::to_value(m).map_err(Error::Json)?);
                    },
                    Move::Pass { punter } => {
                        let m = vec![("punter",serde_json::to_value(punter).map_err(Error::Json)?)].into_iter()
                            .map(|x| (x.0.to_string(),x.1))
                            .collect::<BTreeMap<String,Value>>();
                        res.insert("pass".to_string(),serde_json::to_value(m).map_err(Error::Json)?);
                    }
                }
                if let Some(state) = maybe_state {
                    res.insert("state".to_string(), serde_json::to_value(state).map_err(Error::Json)?);
                }
            }
        }
        Ok(serde_json::to_string(&res).map_err(Error::Json)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn proto_handshake() {
        let object = Rep::from_json::<()>("{\"you\": \"test_name\"}").unwrap().0;
        let result = Rep::Handshake { name: "test_name".to_string() };
        assert_eq!(object,result);
    }

    #[test]
    fn proto_move_1() {
        let object = Rep::from_json::<()>("{\"move\":{\"moves\":[{\"claim\":{\"punter\":0,\"source\":0,\"target\":1}},{\"claim\":{\"punter\":1,\"source\":1,\"target\":2}}]}}").unwrap().0;
        let result = Rep::Move {
            moves: vec![
                Move::Claim { punter: 0, source: 0, target: 1, },
                Move::Claim { punter: 1, source: 1, target: 2, },
                ]
        };
        assert_eq!(object,result);
    }

    #[test]
    fn proto_move_2() {
        let object = Rep::from_json::<()>("{\"move\":{\"moves\":[{\"pass\":{\"punter\":0}},{\"pass\":{\"punter\":1}}]}}").unwrap().0;
        let result = Rep::Move {
            moves: vec![
                Move::Pass { punter: 0 },
                Move::Pass { punter: 1 },
                ]
        };
        assert_eq!(object,result);
    }

    #[test]
    fn proto_stop() {
        let object = Rep::from_json::<()>("{\"stop\":{\"moves\":[{\"claim\":{\"punter\":0,\"source\":5,\"target\":7}},{\"claim\":{\"punter\":1,\"source\":7,\"target\":1}}], \"scores\":[{\"punter\":0,\"score\":6},{\"punter\":1,\"score\":6}]}}").unwrap().0;
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
    fn proto_setup() {
        let object = Rep::from_json::<()>("{\"punter\":0, \"punters\":2,
\"map\":{\"sites\":[{\"id\":4},{\"id\":1},{\"id\":3},{\"id\":6},{\"id\":5},{\"id\":0},{\"id\":7},{\"id\":2}], \"rivers\":[{\"source\":3,\"target\":4},{\"source\":0,\"target\":1},{\"source\":2,\"target\":3}, {\"source\":1,\"target\":3},{\"source\":5,\"target\":6},{\"source\":4,\"target\":5}, {\"source\":3,\"target\":5},{\"source\":6,\"target\":7},{\"source\":5,\"target\":7},{\"source\":1,\"target\":7},{\"source\":0,\"target\":7},{\"source\":1,\"target\":2}], \"mines\":[1,5]}}").unwrap().0;
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
            settings: Default::default(),
        });
        assert_eq!(object,result);
    }

    #[test]
    fn proto_setup_settings() {
        let object = Rep::from_json::<()>("{\"punter\":0, \"punters\":2,
\"map\":{\"sites\":[{\"id\":4},{\"id\":1},{\"id\":3},{\"id\":6},{\"id\":5},{\"id\":0},{\"id\":7},{\"id\":2}], \"rivers\":[{\"source\":3,\"target\":4},{\"source\":0,\"target\":1},{\"source\":2,\"target\":3}, {\"source\":1,\"target\":3},{\"source\":5,\"target\":6},{\"source\":4,\"target\":5}, {\"source\":3,\"target\":5},{\"source\":6,\"target\":7},{\"source\":5,\"target\":7},{\"source\":1,\"target\":7},{\"source\":0,\"target\":7},{\"source\":1,\"target\":2}], \"mines\":[1,5]},\"settings\":{\"futures\":true}}").unwrap().0;
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
            settings: Settings {
                futures: true,
            },
        });
        assert_eq!(object,result);
    }

    #[test]
    fn proto_timeout() {
        let object = Rep::from_json::<()>("{\"timeout\": 10}").unwrap().0;
        let result = Rep::Timeout(10);
        assert_eq!(object,result);
    }

    #[test]
    fn proto_out_handshake() {
        let object = Req::Handshake { name: "test_name".to_string() };
        let result = "{\"me\":\"test_name\"}";
        assert_eq!(object.to_json::<()>(None).unwrap(),result.to_string());
    }

    #[test]
    fn proto_out_ready() {
        let object = Req::Ready { punter: 1, futures: None, };
        let result = "{\"ready\":1}";
        assert_eq!(object.to_json::<()>(None).unwrap(),result.to_string());
    }

    #[test]
    fn proto_out_ready_futs() {
        let object = Req::Ready { punter: 1, futures: Some(vec![Future { source: 0, target: 1, }]), };
        let result = "{\"futures\":[{\"source\":0,\"target\":1}],\"ready\":1}";
        assert_eq!(object.to_json::<()>(None).unwrap(),result.to_string());
    }

    #[test]
    fn proto_out_move_1() {
        let object = Req::Move(Move::Claim { punter: 2, source: 8, target: 1 });
        let result = "{\"claim\":{\"punter\":2,\"source\":8,\"target\":1}}";
        assert_eq!(object.to_json::<()>(None).unwrap(),result.to_string());
    }

    #[test]
    fn proto_out_move_2() {
        let object = Req::Move(Move::Pass { punter: 0 });
        let result = "{\"pass\":{\"punter\":0}}";
        assert_eq!(object.to_json::<()>(None).unwrap(),result.to_string());
    }

}
