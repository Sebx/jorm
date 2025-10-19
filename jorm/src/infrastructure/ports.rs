/// Port trait definitions - Interfaces for infrastructure
/// 
/// Ports define the contracts that adapters must implement.
/// They allow the core business logic to remain independent
/// of specific infrastructure implementations.

use crate::domain::{Dag, DagId, ExecutionId, Task, TaskId};
use crate::parser::Dag as ParserDag;
use async_trait::async_trait;
use std::path::Path;

/// Port for parsing DAGs from various sources
/// 
/// Implementations can parse from files, HTTP, databases, etc.
#[async_trait]
pub trait DagParser: Send + Sync {
    /// Parses a DAG from a file path
    /// 
    /// # Arguments
    /// * `path` - Path to the DAG file
    /// 
    /// # Returns
    /// * `anyhow::Result<ParserDag>` - Parsed DAG or error
    async fn parse_from_file(&self, path: &Path) -> anyhow::Result<ParserDag>;

    /// Parses a DAG from a string
    /// 
    /// # Arguments
    /// * `content` - DAG content as string
    /// * `format` - Format hint (yaml, json, txt, md)
    /// 
    /// # Returns
    /// * `anyhow::Result<ParserDag>` - Parsed DAG or error
    async fn parse_from_string(
        &self,
        content: &str,
        format: &str,
    ) -> anyhow::Result<ParserDag>;
}

/// Port for executing tasks
/// 
/// Implementations can execute tasks locally, remotely, in containers, etc.
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Executes a single task
    /// 
    /// # Arguments
    /// * `task` - The task to execute
    /// * `context` - Execution context
    /// 
    /// # Returns
    /// * `anyhow::Result<TaskExecutionResult>` - Execution result or error
    async fn execute_task(
        &self,
        task: &crate::executor::traits::Task,
        context: &crate::executor::ExecutionContext,
    ) -> anyhow::Result<crate::executor::TaskResult>;

    /// Executes a complete DAG
    /// 
    /// # Arguments
    /// * `dag` - The DAG to execute
    /// 
    /// # Returns
    /// * `anyhow::Result<DagExecutionResult>` - Execution result or error
    async fn execute_dag(
        &self,
        dag: &ParserDag,
    ) -> anyhow::Result<crate::executor::ExecutionResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that port traits are object-safe (can be used as trait objects)
    #[test]
    fn test_dag_parser_is_object_safe() {
        // This test verifies that DagParser can be used as a trait object
        // If this compiles, the trait is object-safe
        let _: Option<Box<dyn DagParser>> = None;
    }

    /// Test that TaskExecutor is object-safe
    #[test]
    fn test_task_executor_is_object_safe() {
        let _: Option<Box<dyn TaskExecutor>> = None;
    }
}
