
use super::proto::{Req, Rep};

#[derive(PartialEq, Debug)]
pub enum Error<SR, RR> {
    Send(SR),
    Recv(RR),
    UnexpectedHandshakeRep(Rep),
}

pub fn run_online<FS, SR, FR, RR>(name: &str, mut send_fn: FS, mut recv_fn: FR) -> Result<(), Error<SR, RR>>
    where FS: FnMut(Req) -> Result<(), SR>,
          FR: FnMut() -> Result<Rep, RR>,
{
    send_fn(Req::Handshake { name: name.to_string(), })
        .map_err(Error::Send)?;
    match recv_fn().map_err(Error::Recv)? {
        Rep::Handshake { name: ref rep_name, } if rep_name == name =>
            (),
        other =>
            return Err(Error::UnexpectedHandshakeRep(other)),
    }

    Ok(())
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
