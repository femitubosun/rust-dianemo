use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unterminated string")]
    UnterminatedString,

    #[error("invalid dianemo value: {0}")]
    InvalidDianemoValue(String),

    #[error("incomplete frame: more byres needed")]
    Incomplete,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse ip address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}
