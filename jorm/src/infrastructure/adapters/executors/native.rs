/// Native task executor adapter
/// 
/// Wraps the existing NativeExecutor to implement the TaskExecutor port.

use crate::executor::{ExecutionResult, ExecutorConfig, NativeExecutor, TaskResult};
use crate::infrastructure::ports::TaskExecutor;
use crate::parser::Dag;
use async_trait::async_trait;
use tracing::{info, instrument};

/// Native implementation of TaskExecutor
/// 
/// Wraps the existing NativeExecutor to provide the TaskExecutor interface.
pub struct NativeTaskExecutorAdapter {
    /// The underlying native executor
    executor: NativeExecutor,
}

impl NativeTaskExecutorAdapter {
    /// Creates a new native task executor adapter
    /// 
    /// # Returns
    /// * `NativeTaskExecutorAdapter` - New executor instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::NativeTaskExecutorAdapter;
    /// 
    /// let executor = NativeTaskExecutorAdapter::new();
    /// ```
    pub fn new() -> Self {
        let config = ExecutorConfig::default();
        Self {
            executor: NativeExecutor::new(config),
        }
    }

    /// Creates a new native task executor adapter with custom configuration
    /// 
    /// # Arguments
    /// * `config` - Executor configuration
    /// 
    /// # Returns
    /// * `NativeTaskExecutorAdapter` - New executor instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::NativeTaskExecutorAdapter;
    /// use jorm::executor::ExecutorConfig;
    /// 
    /// let config = ExecutorConfig::default();
    /// let executor = NativeTaskExecutorAdapter::with_config(config);
    /// ```
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            executor: NativeExecutor::new(config),
        }
    }
}

impl Default for NativeTaskExecutorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskExecutor for NativeTaskExecutorAdapter {
    async fn execute_task(
        &self,
        task: &crate::executor::traits::Task,
        context: &crate::executor::ExecutionContext,
    ) -> anyhow::Result<TaskResult> {
        self.executor.execute_task(task, context).await
    }

    #[instrument(skip(self, dag), fields(dag_name = %dag.name, task_count = dag.tasks.len()))]
    async fn execute_dag(&self, dag: &Dag) -> anyhow::Result<ExecutionResult> {
        info!("Executing DAG with native executor");
        self.executor.execute_dag(dag).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test creating a new executor
    #[test]
    fn test_new() {
        let _executor = NativeTaskExecutorAdapter::new();
    }

    /// Test creating with custom config
    #[test]
    fn test_with_config() {
        let config = ExecutorConfig::default();
        let _executor = NativeTaskExecutorAdapter::with_config(config);
    }

    /// Test default implementation
    #[test]
    fn test_default() {
        let _executor = NativeTaskExecutorAdapter::default();
    }
}
