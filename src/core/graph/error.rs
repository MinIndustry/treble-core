use thiserror::Error;

/// Errors from building or driving the audio graph.
///
/// Every variant carries a stable `TRBC-GRAPH-1xx` code in its message so a
/// report from a log line, a UI banner, or a bug ticket can be grepped
/// straight back to its source. The codes never get reused or renumbered.
#[derive(Debug, Error, Clone)]
pub enum AudioGraphError {
    #[error("TRBC-GRAPH-101: invalid node")]
    InvalidNode,

    #[error("TRBC-GRAPH-102: invalid port")]
    InvalidPort,

    #[error("TRBC-GRAPH-103: node not found")]
    NodeNotFound,

    #[error("TRBC-GRAPH-104: port not found")]
    PortNotFound,

    #[error("TRBC-GRAPH-105: connection not allowed")]
    ConnectionNotAllowed,

    #[error("TRBC-GRAPH-106: invalid merging")]
    InvalidMerging,

    #[error("TRBC-GRAPH-107: audio graph cycle detected")]
    CycleDetected,

    #[error("TRBC-GRAPH-108: processing error: {0}")]
    ProcessingError(&'static str),

    #[error("TRBC-GRAPH-109: unknown parameter '{parameter}' for {target}")]
    UnknownParameter { target: String, parameter: String },
}

impl AudioGraphError {
    /// The variant's stable error code, for programmatic surfacing.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidNode => "TRBC-GRAPH-101",
            Self::InvalidPort => "TRBC-GRAPH-102",
            Self::NodeNotFound => "TRBC-GRAPH-103",
            Self::PortNotFound => "TRBC-GRAPH-104",
            Self::ConnectionNotAllowed => "TRBC-GRAPH-105",
            Self::InvalidMerging => "TRBC-GRAPH-106",
            Self::CycleDetected => "TRBC-GRAPH-107",
            Self::ProcessingError(_) => "TRBC-GRAPH-108",
            Self::UnknownParameter { .. } => "TRBC-GRAPH-109",
        }
    }
}
