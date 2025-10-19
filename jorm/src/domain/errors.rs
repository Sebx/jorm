/// Domain-specific errors
/// 
/// These errors represent business rule violations and domain constraints.
/// They should be independent of infrastructure concerns.

use thiserror::Error;

/// Domain-level errors representing business rule violations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    /// Task identifier is invalid or empty
    #[error("Invalid task identifier: {0}")]
    InvalidTaskId(String),

    /// DAG identifier is invalid or empty
    #[error("Invalid DAG identifier: {0}")]
    InvalidDagId(String),

    /// Execution identifier is invalid
    #[error("Invalid execution identifier: {0}")]
    InvalidExecutionId(String),

    /// Cron expression is malformed
    #[error("Invalid cron expression: {0}")]
    InvalidCronExpression(String),

    /// Circular dependency detected in DAG
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Task not found in DAG
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Dependency references non-existent task
    #[error("Invalid dependency: task '{0}' depends on non-existent task '{1}'")]
    InvalidDependency(String, String),

    /// DAG has no tasks defined
    #[error("DAG must contain at least one task")]
    EmptyDag,

    /// Task configuration is invalid
    #[error("Invalid task configuration: {0}")]
    InvalidTaskConfiguration(String),

    /// Schedule configuration is invalid
    #[error("Invalid schedule configuration: {0}")]
    InvalidScheduleConfiguration(String),

    /// Execution state transition is invalid
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    /// Maximum retry count exceeded
    #[error("Maximum retry count ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    /// Timeout exceeded
    #[error("Operation timeout exceeded: {0}s")]
    TimeoutExceeded(u64),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that domain errors can be created and formatted correctly
    #[test]
    fn test_domain_error_creation() {
        let error = DomainError::InvalidTaskId("".to_string());
        assert_eq!(error.to_string(), "Invalid task identifier: ");

        let error = DomainError::CircularDependency("A -> B -> A".to_string());
        assert!(error.to_string().contains("Circular dependency"));

        let error = DomainError::InvalidStateTransition {
            from: "Running".to_string(),
            to: "Pending".to_string(),
        };
        assert!(error.to_string().contains("Invalid state transition"));
    }

    /// Test that domain errors are cloneable
    #[test]
    fn test_domain_error_clone() {
        let error = DomainError::TaskNotFound("task1".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    /// Test that domain errors can be compared
    #[test]
    fn test_domain_error_equality() {
        let error1 = DomainError::EmptyDag;
        let error2 = DomainError::EmptyDag;
        assert_eq!(error1, error2);

        let error3 = DomainError::TaskNotFound("task1".to_string());
        assert_ne!(error1, error3);
    }
}
