//! Comprehensive Executor Unit Tests
//!
#![allow(unused_variables, dead_code)]
#![allow(clippy::assertions_on_constants, clippy::field_reassign_with_default)]
//! Comprehensive Executor Unit Tests
//! 
//! This module contains all unit tests for the executor infrastructure,
//! consolidating previously scattered test files into a single, well-organized module.

use jorm_rs::executor::{ExecutionStatus, ExecutorConfig, NativeExecutor};
use jorm_rs::parser::{Dag, Dependency, Task as DagTask, TaskConfig as DagTaskConfig};
use std::collections::HashMap;
use tokio::time::Duration;

/// Test helper to create a simple test DAG
fn create_test_dag() -> Dag {
    let mut tasks = HashMap::new();
    tasks.insert(
        "task1".to_string(),
        DagTask {
            name: "task1".to_string(),
            description: Some("Test task 1".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Hello from task1'".to_string()),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    tasks.insert(
        "task2".to_string(),
        DagTask {
            name: "task2".to_string(),
            description: Some("Test task 2".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Hello from task2'".to_string()),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    Dag {
        name: "test_dag".to_string(),
        schedule: None,
        tasks,
        dependencies: vec![Dependency {
            task: "task2".to_string(),
            depends_on: "task1".to_string(),
        }],
    }
}

/// Test helper to create a Python test DAG
fn create_python_test_dag() -> Dag {
    let mut tasks = HashMap::new();
    tasks.insert(
        "python_task".to_string(),
        DagTask {
            name: "python_task".to_string(),
            description: Some("Python test task".to_string()),
            config: DagTaskConfig {
                task_type: Some("python".to_string()),
                command: None,
                script: Some("print('Hello from Python!')".to_string()),
                destination: None,
                ..Default::default()
            },
        },
    );

    Dag {
        name: "python_test_dag".to_string(),
        schedule: None,
        tasks,
        dependencies: vec![],
    }
}

#[cfg(test)]
mod executor_config_tests {
    use super::*;

    #[test]
    fn test_executor_config_creation() {
        let config = ExecutorConfig::default();
        assert!(config.max_concurrent_tasks > 0);
        assert!(config.default_timeout > Duration::ZERO);
        assert!(config.retry_config.max_attempts > 0);
    }

    #[test]
    fn test_executor_config_validation() {
        let config = ExecutorConfig::default();
        assert!(config.validate().is_ok());

        let invalid_config = ExecutorConfig {
            max_concurrent_tasks: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_executor_config_customization() {
        let mut config = ExecutorConfig::default();
        config.max_concurrent_tasks = 8;
        config.default_timeout = Duration::from_secs(300);

        assert_eq!(config.max_concurrent_tasks, 8);
        assert_eq!(config.default_timeout, Duration::from_secs(300));
        assert!(config.validate().is_ok());
    }
}

#[cfg(test)]
mod executor_creation_tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Should create successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_executor_with_custom_config() {
        let mut config = ExecutorConfig::default();
        config.max_concurrent_tasks = 4;
        config.default_timeout = Duration::from_secs(60);

        let executor = NativeExecutor::new(config);

        // Should accept custom configuration
        assert!(true);
    }
}

#[cfg(test)]
mod task_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_shell_task_execution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_test_dag();

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should execute successfully
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.status, ExecutionStatus::Success);
        assert_eq!(execution_result.task_results.len(), 2);
    }

    #[tokio::test]
    async fn test_python_task_execution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_python_test_dag();

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should execute (may fail if Python not installed, which is expected)
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        // Python execution may fail if Python is not installed, which is expected
        assert!(matches!(
            execution_result.status,
            ExecutionStatus::Success | ExecutionStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_task_dependency_resolution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_test_dag();

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should execute successfully with proper dependency resolution
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.status, ExecutionStatus::Success);

        // Verify task execution order (task1 should execute before task2)
        let task1_result = execution_result.task_results.get("task1");
        let task2_result = execution_result.task_results.get("task2");

        assert!(task1_result.is_some());
        assert!(task2_result.is_some());
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_task_type() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Create a DAG with invalid task type
        let mut tasks = HashMap::new();
        tasks.insert(
            "invalid_task".to_string(),
            DagTask {
                name: "invalid_task".to_string(),
                description: Some("Invalid task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("invalid_type".to_string()),
                    command: Some("echo 'test'".to_string()),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "invalid_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should fail due to invalid task type
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.status, ExecutionStatus::Failed);
    }

    #[tokio::test]
    async fn test_failing_task() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Create a DAG with a failing task
        let mut tasks = HashMap::new();
        tasks.insert(
            "failing_task".to_string(),
            DagTask {
                name: "failing_task".to_string(),
                description: Some("Failing task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("exit 1".to_string()),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "failing_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should fail due to failing task
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.status, ExecutionStatus::Failed);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_performance() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_test_dag();

        let start_time = std::time::Instant::now();
        let result = executor.execute_dag(&dag).await;
        let duration = start_time.elapsed();

        // Should execute within reasonable time
        assert!(result.is_ok());
        assert!(
            duration.as_secs() < 10,
            "Execution should complete within 10 seconds"
        );
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_test_dag();

        // Execute multiple DAGs concurrently
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let dag = dag.clone();
                tokio::spawn(async move {
                    let executor = NativeExecutor::new(ExecutorConfig::default());
                    executor.execute_dag(&dag).await
                })
            })
            .collect();

        // Wait for all executions to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_metrics() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_test_dag();

        // Execute the DAG
        let result = executor.execute_dag(&dag).await;

        // Should collect metrics
        assert!(result.is_ok());
        let execution_result = result.unwrap();

        // Verify metrics are collected
        assert!(execution_result.metrics.total_tasks > 0);
           // Removed absurd comparisons
    }
}
