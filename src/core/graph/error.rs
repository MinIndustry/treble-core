use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AudioGraphError {
    #[error("invalid node")]
    InvalidNode,

    #[error("invalid port")]
    InvalidPort,

    #[error("node not found")]
    NodeNotFound,

    #[error("port not found")]
    PortNotFound,

    #[error("connection not allowed")]
    ConnectionNotAllowed,

    #[error("invalid merging")]
    InvalidMerging,

    #[error("audio graph cycle detected")]
    CycleDetected,

    #[error("processing error: {0}")]
    ProcessingError(&'static str),
}
