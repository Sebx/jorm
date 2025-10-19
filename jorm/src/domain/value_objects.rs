/// Value objects - Immutable domain concepts with validation
/// 
/// Value objects represent concepts that are defined by their attributes
/// rather than identity. They are immutable and self-validating.

use super::errors::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Unique identifier for a task within a DAG
/// 
/// TaskId must be non-empty and contain only alphanumeric characters,
/// underscores, and hyphens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    /// Creates a new TaskId with validation
    /// 
    /// # Arguments
    /// * `id` - The task identifier string
    /// 
    /// # Returns
    /// * `DomainResult<TaskId>` - Validated TaskId or error
    /// 
    /// # Errors
    /// * `DomainError::InvalidTaskId` - If id is empty or contains invalid characters
    pub fn new(id: impl Into<String>) -> DomainResult<Self> {
        let id = id.into();
        
        if id.is_empty() {
            return Err(DomainError::InvalidTaskId("Task ID cannot be empty".to_string()));
        }

        if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(DomainError::InvalidTaskId(
                format!("Task ID '{}' contains invalid characters", id)
            ));
        }

        Ok(Self(id))
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a DAG
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DagId(String);

impl DagId {
    /// Creates a new DagId with validation
    /// 
    /// # Arguments
    /// * `id` - The DAG identifier string
    /// 
    /// # Returns
    /// * `DomainResult<DagId>` - Validated DagId or error
    pub fn new(id: impl Into<String>) -> DomainResult<Self> {
        let id = id.into();
        
        if id.is_empty() {
            return Err(DomainError::InvalidDagId("DAG ID cannot be empty".to_string()));
        }

        Ok(Self(id))
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for DagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a DAG execution instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(Uuid);

impl ExecutionId {
    /// Creates a new random ExecutionId
    /// 
    /// # Returns
    /// * `ExecutionId` - A new unique execution identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an ExecutionId from a UUID
    /// 
    /// # Arguments
    /// * `uuid` - The UUID to wrap
    /// 
    /// # Returns
    /// * `ExecutionId` - Execution identifier wrapping the UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Creates an ExecutionId from a string representation
    /// 
    /// # Arguments
    /// * `s` - String representation of a UUID
    /// 
    /// # Returns
    /// * `DomainResult<ExecutionId>` - Parsed ExecutionId or error
    pub fn from_str(s: &str) -> DomainResult<Self> {
        Uuid::parse_str(s)
            .map(Self::from_uuid)
            .map_err(|e| DomainError::InvalidExecutionId(e.to_string()))
    }

    /// Returns the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consumes self and returns the inner UUID
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Cron expression for scheduling
/// 
/// Validates cron expressions according to standard cron syntax
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronExpression(String);

impl CronExpression {
    /// Creates a new CronExpression with validation
    /// 
    /// # Arguments
    /// * `expression` - The cron expression string
    /// 
    /// # Returns
    /// * `DomainResult<CronExpression>` - Validated cron expression or error
    /// 
    /// # Errors
    /// * `DomainError::InvalidCronExpression` - If expression is invalid
    pub fn new(expression: impl Into<String>) -> DomainResult<Self> {
        let expression = expression.into();
        
        if expression.is_empty() {
            return Err(DomainError::InvalidCronExpression(
                "Cron expression cannot be empty".to_string()
            ));
        }

        // Validate cron expression format
        if let Err(e) = cron::Schedule::from_str(&expression) {
            return Err(DomainError::InvalidCronExpression(
                format!("Invalid cron syntax: {}", e)
            ));
        }

        Ok(Self(expression))
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for CronExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TaskId Tests
    
    /// Test creating a valid TaskId
    #[test]
    fn test_task_id_valid() {
        let id = TaskId::new("task_1").unwrap();
        assert_eq!(id.as_str(), "task_1");
        assert_eq!(id.to_string(), "task_1");
    }

    /// Test that empty TaskId is rejected
    #[test]
    fn test_task_id_empty() {
        let result = TaskId::new("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidTaskId(_)));
    }

    /// Test that TaskId with invalid characters is rejected
    #[test]
    fn test_task_id_invalid_chars() {
        let result = TaskId::new("task@123");
        assert!(result.is_err());
        
        let result = TaskId::new("task 123");
        assert!(result.is_err());
    }

    /// Test TaskId with valid characters
    #[test]
    fn test_task_id_valid_chars() {
        assert!(TaskId::new("task123").is_ok());
        assert!(TaskId::new("task_123").is_ok());
        assert!(TaskId::new("task-123").is_ok());
        assert!(TaskId::new("Task_123-ABC").is_ok());
    }

    /// Test TaskId equality
    #[test]
    fn test_task_id_equality() {
        let id1 = TaskId::new("task1").unwrap();
        let id2 = TaskId::new("task1").unwrap();
        let id3 = TaskId::new("task2").unwrap();
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    /// Test TaskId can be cloned
    #[test]
    fn test_task_id_clone() {
        let id = TaskId::new("task1").unwrap();
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    // DagId Tests
    
    /// Test creating a valid DagId
    #[test]
    fn test_dag_id_valid() {
        let id = DagId::new("my_dag").unwrap();
        assert_eq!(id.as_str(), "my_dag");
    }

    /// Test that empty DagId is rejected
    #[test]
    fn test_dag_id_empty() {
        let result = DagId::new("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidDagId(_)));
    }

    // ExecutionId Tests
    
    /// Test creating a new ExecutionId
    #[test]
    fn test_execution_id_new() {
        let id1 = ExecutionId::new();
        let id2 = ExecutionId::new();
        
        // Each new ID should be unique
        assert_ne!(id1, id2);
    }

    /// Test ExecutionId from UUID
    #[test]
    fn test_execution_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = ExecutionId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), &uuid);
    }

    /// Test ExecutionId from string
    #[test]
    fn test_execution_id_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = ExecutionId::from_str(uuid_str).unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }

    /// Test invalid ExecutionId from string
    #[test]
    fn test_execution_id_from_str_invalid() {
        let result = ExecutionId::from_str("not-a-uuid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidExecutionId(_)));
    }

    /// Test ExecutionId default
    #[test]
    fn test_execution_id_default() {
        let id = ExecutionId::default();
        assert!(!id.to_string().is_empty());
    }

    // CronExpression Tests
    
    /// Test creating a valid CronExpression
    #[test]
    fn test_cron_expression_valid() {
        let expr = CronExpression::new("0 0 0 * * *").unwrap();
        assert_eq!(expr.as_str(), "0 0 0 * * *");
    }

    /// Test various valid cron expressions
    #[test]
    fn test_cron_expression_various_valid() {
        // Cron expressions need 6 fields: sec min hour day month dow
        assert!(CronExpression::new("* * * * * *").is_ok());
        assert!(CronExpression::new("0 0 0 * * *").is_ok());
        assert!(CronExpression::new("0 0 */2 * * *").is_ok());
        assert!(CronExpression::new("0 0 0 1 * *").is_ok());
    }

    /// Test that empty CronExpression is rejected
    #[test]
    fn test_cron_expression_empty() {
        let result = CronExpression::new("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidCronExpression(_)));
    }

    /// Test that invalid CronExpression is rejected
    #[test]
    fn test_cron_expression_invalid() {
        let result = CronExpression::new("invalid cron");
        assert!(result.is_err());
        
        let result = CronExpression::new("* * * *");
        assert!(result.is_err());
    }

    /// Test CronExpression equality
    #[test]
    fn test_cron_expression_equality() {
        let expr1 = CronExpression::new("0 0 0 * * *").unwrap();
        let expr2 = CronExpression::new("0 0 0 * * *").unwrap();
        let expr3 = CronExpression::new("* * * * * *").unwrap();
        
        assert_eq!(expr1, expr2);
        assert_ne!(expr1, expr3);
    }
}
