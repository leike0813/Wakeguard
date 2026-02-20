use thiserror::Error;

#[derive(Debug, Error)]
pub enum WakeguardError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("registry error: {0}")]
    Registry(String),

    #[error("command failed: {command}; details: {details}")]
    CommandFailed { command: String, details: String },
}
