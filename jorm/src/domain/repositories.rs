/// Repository interfaces - Abstractions for data persistence
/// 
/// Repositories define the contract for storing and retrieving domain entities.
/// They are interfaces (traits) that will be implemented by infrastructure layer.

use super::entities::{Dag, Task};
use super::errors::DomainResult;
use super::value_objects::{DagId, ExecutionId, TaskId};
use async_trait::async_trait;

/// Repository for persisting and retrieving DAGs
/// 
/// This trait defines the contract for DAG storage. Implementations
/// can use databases, file systems, or any other storage mechanism.
#[async_trait]
pub trait DagRepository: Send + Sync {
    /// Saves a DAG to the repository
    /// 
    /// # Arguments
    /// * `dag` - The DAG to save
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if saved successfully
    async fn save(&self, dag: &Dag) -> DomainResult<()>;

    /// Retrieves a DAG by its identifier
    /// 
    /// # Arguments
    /// * `id` - The DAG identifier
    /// 
    /// # Returns
    /// * `DomainResult<Option<Dag>>` - The DAG if found, None otherwise
    async fn find_by_id(&self, id: &DagId) -> DomainResult<Option<Dag>>;

    /// Retrieves all DAGs
    /// 
    /// # Returns
    /// * `DomainResult<Vec<Dag>>` - List of all DAGs
    async fn find_all(&self) -> DomainResult<Vec<Dag>>;

    /// Deletes a DAG by its identifier
    /// 
    /// # Arguments
    /// * `id` - The DAG identifier
    /// 
    /// # Returns
    /// * `DomainResult<bool>` - True if deleted, false if not found
    async fn delete(&self, id: &DagId) -> DomainResult<bool>;

    /// Checks if a DAG exists
    /// 
    /// # Arguments
    /// * `id` - The DAG identifier
    /// 
    /// # Returns
    /// * `DomainResult<bool>` - True if exists, false otherwise
    async fn exists(&self, id: &DagId) -> DomainResult<bool>;
}

/// Repository for persisting execution state
/// 
/// Tracks the state of DAG executions including task results,
/// timing information, and execution metadata.
#[async_trait]
pub trait ExecutionRepository: Send + Sync {
    /// Creates a new execution record
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG being executed
    /// * `execution_id` - Unique execution identifier
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if created successfully
    async fn create_execution(
        &self,
        dag_id: &DagId,
        execution_id: &ExecutionId,
    ) -> DomainResult<()>;

    /// Updates the status of a task within an execution
    /// 
    /// # Arguments
    /// * `execution_id` - The execution identifier
    /// * `task_id` - The task identifier
    /// * `task` - The updated task state
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if updated successfully
    async fn update_task_status(
        &self,
        execution_id: &ExecutionId,
        task_id: &TaskId,
        task: &Task,
    ) -> DomainResult<()>;

    /// Retrieves the current state of an execution
    /// 
    /// # Arguments
    /// * `execution_id` - The execution identifier
    /// 
    /// # Returns
    /// * `DomainResult<Option<ExecutionState>>` - The execution state if found
    async fn get_execution_state(
        &self,
        execution_id: &ExecutionId,
    ) -> DomainResult<Option<ExecutionState>>;

    /// Lists all executions for a DAG
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG identifier
    /// 
    /// # Returns
    /// * `DomainResult<Vec<ExecutionId>>` - List of execution IDs
    async fn list_executions(&self, dag_id: &DagId) -> DomainResult<Vec<ExecutionId>>;
}

/// Represents the state of a DAG execution
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// The execution identifier
    pub execution_id: ExecutionId,
    /// The DAG being executed
    pub dag_id: DagId,
    /// Current state of all tasks
    pub task_states: std::collections::HashMap<TaskId, Task>,
    /// When the execution started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the execution completed (if finished)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that ExecutionState can be created
    #[test]
    fn test_execution_state_creation() {
        let execution_id = ExecutionId::new();
        let dag_id = DagId::new("test_dag").unwrap();
        let started_at = chrono::Utc::now();

        let state = ExecutionState {
            execution_id: execution_id.clone(),
            dag_id: dag_id.clone(),
            task_states: std::collections::HashMap::new(),
            started_at,
            completed_at: None,
        };

        assert_eq!(state.execution_id, execution_id);
        assert_eq!(state.dag_id, dag_id);
        assert!(state.task_states.is_empty());
        assert!(state.completed_at.is_none());
    }

    /// Test that ExecutionState can be cloned
    #[test]
    fn test_execution_state_clone() {
        let execution_id = ExecutionId::new();
        let dag_id = DagId::new("test_dag").unwrap();
        let started_at = chrono::Utc::now();

        let state = ExecutionState {
            execution_id,
            dag_id,
            task_states: std::collections::HashMap::new(),
            started_at,
            completed_at: None,
        };

        let cloned = state.clone();
        assert_eq!(state.execution_id, cloned.execution_id);
        assert_eq!(state.dag_id, cloned.dag_id);
    }
}
