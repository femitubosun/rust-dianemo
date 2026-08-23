use std::fmt;

use bytes::Bytes;

use crate::codec::encoder::encode_dia_req;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub ip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResStatus {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiaReq {
    Announce(Peer),
    PairReq(Peer),
    PairRes(Peer, ResStatus),
    /// from, filename, size in bytes — raw bytes follow after Accept
    FileOffer(Peer, String, u64),
    Err(String),
}

impl fmt::Display for DiaReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiaReq::Announce(p) => write!(f, "Id: {} |  Name: {} | Ip: {}", p.id, p.name, p.ip),
            DiaReq::PairReq(p) => write!(
                f,
                "Id: {} | Name: {} | Ip: {} | Peer Req",
                p.id, p.name, p.ip
            ),
            DiaReq::PairRes(p, status) => {
                let s = match status {
                    ResStatus::Accept => "Accept",
                    ResStatus::Reject => "Reject",
                };
                write!(
                    f,
                    "Id: {} | Name: {} | Ip : {} | Peer Resp : {}",
                    p.id, p.name, p.ip, s
                )
            }
            DiaReq::Err(s) => write!(f, "(error) {}", s),
            DiaReq::FileOffer(p, name, size) => write!(
                f,
                "Id: {} | Name: {} | File: {} ({} bytes)",
                p.id, p.name, name, size
            ),
        }
    }
}

impl Peer {
    pub fn new(id: impl Into<String>, name: impl Into<String>, ip: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ip: ip.into(),
        }
    }
}

impl DiaReq {
    pub fn announce(peer: Peer) -> Self {
        Self::Announce(peer)
    }

    pub fn pair_req(peer: Peer) -> Self {
        Self::PairReq(peer)
    }

    pub fn pair_res(peer: Peer, status: ResStatus) -> Self {
        Self::PairRes(peer, status)
    }

    pub fn file_offer(peer: Peer, name: impl Into<String>, size: u64) -> Self {
        Self::FileOffer(peer, name.into(), size)
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self::Err(msg.into())
    }

    pub fn to_bytes(&self) -> Result<Bytes, AppError> {
        encode_dia_req(self.clone())
    }
}
