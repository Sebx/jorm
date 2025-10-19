/// Dependency Injection Container
/// 
/// Provides a centralized way to manage dependencies and create
/// service instances with their required dependencies.

use super::services::{DagService, ExecutionService};
use crate::domain::{DagRepository, ExecutionRepository};
use std::sync::Arc;

/// Service container for dependency injection
/// 
/// The container holds references to all infrastructure dependencies
/// and provides factory methods for creating application services.
pub struct ServiceContainer {
    /// Repository for DAG persistence
    dag_repository: Arc<dyn DagRepository>,
    /// Repository for execution state
    execution_repository: Arc<dyn ExecutionRepository>,
}

impl ServiceContainer {
    /// Creates a new service container
    /// 
    /// # Arguments
    /// * `dag_repository` - Repository for DAG persistence
    /// * `execution_repository` - Repository for execution state
    /// 
    /// # Returns
    /// * `ServiceContainer` - New container instance
    pub fn new(
        dag_repository: Arc<dyn DagRepository>,
        execution_repository: Arc<dyn ExecutionRepository>,
    ) -> Self {
        Self {
            dag_repository,
            execution_repository,
        }
    }

    /// Creates a DAG service instance
    /// 
    /// # Returns
    /// * `DagService` - Service with injected dependencies
    pub fn dag_service(&self) -> DagService {
        DagService::new(Arc::clone(&self.dag_repository))
    }

    /// Creates an execution service instance
    /// 
    /// # Returns
    /// * `ExecutionService` - Service with injected dependencies
    pub fn execution_service(&self) -> ExecutionService {
        ExecutionService::new(
            Arc::clone(&self.dag_repository),
            Arc::clone(&self.execution_repository),
        )
    }

    /// Gets a reference to the DAG repository
    /// 
    /// # Returns
    /// * `Arc<dyn DagRepository>` - Shared reference to the repository
    pub fn dag_repository(&self) -> Arc<dyn DagRepository> {
        Arc::clone(&self.dag_repository)
    }

    /// Gets a reference to the execution repository
    /// 
    /// # Returns
    /// * `Arc<dyn ExecutionRepository>` - Shared reference to the repository
    pub fn execution_repository(&self) -> Arc<dyn ExecutionRepository> {
        Arc::clone(&self.execution_repository)
    }
}

/// Builder for creating a service container
/// 
/// Provides a fluent API for configuring and building a container
/// with all required dependencies.
pub struct ServiceContainerBuilder {
    dag_repository: Option<Arc<dyn DagRepository>>,
    execution_repository: Option<Arc<dyn ExecutionRepository>>,
}

impl ServiceContainerBuilder {
    /// Creates a new builder
    /// 
    /// # Returns
    /// * `ServiceContainerBuilder` - New builder instance
    pub fn new() -> Self {
        Self {
            dag_repository: None,
            execution_repository: None,
        }
    }

    /// Sets the DAG repository
    /// 
    /// # Arguments
    /// * `repository` - The DAG repository to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    pub fn with_dag_repository(mut self, repository: Arc<dyn DagRepository>) -> Self {
        self.dag_repository = Some(repository);
        self
    }

    /// Sets the execution repository
    /// 
    /// # Arguments
    /// * `repository` - The execution repository to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    pub fn with_execution_repository(mut self, repository: Arc<dyn ExecutionRepository>) -> Self {
        self.execution_repository = Some(repository);
        self
    }

    /// Builds the service container
    /// 
    /// # Returns
    /// * `Result<ServiceContainer, String>` - Container or error if dependencies missing
    pub fn build(self) -> Result<ServiceContainer, String> {
        let dag_repository = self.dag_repository
            .ok_or_else(|| "DAG repository not configured".to_string())?;
        
        let execution_repository = self.execution_repository
            .ok_or_else(|| "Execution repository not configured".to_string())?;

        Ok(ServiceContainer::new(dag_repository, execution_repository))
    }
}

impl Default for ServiceContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::*;
    use async_trait::async_trait;
    use std::collections::HashMap;

    /// Mock DAG repository for testing
    struct MockDagRepository;

    #[async_trait]
    impl DagRepository for MockDagRepository {
        async fn save(&self, _dag: &Dag) -> DomainResult<()> {
            Ok(())
        }

        async fn find_by_id(&self, _id: &DagId) -> DomainResult<Option<Dag>> {
            Ok(None)
        }

        async fn find_all(&self) -> DomainResult<Vec<Dag>> {
            Ok(vec![])
        }

        async fn delete(&self, _id: &DagId) -> DomainResult<bool> {
            Ok(false)
        }

        async fn exists(&self, _id: &DagId) -> DomainResult<bool> {
            Ok(false)
        }
    }

    /// Mock execution repository for testing
    struct MockExecutionRepository;

    #[async_trait]
    impl ExecutionRepository for MockExecutionRepository {
        async fn create_execution(
            &self,
            _dag_id: &DagId,
            _execution_id: &ExecutionId,
        ) -> DomainResult<()> {
            Ok(())
        }

        async fn update_task_status(
            &self,
            _execution_id: &ExecutionId,
            _task_id: &TaskId,
            _task: &Task,
        ) -> DomainResult<()> {
            Ok(())
        }

        async fn get_execution_state(
            &self,
            _execution_id: &ExecutionId,
        ) -> DomainResult<Option<ExecutionState>> {
            Ok(None)
        }

        async fn list_executions(&self, _dag_id: &DagId) -> DomainResult<Vec<ExecutionId>> {
            Ok(vec![])
        }
    }

    /// Test creating a service container
    #[test]
    fn test_create_container() {
        let dag_repo = Arc::new(MockDagRepository);
        let exec_repo = Arc::new(MockExecutionRepository);

        let container = ServiceContainer::new(dag_repo, exec_repo);
        
        // Should be able to create services
        let _dag_service = container.dag_service();
        let _exec_service = container.execution_service();
    }

    /// Test builder pattern
    #[test]
    fn test_container_builder() {
        let dag_repo = Arc::new(MockDagRepository);
        let exec_repo = Arc::new(MockExecutionRepository);

        let container = ServiceContainerBuilder::new()
            .with_dag_repository(dag_repo)
            .with_execution_repository(exec_repo)
            .build()
            .unwrap();

        let _dag_service = container.dag_service();
        let _exec_service = container.execution_service();
    }

    /// Test builder fails without required dependencies
    #[test]
    fn test_builder_missing_dependencies() {
        let result = ServiceContainerBuilder::new().build();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("DAG repository"));
    }

    /// Test builder with only DAG repository fails
    #[test]
    fn test_builder_missing_execution_repository() {
        let dag_repo = Arc::new(MockDagRepository);

        let result = ServiceContainerBuilder::new()
            .with_dag_repository(dag_repo)
            .build();

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Execution repository"));
    }

    /// Test getting repository references from container
    #[test]
    fn test_get_repositories() {
        let dag_repo = Arc::new(MockDagRepository);
        let exec_repo = Arc::new(MockExecutionRepository);

        let container = ServiceContainer::new(dag_repo, exec_repo);

        let _dag_repo_ref = container.dag_repository();
        let _exec_repo_ref = container.execution_repository();
    }
}
