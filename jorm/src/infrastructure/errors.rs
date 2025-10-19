/// Infrastructure-level errors
/// 
/// These errors represent failures in infrastructure operations
/// such as file I/O, network operations, or database access.

use thiserror::Error;

/// Infrastructure-level errors
#[derive(Error, Debug)]
pub enum InfrastructureError {
    /// File system error
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Parsing error
    #[error("Parsing error: {0}")]
    Parsing(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic infrastructure error
    #[error("Infrastructure error: {0}")]
    Generic(String),
}

/// Result type for infrastructure operations
pub type InfrastructureResult<T> = Result<T, InfrastructureError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = InfrastructureError::FileSystem("test error".to_string());
        assert!(err.to_string().contains("File system error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let infra_err: InfrastructureError = io_err.into();
        assert!(infra_err.to_string().contains("I/O error"));
    }
}
