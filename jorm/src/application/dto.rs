/// Data Transfer Objects (DTOs)
/// 
/// DTOs are used to transfer data between layers without exposing
/// internal domain structures. They provide a stable API contract.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to create a new DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDagRequest {
    /// Unique identifier for the DAG
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Tasks to include in the DAG
    pub tasks: Vec<CreateTaskRequest>,
    /// Dependencies between tasks
    pub dependencies: Vec<DependencyRequest>,
}

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Unique identifier for the task
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of task (shell, python, http, file)
    pub task_type: String,
    /// Task-specific configuration
    pub config: TaskConfigRequest,
}

/// Task configuration request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskConfigRequest {
    /// Shell command execution
    Shell {
        command: String,
        working_dir: Option<String>,
    },
    /// Python script execution
    Python {
        script: String,
        args: Vec<String>,
    },
    /// HTTP request
    Http {
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
    /// File operation
    File {
        operation: String,
        source: Option<String>,
        destination: Option<String>,
    },
}

/// Dependency between tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRequest {
    /// Task that has the dependency
    pub task_id: String,
    /// Task that must complete first
    pub depends_on: String,
}

/// Response containing DAG information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagResponse {
    /// DAG identifier
    pub id: String,
    /// DAG name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Number of tasks
    pub task_count: usize,
    /// Number of dependencies
    pub dependency_count: usize,
    /// When the DAG was created
    pub created_at: String,
}

/// Request to execute a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteDagRequest {
    /// DAG identifier to execute
    pub dag_id: String,
    /// Optional execution parameters
    pub parameters: Option<HashMap<String, String>>,
}

/// Response from DAG execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    /// Unique execution identifier
    pub execution_id: String,
    /// DAG that was executed
    pub dag_id: String,
    /// Execution status
    pub status: String,
    /// When execution started
    pub started_at: String,
    /// When execution completed (if finished)
    pub completed_at: Option<String>,
    /// Task results
    pub task_results: HashMap<String, TaskResultResponse>,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultResponse {
    /// Task identifier
    pub task_id: String,
    /// Execution status
    pub status: String,
    /// Standard output
    pub stdout: Option<String>,
    /// Standard error
    pub stderr: Option<String>,
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test creating a CreateDagRequest
    #[test]
    fn test_create_dag_request() {
        let request = CreateDagRequest {
            id: "test_dag".to_string(),
            name: "Test DAG".to_string(),
            description: Some("A test DAG".to_string()),
            tasks: vec![],
            dependencies: vec![],
        };

        assert_eq!(request.id, "test_dag");
        assert_eq!(request.name, "Test DAG");
        assert!(request.description.is_some());
    }

    /// Test serializing and deserializing DTOs
    #[test]
    fn test_dto_serialization() {
        let request = CreateTaskRequest {
            id: "task1".to_string(),
            name: "Task 1".to_string(),
            task_type: "shell".to_string(),
            config: TaskConfigRequest::Shell {
                command: "echo hello".to_string(),
                working_dir: None,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateTaskRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.id, deserialized.id);
        assert_eq!(request.name, deserialized.name);
    }

    /// Test DagResponse creation
    #[test]
    fn test_dag_response() {
        let response = DagResponse {
            id: "dag1".to_string(),
            name: "DAG 1".to_string(),
            description: None,
            task_count: 5,
            dependency_count: 3,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(response.task_count, 5);
        assert_eq!(response.dependency_count, 3);
    }
}
