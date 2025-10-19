/// Domain entities - Core business objects with identity
/// 
/// Entities are objects that have a distinct identity that runs through time
/// and different representations. They are mutable and have a lifecycle.

use super::errors::{DomainError, DomainResult};
use super::value_objects::{DagId, ExecutionId, TaskId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents the execution status of a task or DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Not yet started
    Pending,
    /// Currently executing
    Running,
    /// Completed successfully
    Success,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
    /// Skipped due to conditions
    Skipped,
}

impl ExecutionStatus {
    /// Checks if the status represents a terminal state
    /// 
    /// # Returns
    /// * `bool` - True if status is terminal (Success, Failed, Cancelled, Skipped)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Success
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::Skipped
        )
    }

    /// Checks if the status represents an active state
    /// 
    /// # Returns
    /// * `bool` - True if status is active (Pending, Running)
    pub fn is_active(&self) -> bool {
        matches!(self, ExecutionStatus::Pending | ExecutionStatus::Running)
    }
}

/// Task entity representing a unit of work in a DAG
/// 
/// A task is the fundamental unit of execution. It has an identity (TaskId),
/// configuration, and tracks its execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    id: TaskId,
    /// Human-readable name
    name: String,
    /// Type of task (shell, python, http, file)
    task_type: String,
    /// Task-specific configuration
    config: TaskConfig,
    /// Current execution status
    status: ExecutionStatus,
    /// When the task was created
    created_at: DateTime<Utc>,
    /// When the task was last updated
    updated_at: DateTime<Utc>,
}

impl Task {
    /// Creates a new task with the given parameters
    /// 
    /// # Arguments
    /// * `id` - Unique task identifier
    /// * `name` - Human-readable task name
    /// * `task_type` - Type of task executor to use
    /// * `config` - Task-specific configuration
    /// 
    /// # Returns
    /// * `Task` - New task instance in Pending state
    pub fn new(
        id: TaskId,
        name: String,
        task_type: String,
        config: TaskConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            task_type,
            config,
            status: ExecutionStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the task's unique identifier
    pub fn id(&self) -> &TaskId {
        &self.id
    }

    /// Returns the task's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the task type
    pub fn task_type(&self) -> &str {
        &self.task_type
    }

    /// Returns the task configuration
    pub fn config(&self) -> &TaskConfig {
        &self.config
    }

    /// Returns the current execution status
    pub fn status(&self) -> ExecutionStatus {
        self.status
    }

    /// Updates the task status
    /// 
    /// # Arguments
    /// * `new_status` - The new status to set
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if transition is valid, error otherwise
    pub fn update_status(&mut self, new_status: ExecutionStatus) -> DomainResult<()> {
        // Validate state transition
        if !self.is_valid_transition(new_status) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", new_status),
            });
        }

        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Checks if a status transition is valid
    fn is_valid_transition(&self, new_status: ExecutionStatus) -> bool {
        use ExecutionStatus::*;
        
        match (self.status, new_status) {
            // Can always stay in same state
            (s1, s2) if s1 == s2 => true,
            // From Pending
            (Pending, Running) | (Pending, Cancelled) | (Pending, Skipped) => true,
            // From Running
            (Running, Success) | (Running, Failed) | (Running, Cancelled) => true,
            // Terminal states cannot transition
            (s, _) if s.is_terminal() => false,
            // All other transitions are invalid
            _ => false,
        }
    }
}

/// Task configuration - polymorphic configuration for different task types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskConfig {
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

/// DAG entity representing a directed acyclic graph of tasks
/// 
/// A DAG is a collection of tasks with dependencies between them.
/// It ensures that tasks are executed in the correct order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    /// Unique identifier for this DAG
    id: DagId,
    /// Human-readable name
    name: String,
    /// Description of what this DAG does
    description: Option<String>,
    /// Tasks in this DAG, indexed by TaskId
    tasks: HashMap<TaskId, Task>,
    /// Dependencies between tasks
    dependencies: HashSet<Dependency>,
    /// When the DAG was created
    created_at: DateTime<Utc>,
    /// When the DAG was last updated
    updated_at: DateTime<Utc>,
}

impl Dag {
    /// Creates a new DAG with the given identifier and name
    /// 
    /// # Arguments
    /// * `id` - Unique DAG identifier
    /// * `name` - Human-readable DAG name
    /// 
    /// # Returns
    /// * `Dag` - New DAG instance with no tasks
    pub fn new(id: DagId, name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            tasks: HashMap::new(),
            dependencies: HashSet::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the DAG's unique identifier
    pub fn id(&self) -> &DagId {
        &self.id
    }

    /// Returns the DAG's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the DAG's description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the DAG's description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = Utc::now();
    }

    /// Returns all tasks in the DAG
    pub fn tasks(&self) -> &HashMap<TaskId, Task> {
        &self.tasks
    }

    /// Returns all dependencies in the DAG
    pub fn dependencies(&self) -> &HashSet<Dependency> {
        &self.dependencies
    }

    /// Adds a task to the DAG
    /// 
    /// # Arguments
    /// * `task` - The task to add
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if task was added, error if duplicate
    pub fn add_task(&mut self, task: Task) -> DomainResult<()> {
        let task_id = task.id().clone();
        
        if self.tasks.contains_key(&task_id) {
            return Err(DomainError::ValidationFailed(
                format!("Task with ID '{}' already exists", task_id)
            ));
        }

        self.tasks.insert(task_id, task);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Adds a dependency between two tasks
    /// 
    /// # Arguments
    /// * `task_id` - The task that depends on another
    /// * `depends_on` - The task that must complete first
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if dependency was added, error if invalid
    pub fn add_dependency(&mut self, task_id: TaskId, depends_on: TaskId) -> DomainResult<()> {
        // Validate both tasks exist
        if !self.tasks.contains_key(&task_id) {
            return Err(DomainError::TaskNotFound(task_id.to_string()));
        }
        if !self.tasks.contains_key(&depends_on) {
            return Err(DomainError::InvalidDependency(
                task_id.to_string(),
                depends_on.to_string(),
            ));
        }

        let dependency = Dependency::new(task_id, depends_on);
        
        // Check for circular dependencies before adding
        let mut temp_deps = self.dependencies.clone();
        temp_deps.insert(dependency.clone());
        
        if self.has_cycle_with_deps(&temp_deps) {
            return Err(DomainError::CircularDependency(
                format!("{} -> {}", dependency.task_id(), dependency.depends_on())
            ));
        }

        self.dependencies.insert(dependency);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Checks if the DAG has any tasks
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Returns the number of tasks in the DAG
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Validates the DAG structure
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if DAG is valid, error with details otherwise
    pub fn validate(&self) -> DomainResult<()> {
        // Check for empty DAG
        if self.is_empty() {
            return Err(DomainError::EmptyDag);
        }

        // Check for circular dependencies
        if self.has_cycle_with_deps(&self.dependencies) {
            return Err(DomainError::CircularDependency(
                "Circular dependency detected in DAG".to_string()
            ));
        }

        Ok(())
    }

    /// Checks for circular dependencies using DFS
    fn has_cycle_with_deps(&self, deps: &HashSet<Dependency>) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in self.tasks.keys() {
            if !visited.contains(task_id) {
                if self.has_cycle_dfs(task_id, deps, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        task_id: &TaskId,
        deps: &HashSet<Dependency>,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
    ) -> bool {
        visited.insert(task_id.clone());
        rec_stack.insert(task_id.clone());

        // Find all tasks that depend on this task
        for dep in deps {
            if dep.depends_on() == task_id {
                let dependent = dep.task_id();
                if !visited.contains(dependent) {
                    if self.has_cycle_dfs(dependent, deps, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dependent) {
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        false
    }
}

/// Dependency between two tasks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// The task that has the dependency
    task_id: TaskId,
    /// The task that must complete first
    depends_on: TaskId,
}

impl Dependency {
    /// Creates a new dependency
    /// 
    /// # Arguments
    /// * `task_id` - The task that depends on another
    /// * `depends_on` - The task that must complete first
    /// 
    /// # Returns
    /// * `Dependency` - New dependency instance
    pub fn new(task_id: TaskId, depends_on: TaskId) -> Self {
        Self {
            task_id,
            depends_on,
        }
    }

    /// Returns the task that has the dependency
    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }

    /// Returns the task that must complete first
    pub fn depends_on(&self) -> &TaskId {
        &self.depends_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ExecutionStatus Tests

    /// Test that terminal statuses are correctly identified
    #[test]
    fn test_execution_status_is_terminal() {
        assert!(ExecutionStatus::Success.is_terminal());
        assert!(ExecutionStatus::Failed.is_terminal());
        assert!(ExecutionStatus::Cancelled.is_terminal());
        assert!(ExecutionStatus::Skipped.is_terminal());
        assert!(!ExecutionStatus::Pending.is_terminal());
        assert!(!ExecutionStatus::Running.is_terminal());
    }

    /// Test that active statuses are correctly identified
    #[test]
    fn test_execution_status_is_active() {
        assert!(ExecutionStatus::Pending.is_active());
        assert!(ExecutionStatus::Running.is_active());
        assert!(!ExecutionStatus::Success.is_active());
        assert!(!ExecutionStatus::Failed.is_active());
    }

    // Task Tests

    /// Test creating a new task
    #[test]
    fn test_task_creation() {
        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let task = Task::new(
            task_id.clone(),
            "Test Task".to_string(),
            "shell".to_string(),
            config,
        );

        assert_eq!(task.id(), &task_id);
        assert_eq!(task.name(), "Test Task");
        assert_eq!(task.task_type(), "shell");
        assert_eq!(task.status(), ExecutionStatus::Pending);
    }

    /// Test valid task status transitions
    #[test]
    fn test_task_status_transitions_valid() {
        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let mut task = Task::new(
            task_id,
            "Test Task".to_string(),
            "shell".to_string(),
            config,
        );

        // Pending -> Running
        assert!(task.update_status(ExecutionStatus::Running).is_ok());
        assert_eq!(task.status(), ExecutionStatus::Running);

        // Running -> Success
        assert!(task.update_status(ExecutionStatus::Success).is_ok());
        assert_eq!(task.status(), ExecutionStatus::Success);
    }

    /// Test invalid task status transitions
    #[test]
    fn test_task_status_transitions_invalid() {
        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let mut task = Task::new(
            task_id,
            "Test Task".to_string(),
            "shell".to_string(),
            config,
        );

        // Pending -> Success (invalid, must go through Running)
        let result = task.update_status(ExecutionStatus::Success);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidStateTransition { .. }));
    }

    /// Test that terminal states cannot transition
    #[test]
    fn test_task_terminal_state_no_transition() {
        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let mut task = Task::new(
            task_id,
            "Test Task".to_string(),
            "shell".to_string(),
            config,
        );

        // Move to terminal state
        task.update_status(ExecutionStatus::Running).unwrap();
        task.update_status(ExecutionStatus::Success).unwrap();

        // Try to transition from terminal state
        let result = task.update_status(ExecutionStatus::Running);
        assert!(result.is_err());
    }

    // DAG Tests

    /// Test creating a new DAG
    #[test]
    fn test_dag_creation() {
        let dag_id = DagId::new("my_dag").unwrap();
        let dag = Dag::new(dag_id.clone(), "My DAG".to_string());

        assert_eq!(dag.id(), &dag_id);
        assert_eq!(dag.name(), "My DAG");
        assert!(dag.is_empty());
        assert_eq!(dag.task_count(), 0);
    }

    /// Test adding tasks to DAG
    #[test]
    fn test_dag_add_task() {
        let dag_id = DagId::new("my_dag").unwrap();
        let mut dag = Dag::new(dag_id, "My DAG".to_string());

        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        let task = Task::new(task_id, "Task 1".to_string(), "shell".to_string(), config);

        assert!(dag.add_task(task).is_ok());
        assert_eq!(dag.task_count(), 1);
        assert!(!dag.is_empty());
    }

    /// Test that duplicate tasks are rejected
    #[test]
    fn test_dag_add_duplicate_task() {
        let dag_id = DagId::new("my_dag").unwrap();
        let mut dag = Dag::new(dag_id, "My DAG".to_string());

        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        let task1 = Task::new(task_id.clone(), "Task 1".to_string(), "shell".to_string(), config.clone());
        let task2 = Task::new(task_id, "Task 1 Duplicate".to_string(), "shell".to_string(), config);

        assert!(dag.add_task(task1).is_ok());
        let result = dag.add_task(task2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::ValidationFailed(_)));
    }

    /// Test adding valid dependencies
    #[test]
    fn test_dag_add_dependency() {
        let dag_id = DagId::new("my_dag").unwrap();
        let mut dag = Dag::new(dag_id, "My DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let task1 = Task::new(task1_id.clone(), "Task 1".to_string(), "shell".to_string(), config.clone());
        let task2 = Task::new(task2_id.clone(), "Task 2".to_string(), "shell".to_string(), config);

        dag.add_task(task1).unwrap();
        dag.add_task(task2).unwrap();

        // task2 depends on task1
        assert!(dag.add_dependency(task2_id, task1_id).is_ok());
        assert_eq!(dag.dependencies().len(), 1);
    }

    /// Test that circular dependencies are detected
    #[test]
    fn test_dag_circular_dependency() {
        let dag_id = DagId::new("my_dag").unwrap();
        let mut dag = Dag::new(dag_id, "My DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        
        let task1 = Task::new(task1_id.clone(), "Task 1".to_string(), "shell".to_string(), config.clone());
        let task2 = Task::new(task2_id.clone(), "Task 2".to_string(), "shell".to_string(), config);

        dag.add_task(task1).unwrap();
        dag.add_task(task2).unwrap();

        // task2 depends on task1
        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();

        // task1 depends on task2 (creates cycle)
        let result = dag.add_dependency(task1_id, task2_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::CircularDependency(_)));
    }

    /// Test that empty DAG validation fails
    #[test]
    fn test_dag_validate_empty() {
        let dag_id = DagId::new("my_dag").unwrap();
        let dag = Dag::new(dag_id, "My DAG".to_string());

        let result = dag.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::EmptyDag));
    }

    /// Test that valid DAG passes validation
    #[test]
    fn test_dag_validate_success() {
        let dag_id = DagId::new("my_dag").unwrap();
        let mut dag = Dag::new(dag_id, "My DAG".to_string());

        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo hello".to_string(),
            working_dir: None,
        };
        let task = Task::new(task_id, "Task 1".to_string(), "shell".to_string(), config);

        dag.add_task(task).unwrap();

        assert!(dag.validate().is_ok());
    }

    // Dependency Tests

    /// Test creating a dependency
    #[test]
    fn test_dependency_creation() {
        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        
        let dep = Dependency::new(task2_id.clone(), task1_id.clone());

        assert_eq!(dep.task_id(), &task2_id);
        assert_eq!(dep.depends_on(), &task1_id);
    }

    /// Test dependency equality
    #[test]
    fn test_dependency_equality() {
        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        
        let dep1 = Dependency::new(task2_id.clone(), task1_id.clone());
        let dep2 = Dependency::new(task2_id.clone(), task1_id.clone());
        let dep3 = Dependency::new(task1_id, task2_id);

        assert_eq!(dep1, dep2);
        assert_ne!(dep1, dep3);
    }
}
