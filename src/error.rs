//! Error types for iSCSI target operations

use thiserror::Error;

/// iSCSI target errors
#[derive(Debug, Error)]
pub enum IscsiError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("SCSI error: {0}")]
    Scsi(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Invalid PDU: {0}")]
    InvalidPdu(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),
}

/// Result type for SCSI operations
pub type ScsiResult<T> = Result<T, IscsiError>;
