//! Error types for the unified UE tools library

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, UeToolError>;

/// Main error enum for the library
#[derive(Error, Debug)]
pub enum UeToolError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Pak file error: {0}")]
    PakError(String),

    #[error("UTOC file error: {0}")]
    UtocError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("Missing required file: {0}")]
    MissingFile(PathBuf),

    #[error("Invalid AES key: {0}")]
    InvalidAesKey(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("JSON error: {0}")]
    JsonError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("External tool error: {0}")]
    ExternalTool(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Timeout")]
    Timeout,

    #[error("Cancelled")]
    Cancelled,

    #[error("Other error: {0}")]
    Other(String),
}

impl UeToolError {
    /// Create an IO error with a formatted message
    pub fn io_error<S: Into<String>>(msg: S) -> Self {
        Self::IoError(msg.into())
    }

    /// Create a file not found error
    pub fn file_not_found<P: Into<PathBuf>>(path: P) -> Self {
        Self::FileNotFound(path.into())
    }

    /// Create an invalid format error
    pub fn invalid_format<S: Into<String>>(msg: S) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Create a missing file error
    pub fn missing_file<P: Into<PathBuf>>(path: P) -> Self {
        Self::MissingFile(path.into())
    }

    /// Create an invalid AES key error
    pub fn invalid_aes_key<S: Into<String>>(msg: S) -> Self {
        Self::InvalidAesKey(msg.into())
    }

    /// Create an invalid argument error
    pub fn invalid_argument<S: Into<String>>(msg: S) -> Self {
        Self::InvalidArgument(msg.into())
    }
}

impl From<std::io::Error> for UeToolError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::FileNotFound(PathBuf::new()),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied(error.to_string()),
            std::io::ErrorKind::OutOfMemory => Self::OutOfMemory,
            _ => Self::IoError(error.to_string()),
        }
    }
}

impl From<zip::result::ZipError> for UeToolError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::IoError(format!("Zip error: {}", error))
    }
}

impl From<hex::FromHexError> for UeToolError {
    fn from(error: hex::FromHexError) -> Self {
        Self::InvalidAesKey(format!("Invalid hex: {}", error))
    }
}

impl From<base64::DecodeError> for UeToolError {
    fn from(error: base64::DecodeError) -> Self {
        Self::InvalidAesKey(format!("Invalid base64: {}", error))
    }
}

impl From<serde_json::Error> for UeToolError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonError(error.to_string())
    }
}