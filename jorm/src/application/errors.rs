/// Application-level errors
/// 
/// These errors represent failures in application use cases,
/// wrapping domain errors and adding application context.

use thiserror::Error;
use crate::domain::DomainError;

/// Application-level errors
#[derive(Error, Debug)]
pub enum ApplicationError {
    /// Domain rule violation
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    /// Resource not found
    #[error("Resource not found: {resource_type} with id '{id}'")]
    NotFound {
        resource_type: String,
        id: String,
    },

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Infrastructure error
    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    /// Concurrent modification detected
    #[error("Concurrent modification detected for {resource_type} '{id}'")]
    ConcurrentModification {
        resource_type: String,
        id: String,
    },

    /// Operation not allowed
    #[error("Operation not allowed: {0}")]
    NotAllowed(String),
}

/// Result type for application operations
pub type ApplicationResult<T> = Result<T, ApplicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test creating application errors
    #[test]
    fn test_application_error_creation() {
        let error = ApplicationError::NotFound {
            resource_type: "DAG".to_string(),
            id: "test_dag".to_string(),
        };
        assert!(error.to_string().contains("DAG"));
        assert!(error.to_string().contains("test_dag"));
    }

    /// Test domain error conversion
    #[test]
    fn test_domain_error_conversion() {
        let domain_error = DomainError::EmptyDag;
        let app_error: ApplicationError = domain_error.into();
        assert!(matches!(app_error, ApplicationError::Domain(_)));
    }
}
