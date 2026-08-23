use bytes::Bytes;

use crate::{
    errors::AppError,
    types::{DiaReq, ResStatus},
};

pub fn encode_dia_req(val: DiaReq) -> Result<Bytes, AppError> {
    match val {
        DiaReq::Announce(_) => encode_announce(val),
        DiaReq::PairReq(_) => encode_peer_req(val),
        DiaReq::PairRes(_, _) => encode_peer_resp(val),
        DiaReq::FileOffer(_, _, _) => encode_file_offer(val),
        DiaReq::Err(s) => Ok(Bytes::from(format!("-{}\r\n", s))),
    }
}

fn encode_file_offer(val: DiaReq) -> Result<Bytes, AppError> {
    match val {
        DiaReq::FileOffer(p, name, size) => Ok(Bytes::from(format!(
            "FI{}\r\nN{}\r\nP{}\r\nL{}\r\nS{}\r\n",
            p.id, p.name, p.ip, name, size
        ))),
        _ => Err(AppError::InvalidDianemoValue(
            "expected file offer".to_string(),
        )),
    }
}

fn encode_announce(val: DiaReq) -> Result<Bytes, AppError> {
    match val {
        DiaReq::Announce(p) => Ok(Bytes::from(format!(
            "%I{}\r\nN{}\r\nP{}\r\n",
            p.id, p.name, p.ip
        ))),
        _ => Err(AppError::InvalidDianemoValue(
            "expected announce".to_string(),
        )),
    }
}

fn encode_peer_req(val: DiaReq) -> Result<Bytes, AppError> {
    match val {
        DiaReq::PairReq(p) => Ok(Bytes::from(format!(
            "(I{}\r\nN{}\r\nP{}\r\n",
            p.id, p.name, p.ip
        ))),
        _ => Err(AppError::InvalidDianemoValue(
            "expected peer request".to_string(),
        )),
    }
}

fn encode_peer_resp(val: DiaReq) -> Result<Bytes, AppError> {
    match val {
        DiaReq::PairRes(p, status) => {
            let s = match status {
                ResStatus::Accept => "+",
                ResStatus::Reject => "-",
            };

            Ok(Bytes::from(format!(
                ")I{}\r\nN{}\r\nP{}\r\nR{}\r\n",
                p.id, p.name, p.ip, s
            )))
        }
        _ => Err(AppError::InvalidDianemoValue(
            "expected peer response".to_string(),
        )),
    }
}
