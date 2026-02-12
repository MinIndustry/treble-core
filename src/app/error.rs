use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unable to build app's config directory")]
    ConfigDirError,

    #[error("File not found")]
    FileNotFound,
}
