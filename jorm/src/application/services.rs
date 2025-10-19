/// Application services - Use case implementations
/// 
/// Services orchestrate domain objects and repositories to implement
/// business use cases. They are the entry points for the application.

use super::dto::*;
use super::errors::{ApplicationError, ApplicationResult};
use crate::domain::*;
use async_trait::async_trait;
use std::sync::Arc;

/// Service for managing DAGs
/// 
/// Provides high-level operations for creating, retrieving, and
/// managing DAG definitions.
pub struct DagService {
    /// Repository for persisting DAGs
    dag_repository: Arc<dyn DagRepository>,
}

impl DagService {
    /// Creates a new DAG service
    /// 
    /// # Arguments
    /// * `dag_repository` - Repository for DAG persistence
    /// 
    /// # Returns
    /// * `DagService` - New service instance
    pub fn new(dag_repository: Arc<dyn DagRepository>) -> Self {
        Self { dag_repository }
    }

    /// Creates a new DAG from a request
    /// 
    /// # Arguments
    /// * `request` - DAG creation request
    /// 
    /// # Returns
    /// * `ApplicationResult<DagResponse>` - Created DAG information
    pub async fn create_dag(&self, request: CreateDagRequest) -> ApplicationResult<DagResponse> {
        // Convert request to domain objects
        let dag_id = DagId::new(request.id)?;
        let mut dag = Dag::new(dag_id, request.name);

        if let Some(desc) = request.description {
            dag.set_description(desc);
        }

        // Add tasks
        for task_req in request.tasks {
            let task = self.create_task_from_request(task_req)?;
            dag.add_task(task)?;
        }

        // Add dependencies
        for dep_req in request.dependencies {
            let task_id = TaskId::new(dep_req.task_id)?;
            let depends_on = TaskId::new(dep_req.depends_on)?;
            dag.add_dependency(task_id, depends_on)?;
        }

        // Validate the DAG
        dag.validate()?;

        // Persist the DAG
        self.dag_repository.save(&dag).await?;

        // Return response
        Ok(self.dag_to_response(&dag))
    }

    /// Retrieves a DAG by its identifier
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG identifier
    /// 
    /// # Returns
    /// * `ApplicationResult<DagResponse>` - DAG information if found
    pub async fn get_dag(&self, dag_id: &str) -> ApplicationResult<DagResponse> {
        let id = DagId::new(dag_id)?;
        
        let dag = self.dag_repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                resource_type: "DAG".to_string(),
                id: dag_id.to_string(),
            })?;

        Ok(self.dag_to_response(&dag))
    }

    /// Lists all DAGs
    /// 
    /// # Returns
    /// * `ApplicationResult<Vec<DagResponse>>` - List of all DAGs
    pub async fn list_dags(&self) -> ApplicationResult<Vec<DagResponse>> {
        let dags = self.dag_repository.find_all().await?;
        Ok(dags.iter().map(|dag| self.dag_to_response(dag)).collect())
    }

    /// Deletes a DAG
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG identifier
    /// 
    /// # Returns
    /// * `ApplicationResult<bool>` - True if deleted, false if not found
    pub async fn delete_dag(&self, dag_id: &str) -> ApplicationResult<bool> {
        let id = DagId::new(dag_id)?;
        Ok(self.dag_repository.delete(&id).await?)
    }

    /// Validates a DAG
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG identifier
    /// 
    /// # Returns
    /// * `ApplicationResult<Vec<String>>` - List of validation errors (empty if valid)
    pub async fn validate_dag(&self, dag_id: &str) -> ApplicationResult<Vec<String>> {
        let id = DagId::new(dag_id)?;
        
        let dag = self.dag_repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                resource_type: "DAG".to_string(),
                id: dag_id.to_string(),
            })?;

        Ok(DagValidator::validate(&dag)?)
    }

    /// Helper to create a task from a request
    fn create_task_from_request(&self, request: CreateTaskRequest) -> ApplicationResult<Task> {
        let task_id = TaskId::new(request.id)?;
        let config = self.convert_task_config(request.config);
        
        Ok(Task::new(
            task_id,
            request.name,
            request.task_type,
            config,
        ))
    }

    /// Helper to convert task config request to domain config
    fn convert_task_config(&self, config: TaskConfigRequest) -> TaskConfig {
        match config {
            TaskConfigRequest::Shell { command, working_dir } => {
                TaskConfig::Shell { command, working_dir }
            }
            TaskConfigRequest::Python { script, args } => {
                TaskConfig::Python { script, args }
            }
            TaskConfigRequest::Http { method, url, headers, body } => {
                TaskConfig::Http { method, url, headers, body }
            }
            TaskConfigRequest::File { operation, source, destination } => {
                TaskConfig::File { operation, source, destination }
            }
        }
    }

    /// Helper to convert DAG to response
    fn dag_to_response(&self, dag: &Dag) -> DagResponse {
        DagResponse {
            id: dag.id().to_string(),
            name: dag.name().to_string(),
            description: dag.description().map(|s| s.to_string()),
            task_count: dag.task_count(),
            dependency_count: dag.dependencies().len(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}


/// Service for executing DAGs
/// 
/// Orchestrates DAG execution including task scheduling, dependency
/// resolution, and result tracking.
pub struct ExecutionService {
    /// Repository for DAG definitions
    dag_repository: Arc<dyn DagRepository>,
    /// Repository for execution state
    execution_repository: Arc<dyn ExecutionRepository>,
}

impl ExecutionService {
    /// Creates a new execution service
    /// 
    /// # Arguments
    /// * `dag_repository` - Repository for DAG definitions
    /// * `execution_repository` - Repository for execution state
    /// 
    /// # Returns
    /// * `ExecutionService` - New service instance
    pub fn new(
        dag_repository: Arc<dyn DagRepository>,
        execution_repository: Arc<dyn ExecutionRepository>,
    ) -> Self {
        Self {
            dag_repository,
            execution_repository,
        }
    }

    /// Executes a DAG
    /// 
    /// # Arguments
    /// * `request` - Execution request
    /// 
    /// # Returns
    /// * `ApplicationResult<ExecutionResponse>` - Execution result
    pub async fn execute_dag(
        &self,
        request: ExecuteDagRequest,
    ) -> ApplicationResult<ExecutionResponse> {
        // Retrieve the DAG
        let dag_id = DagId::new(&request.dag_id)?;
        let dag = self.dag_repository
            .find_by_id(&dag_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                resource_type: "DAG".to_string(),
                id: request.dag_id.clone(),
            })?;

        // Validate the DAG
        dag.validate()?;

        // Create execution record
        let execution_id = ExecutionId::new();
        self.execution_repository
            .create_execution(&dag_id, &execution_id)
            .await?;

        // Resolve execution order
        let execution_order = DependencyResolver::resolve_execution_order(&dag)?;

        // Execute tasks (simplified - actual execution would be more complex)
        let started_at = chrono::Utc::now();
        let mut task_results = std::collections::HashMap::new();

        for task_id in execution_order {
            if let Some(task) = dag.tasks().get(&task_id) {
                // In a real implementation, this would execute the task
                // For now, we just record it as successful
                let result = TaskResultResponse {
                    task_id: task_id.to_string(),
                    status: "Success".to_string(),
                    stdout: Some("Task executed successfully".to_string()),
                    stderr: None,
                    exit_code: Some(0),
                    error_message: None,
                };
                task_results.insert(task_id.to_string(), result);
            }
        }

        Ok(ExecutionResponse {
            execution_id: execution_id.to_string(),
            dag_id: request.dag_id,
            status: "Success".to_string(),
            started_at: started_at.to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            task_results,
        })
    }

    /// Gets the status of an execution
    /// 
    /// # Arguments
    /// * `execution_id` - The execution identifier
    /// 
    /// # Returns
    /// * `ApplicationResult<ExecutionResponse>` - Execution status
    pub async fn get_execution_status(
        &self,
        execution_id: &str,
    ) -> ApplicationResult<ExecutionResponse> {
        let id = ExecutionId::from_str(execution_id)?;
        
        let state = self.execution_repository
            .get_execution_state(&id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                resource_type: "Execution".to_string(),
                id: execution_id.to_string(),
            })?;

        // Convert state to response
        let task_results = state.task_states
            .iter()
            .map(|(task_id, task)| {
                (
                    task_id.to_string(),
                    TaskResultResponse {
                        task_id: task_id.to_string(),
                        status: format!("{:?}", task.status()),
                        stdout: None,
                        stderr: None,
                        exit_code: None,
                        error_message: None,
                    },
                )
            })
            .collect();

        Ok(ExecutionResponse {
            execution_id: execution_id.to_string(),
            dag_id: state.dag_id.to_string(),
            status: "Running".to_string(),
            started_at: state.started_at.to_rfc3339(),
            completed_at: state.completed_at.map(|dt| dt.to_rfc3339()),
            task_results,
        })
    }

    /// Lists all executions for a DAG
    /// 
    /// # Arguments
    /// * `dag_id` - The DAG identifier
    /// 
    /// # Returns
    /// * `ApplicationResult<Vec<String>>` - List of execution IDs
    pub async fn list_executions(&self, dag_id: &str) -> ApplicationResult<Vec<String>> {
        let id = DagId::new(dag_id)?;
        let executions = self.execution_repository.list_executions(&id).await?;
        Ok(executions.iter().map(|e| e.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock DAG repository for testing
    struct MockDagRepository {
        dags: std::sync::Mutex<HashMap<String, Dag>>,
    }

    impl MockDagRepository {
        fn new() -> Self {
            Self {
                dags: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl DagRepository for MockDagRepository {
        async fn save(&self, dag: &Dag) -> DomainResult<()> {
            let mut dags = self.dags.lock().unwrap();
            dags.insert(dag.id().to_string(), dag.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &DagId) -> DomainResult<Option<Dag>> {
            let dags = self.dags.lock().unwrap();
            Ok(dags.get(&id.to_string()).cloned())
        }

        async fn find_all(&self) -> DomainResult<Vec<Dag>> {
            let dags = self.dags.lock().unwrap();
            Ok(dags.values().cloned().collect())
        }

        async fn delete(&self, id: &DagId) -> DomainResult<bool> {
            let mut dags = self.dags.lock().unwrap();
            Ok(dags.remove(&id.to_string()).is_some())
        }

        async fn exists(&self, id: &DagId) -> DomainResult<bool> {
            let dags = self.dags.lock().unwrap();
            Ok(dags.contains_key(&id.to_string()))
        }
    }

    /// Test creating a DAG through the service
    #[tokio::test]
    async fn test_create_dag() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        let request = CreateDagRequest {
            id: "test_dag".to_string(),
            name: "Test DAG".to_string(),
            description: Some("A test DAG".to_string()),
            tasks: vec![CreateTaskRequest {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                task_type: "shell".to_string(),
                config: TaskConfigRequest::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            }],
            dependencies: vec![],
        };

        let response = service.create_dag(request).await.unwrap();
        assert_eq!(response.id, "test_dag");
        assert_eq!(response.name, "Test DAG");
        assert_eq!(response.task_count, 1);
    }

    /// Test retrieving a DAG
    #[tokio::test]
    async fn test_get_dag() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        // Create a DAG first
        let request = CreateDagRequest {
            id: "test_dag".to_string(),
            name: "Test DAG".to_string(),
            description: None,
            tasks: vec![CreateTaskRequest {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                task_type: "shell".to_string(),
                config: TaskConfigRequest::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            }],
            dependencies: vec![],
        };

        service.create_dag(request).await.unwrap();

        // Retrieve it
        let response = service.get_dag("test_dag").await.unwrap();
        assert_eq!(response.id, "test_dag");
        assert_eq!(response.task_count, 1);
    }

    /// Test getting non-existent DAG returns error
    #[tokio::test]
    async fn test_get_dag_not_found() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        let result = service.get_dag("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApplicationError::NotFound { .. }));
    }

    /// Test listing DAGs
    #[tokio::test]
    async fn test_list_dags() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        // Create multiple DAGs
        for i in 1..=3 {
            let request = CreateDagRequest {
                id: format!("dag{}", i),
                name: format!("DAG {}", i),
                description: None,
                tasks: vec![CreateTaskRequest {
                    id: "task1".to_string(),
                    name: "Task 1".to_string(),
                    task_type: "shell".to_string(),
                    config: TaskConfigRequest::Shell {
                        command: "echo hello".to_string(),
                        working_dir: None,
                    },
                }],
                dependencies: vec![],
            };
            service.create_dag(request).await.unwrap();
        }

        let dags = service.list_dags().await.unwrap();
        assert_eq!(dags.len(), 3);
    }

    /// Test deleting a DAG
    #[tokio::test]
    async fn test_delete_dag() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        // Create a DAG
        let request = CreateDagRequest {
            id: "test_dag".to_string(),
            name: "Test DAG".to_string(),
            description: None,
            tasks: vec![CreateTaskRequest {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                task_type: "shell".to_string(),
                config: TaskConfigRequest::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            }],
            dependencies: vec![],
        };
        service.create_dag(request).await.unwrap();

        // Delete it
        let deleted = service.delete_dag("test_dag").await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let result = service.get_dag("test_dag").await;
        assert!(result.is_err());
    }

    /// Test validating a DAG
    #[tokio::test]
    async fn test_validate_dag() {
        let repo = Arc::new(MockDagRepository::new());
        let service = DagService::new(repo);

        // Create a valid DAG
        let request = CreateDagRequest {
            id: "test_dag".to_string(),
            name: "Test DAG".to_string(),
            description: None,
            tasks: vec![CreateTaskRequest {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                task_type: "shell".to_string(),
                config: TaskConfigRequest::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            }],
            dependencies: vec![],
        };
        service.create_dag(request).await.unwrap();

        // Validate it
        let errors = service.validate_dag("test_dag").await.unwrap();
        assert!(errors.is_empty());
    }
}
