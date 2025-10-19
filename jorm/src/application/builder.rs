/// Application builder - Composition root for dependency injection
/// 
/// The ApplicationBuilder provides a fluent API for constructing the
/// application with all its dependencies. This is the composition root
/// where all dependencies are wired together.

use super::services::{DagService, ExecutionService};
use crate::domain::{DagRepository, ExecutionRepository};
use crate::infrastructure::ports::{DagParser, TaskExecutor};
use std::sync::Arc;
use tracing::info;

/// Application instance with all dependencies
/// 
/// Holds references to all services and can be used to access
/// application functionality.
pub struct Application {
    /// DAG service for managing DAGs
    dag_service: DagService,
    /// Execution service for running DAGs
    execution_service: ExecutionService,
    /// DAG parser for reading DAG files
    dag_parser: Arc<dyn DagParser>,
    /// Task executor for running tasks
    task_executor: Arc<dyn TaskExecutor>,
}

impl Application {
    /// Returns the DAG service
    /// 
    /// # Returns
    /// * `&DagService` - Reference to the DAG service
    pub fn dag_service(&self) -> &DagService {
        &self.dag_service
    }

    /// Returns the execution service
    /// 
    /// # Returns
    /// * `&ExecutionService` - Reference to the execution service
    pub fn execution_service(&self) -> &ExecutionService {
        &self.execution_service
    }

    /// Returns the DAG parser
    /// 
    /// # Returns
    /// * `Arc<dyn DagParser>` - Shared reference to the DAG parser
    pub fn dag_parser(&self) -> Arc<dyn DagParser> {
        Arc::clone(&self.dag_parser)
    }

    /// Returns the task executor
    /// 
    /// # Returns
    /// * `Arc<dyn TaskExecutor>` - Shared reference to the task executor
    pub fn task_executor(&self) -> Arc<dyn TaskExecutor> {
        Arc::clone(&self.task_executor)
    }
}

/// Builder for constructing an Application instance
/// 
/// Provides a fluent API for configuring and building the application
/// with all required dependencies.
/// 
/// # Examples
/// 
/// ```no_run
/// use jorm::application::ApplicationBuilder;
/// use jorm::infrastructure::{FileSystemDagParser, InMemoryDagStorage, NativeTaskExecutorAdapter};
/// use std::sync::Arc;
/// 
/// # async fn example() -> anyhow::Result<()> {
/// let app = ApplicationBuilder::new()
///     .with_dag_parser(Arc::new(FileSystemDagParser::new()))
///     .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
///     .with_execution_storage(Arc::new(InMemoryDagStorage::new()))
///     .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct ApplicationBuilder {
    dag_parser: Option<Arc<dyn DagParser>>,
    dag_storage: Option<Arc<dyn DagRepository>>,
    execution_storage: Option<Arc<dyn ExecutionRepository>>,
    task_executor: Option<Arc<dyn TaskExecutor>>,
}

impl ApplicationBuilder {
    /// Creates a new application builder
    /// 
    /// # Returns
    /// * `ApplicationBuilder` - New builder instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::application::ApplicationBuilder;
    /// 
    /// let builder = ApplicationBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            dag_parser: None,
            dag_storage: None,
            execution_storage: None,
            task_executor: None,
        }
    }

    /// Sets the DAG parser
    /// 
    /// # Arguments
    /// * `parser` - The DAG parser to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::application::ApplicationBuilder;
    /// use jorm::infrastructure::FileSystemDagParser;
    /// use std::sync::Arc;
    /// 
    /// let builder = ApplicationBuilder::new()
    ///     .with_dag_parser(Arc::new(FileSystemDagParser::new()));
    /// ```
    pub fn with_dag_parser(mut self, parser: Arc<dyn DagParser>) -> Self {
        self.dag_parser = Some(parser);
        self
    }

    /// Sets the DAG storage
    /// 
    /// # Arguments
    /// * `storage` - The DAG storage to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    pub fn with_dag_storage(mut self, storage: Arc<dyn DagRepository>) -> Self {
        self.dag_storage = Some(storage);
        self
    }

    /// Sets the execution storage
    /// 
    /// # Arguments
    /// * `storage` - The execution storage to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    pub fn with_execution_storage(mut self, storage: Arc<dyn ExecutionRepository>) -> Self {
        self.execution_storage = Some(storage);
        self
    }

    /// Sets the task executor
    /// 
    /// # Arguments
    /// * `executor` - The task executor to use
    /// 
    /// # Returns
    /// * `Self` - Builder for method chaining
    pub fn with_task_executor(mut self, executor: Arc<dyn TaskExecutor>) -> Self {
        self.task_executor = Some(executor);
        self
    }

    /// Builds the application with all configured dependencies
    /// 
    /// # Returns
    /// * `anyhow::Result<Application>` - Configured application or error
    /// 
    /// # Errors
    /// Returns an error if any required dependency is missing
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jorm::application::ApplicationBuilder;
    /// use jorm::infrastructure::{FileSystemDagParser, InMemoryDagStorage, NativeTaskExecutorAdapter};
    /// use std::sync::Arc;
    /// 
    /// # async fn example() -> anyhow::Result<()> {
    /// let app = ApplicationBuilder::new()
    ///     .with_dag_parser(Arc::new(FileSystemDagParser::new()))
    ///     .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
    ///     .with_execution_storage(Arc::new(InMemoryDagStorage::new()))
    ///     .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> anyhow::Result<Application> {
        info!("Building application with dependencies");

        let dag_parser = self.dag_parser
            .ok_or_else(|| anyhow::anyhow!("DAG parser not configured"))?;

        let dag_storage = self.dag_storage
            .ok_or_else(|| anyhow::anyhow!("DAG storage not configured"))?;

        let execution_storage = self.execution_storage
            .ok_or_else(|| anyhow::anyhow!("Execution storage not configured"))?;

        let task_executor = self.task_executor
            .ok_or_else(|| anyhow::anyhow!("Task executor not configured"))?;

        // Create services with injected dependencies
        let dag_service = DagService::new(Arc::clone(&dag_storage));
        let execution_service = ExecutionService::new(
            Arc::clone(&dag_storage),
            Arc::clone(&execution_storage),
        );

        info!("Application built successfully");

        Ok(Application {
            dag_service,
            execution_service,
            dag_parser,
            task_executor,
        })
    }
}

impl Default for ApplicationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{
        FileSystemDagParser, InMemoryDagStorage, NativeTaskExecutorAdapter,
    };

    /// Mock execution repository for testing
    struct MockExecutionRepository;

    #[async_trait::async_trait]
    impl ExecutionRepository for MockExecutionRepository {
        async fn create_execution(
            &self,
            _dag_id: &crate::domain::DagId,
            _execution_id: &crate::domain::ExecutionId,
        ) -> crate::domain::DomainResult<()> {
            Ok(())
        }

        async fn update_task_status(
            &self,
            _execution_id: &crate::domain::ExecutionId,
            _task_id: &crate::domain::TaskId,
            _task: &crate::domain::Task,
        ) -> crate::domain::DomainResult<()> {
            Ok(())
        }

        async fn get_execution_state(
            &self,
            _execution_id: &crate::domain::ExecutionId,
        ) -> crate::domain::DomainResult<Option<crate::domain::ExecutionState>> {
            Ok(None)
        }

        async fn list_executions(
            &self,
            _dag_id: &crate::domain::DagId,
        ) -> crate::domain::DomainResult<Vec<crate::domain::ExecutionId>> {
            Ok(vec![])
        }
    }

    /// Test building application with all dependencies
    #[test]
    fn test_build_with_all_dependencies() {
        let result = ApplicationBuilder::new()
            .with_dag_parser(Arc::new(FileSystemDagParser::new()))
            .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
            .with_execution_storage(Arc::new(MockExecutionRepository))
            .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
            .build();

        assert!(result.is_ok());
    }

    /// Test building without DAG parser fails
    #[test]
    fn test_build_without_parser() {
        let result = ApplicationBuilder::new()
            .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
            .with_execution_storage(Arc::new(MockExecutionRepository))
            .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
            .build();

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("DAG parser"));
    }

    /// Test building without DAG storage fails
    #[test]
    fn test_build_without_dag_storage() {
        let result = ApplicationBuilder::new()
            .with_dag_parser(Arc::new(FileSystemDagParser::new()))
            .with_execution_storage(Arc::new(MockExecutionRepository))
            .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
            .build();

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("DAG storage"));
    }

    /// Test building without execution storage fails
    #[test]
    fn test_build_without_execution_storage() {
        let result = ApplicationBuilder::new()
            .with_dag_parser(Arc::new(FileSystemDagParser::new()))
            .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
            .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
            .build();

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Execution storage"));
    }

    /// Test building without task executor fails
    #[test]
    fn test_build_without_executor() {
        let result = ApplicationBuilder::new()
            .with_dag_parser(Arc::new(FileSystemDagParser::new()))
            .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
            .with_execution_storage(Arc::new(MockExecutionRepository))
            .build();

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Task executor"));
    }

    /// Test accessing services from application
    #[test]
    fn test_access_services() {
        let app = ApplicationBuilder::new()
            .with_dag_parser(Arc::new(FileSystemDagParser::new()))
            .with_dag_storage(Arc::new(InMemoryDagStorage::new()))
            .with_execution_storage(Arc::new(MockExecutionRepository))
            .with_task_executor(Arc::new(NativeTaskExecutorAdapter::new()))
            .build()
            .unwrap();

        let _dag_service = app.dag_service();
        let _exec_service = app.execution_service();
        let _parser = app.dag_parser();
        let _executor = app.task_executor();
    }

    /// Test default implementation
    #[test]
    fn test_default() {
        let _builder = ApplicationBuilder::default();
    }
}
