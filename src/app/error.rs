use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unable to build app's config directory")]
    ConfigDirError,

    #[error("File not found")]
    FileNotFound,

    #[error("App has not been started yet")]
    NotStarted,

    #[error("Command channel closed")]
    ChannelClosed,

    #[error("Invalid instrument index (not found in compiled graph)")]
    InvalidInstrumentIndex,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Audio error: {0}")]
    AudioError(String),
}
