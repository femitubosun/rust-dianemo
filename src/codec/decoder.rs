use bytes::{Buf, BytesMut};

use crate::{
    errors::AppError,
    types::{DiaReq, Peer, ResStatus},
};

/**
 *
 * 1. `%I{}\r\nN{}\r\nP\r\n`
 */
pub fn decode_dia_value(bs: &mut BytesMut) -> Result<DiaReq, AppError> {
    if bs.is_empty() {
        return Err(AppError::Incomplete);
    }

    let prefix = bs.get_u8();

    match prefix {
        b'%' => decode_anounce(bs),
        b'(' => decode_peer_request(bs),
        b')' => decode_peer_response(bs),
        b'F' => decode_file_offer(bs),
        _ => Err(AppError::InvalidDianemoValue(format!(
            "unknown dia type: {}",
            prefix
        ))),
    }
}

pub fn decode_anounce(bs: &mut BytesMut) -> Result<DiaReq, AppError> {
    let s =
        std::str::from_utf8(bs).map_err(|_| AppError::InvalidDianemoValue("invalid ut8".into()))?;
    let mut lines = s.split("\r\n");
    let peer = decode_peer(&mut lines)?;

    Ok(DiaReq::Announce(peer))
}

pub fn decode_peer_request(bs: &mut BytesMut) -> Result<DiaReq, AppError> {
    let s =
        std::str::from_utf8(bs).map_err(|_| AppError::InvalidDianemoValue("invalid ut8".into()))?;
    let mut lines = s.split("\r\n");
    let peer = decode_peer(&mut lines)?;

    Ok(DiaReq::PairReq(peer))
}

pub fn decode_peer_response(bs: &mut BytesMut) -> Result<DiaReq, AppError> {
    let s =
        std::str::from_utf8(bs).map_err(|_| AppError::InvalidDianemoValue("invalid ut8".into()))?;
    let mut lines = s.split("\r\n");
    let peer = decode_peer(&mut lines)?;

    let status = match next_line(&mut lines, "R", "expected status")? {
        "+" => ResStatus::Accept,
        "-" => ResStatus::Reject,
        _ => return Err(AppError::InvalidDianemoValue("bad status".into())),
    };
    Ok(DiaReq::PairRes(peer, status))
}

pub fn decode_file_offer(bs: &mut BytesMut) -> Result<DiaReq, AppError> {
    let s =
        std::str::from_utf8(bs).map_err(|_| AppError::InvalidDianemoValue("invalid ut8".into()))?;
    let mut lines = s.split("\r\n");
    let peer = decode_peer(&mut lines)?;

    let name = next_line(&mut lines, "L", "expected file name")?.to_string();
    let size = next_line(&mut lines, "S", "expected file size")?
        .parse::<u64>()
        .map_err(|_| AppError::InvalidDianemoValue("bad file size".into()))?;

    Ok(DiaReq::FileOffer(peer, name, size))
}

fn decode_peer<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<Peer, AppError> {
    let id = next_line(lines, "I", "expectedid")?;
    let name = next_line(lines, "N", "expected name")?;
    let ip = next_line(lines, "P", "expected ip")?;
    Ok(Peer::new(id, name, ip))
}

fn next_line<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    prefix: &str,
    err: &str,
) -> Result<&'a str, AppError> {
    lines
        .next()
        .and_then(|l| l.strip_prefix(prefix))
        .ok_or(AppError::InvalidDianemoValue(err.into()))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> BytesMut {
        BytesMut::from(s.as_bytes())
    }

    #[test]
    fn decodes_announce_request() {
        let value =
            decode_dia_value(&mut buf("%I1234\r\nNNormandyPeer\r\nP127.0.0.1\r\n")).unwrap();

        assert_eq!(
            value,
            DiaReq::Announce(Peer::new("1234", "NormandyPeer", "127.0.0.1"))
        )
    }

    #[test]
    fn decodes_peer_request() {
        let value =
            decode_dia_value(&mut buf("(I1234\r\nNNormandyPeer\r\nP127.0.0.1\r\n")).unwrap();

        assert_eq!(
            value,
            DiaReq::PairReq(Peer::new("1234", "NormandyPeer", "127.0.0.1"))
        )
    }

    #[test]
    fn decodes_accept_peer_response() {
        let value =
            decode_dia_value(&mut buf(")I1234\r\nNNormandyPeer\r\nP127.0.0.1\r\nR+\r\n")).unwrap();

        assert_eq!(
            value,
            DiaReq::PairRes(
                Peer::new("1234", "NormandyPeer", "127.0.0.1"),
                ResStatus::Accept
            )
        )
    }

    #[test]
    fn decodes_reject_peer_response() {
        let value =
            decode_dia_value(&mut buf(")I1234\r\nNNormandyPeer\r\nP127.0.0.1\r\nR-\r\n")).unwrap();

        assert_eq!(
            value,
            DiaReq::PairRes(
                Peer::new("1234", "NormandyPeer", "127.0.0.1"),
                ResStatus::Reject
            )
        )
    }

    #[test]
    fn decodes_file_offer() {
        let value =
            decode_dia_value(&mut buf("FI1234\r\nNNormandyPeer\r\nP127.0.0.1\r\nLphoto.jpg\r\nS1024\r\n")).unwrap();

        assert_eq!(
            value,
            DiaReq::FileOffer(
                Peer::new("1234", "NormandyPeer", "127.0.0.1"),
                "photo.jpg".into(),
                1024
            )
        )
    }
}
