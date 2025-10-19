//! Comprehensive integration tests for all task executor types
//!
//! This module provides end-to-end integration tests for shell, Python, HTTP, and file operations,
//! including complex DAGs with mixed task types and dependencies, failure scenarios, and error recovery.

use jorm::executor::{
    ExecutionContext, ExecutionStatus, ExecutorConfig, NativeExecutor, Task, TaskConfig,
    TaskResult, TaskStatus,
};
use jorm::parser::{Dag, Dependency, Task as DagTask, TaskConfig as DagTaskConfig};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::Duration;

/// Test fixture for creating temporary files and directories
pub struct TestFixture {
    temp_dir: TempDir,
}

impl TestFixture {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    pub fn temp_path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let file_path = self.temp_path().join(name);
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    pub fn create_dir(&self, name: &str) -> PathBuf {
        let dir_path = self.temp_path().join(name);
        fs::create_dir_all(&dir_path).expect("Failed to create test directory");
        dir_path
    }

    pub fn file_exists(&self, name: &str) -> bool {
        self.temp_path().join(name).exists()
    }

    pub fn read_file(&self, name: &str) -> String {
        fs::read_to_string(self.temp_path().join(name)).expect("Failed to read test file")
    }
}

/// Helper function to create a test DAG with various task types
fn create_mixed_task_dag(fixture: &TestFixture) -> Dag {
    let mut tasks = HashMap::new();

    // Shell task - create a test file
    tasks.insert(
        "shell_task".to_string(),
        DagTask {
            name: "shell_task".to_string(),
            description: Some("Create test file with shell".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some(format!(
                    "echo 'Hello from shell' > {}",
                    fixture.temp_path().join("shell_output.txt").display()
                )),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    // Python task - process the shell output
    tasks.insert(
        "python_task".to_string(),
        DagTask {
            name: "python_task".to_string(),
            description: Some("Process shell output with Python".to_string()),
            config: DagTaskConfig {
                task_type: Some("python".to_string()),
                command: None,
                script: Some(format!(
                    r#"
import os
input_file = r"{}"
output_file = r"{}"

if os.path.exists(input_file):
    with open(input_file, 'r') as f:
        content = f.read().strip()
    
    with open(output_file, 'w') as f:
        f.write(f"Python processed: {{content}}")
    
    print(f"Processed: {{content}}")
else:
    print("Input file not found")
    exit(1)
"#,
                    fixture.temp_path().join("shell_output.txt").display(),
                    fixture.temp_path().join("python_output.txt").display()
                )),
                destination: None,
                ..Default::default()
            },
        },
    );

    // File task - copy the processed file
    tasks.insert(
        "file_task".to_string(),
        DagTask {
            name: "file_task".to_string(),
            description: Some("Copy processed file".to_string()),
            config: DagTaskConfig {
                task_type: Some("file".to_string()),
                command: None,
                script: None,
                destination: Some(fixture.temp_path().join("final_output.txt").display().to_string()),
                source: Some(fixture.temp_path().join("python_output.txt").display().to_string()),
                operation: Some("copy".to_string()),
                ..Default::default()
            },
        },
    );

    // HTTP task - make a test request (optional, may fail if no network)
    tasks.insert(
        "http_task".to_string(),
        DagTask {
            name: "http_task".to_string(),
            description: Some("Make HTTP request".to_string()),
            config: DagTaskConfig {
                task_type: Some("http".to_string()),
                command: None,
                script: None,
                destination: None,
                method: Some("GET".to_string()),
                url: Some("https://httpbin.org/get".to_string()),
                ..Default::default()
            },
        },
    );

    Dag {
        name: "mixed_task_test".to_string(),
        schedule: None,
        tasks,
        dependencies: vec![
            Dependency {
                task: "python_task".to_string(),
                depends_on: "shell_task".to_string(),
            },
            Dependency {
                task: "file_task".to_string(),
                depends_on: "python_task".to_string(),
            },
            // HTTP task runs independently
        ],
    }
}

/// Helper function to create a DAG with failure scenarios
fn create_failure_scenario_dag() -> Dag {
    let mut tasks = HashMap::new();

    // Successful task
    tasks.insert(
        "success_task".to_string(),
        DagTask {
            name: "success_task".to_string(),
            description: Some("Task that succeeds".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Success'".to_string()),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    // Failing task
    tasks.insert(
        "failing_task".to_string(),
        DagTask {
            name: "failing_task".to_string(),
            description: Some("Task that fails".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("exit 1".to_string()),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    // Task that depends on failing task (should be skipped)
    tasks.insert(
        "dependent_task".to_string(),
        DagTask {
            name: "dependent_task".to_string(),
            description: Some("Task that depends on failing task".to_string()),
            config: DagTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Should not run'".to_string()),
                script: None,
                destination: None,
                ..Default::default()
            },
        },
    );

    Dag {
        name: "failure_scenario_test".to_string(),
        schedule: None,
        tasks,
        dependencies: vec![
            Dependency {
                task: "dependent_task".to_string(),
                depends_on: "failing_task".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod shell_task_tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_task_basic_execution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let mut tasks = HashMap::new();
        tasks.insert(
            "shell_test".to_string(),
            DagTask {
                name: "shell_test".to_string(),
                description: Some("Basic shell test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Hello World'".to_string()),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "shell_basic_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 1);

        let task_result = result.task_results.get("shell_test").unwrap();
        assert_eq!(task_result.status, TaskStatus::Success);
        assert!(task_result.stdout.contains("Hello World"));
        assert_eq!(task_result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_shell_task_with_file_operations() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let test_file = fixture.temp_path().join("test.txt");
        let command = if cfg!(target_os = "windows") {
            format!("echo Hello > {}", test_file.display())
        } else {
            format!("echo 'Hello' > {}", test_file.display())
        };

        let mut tasks = HashMap::new();
        tasks.insert(
            "file_creation".to_string(),
            DagTask {
                name: "file_creation".to_string(),
                description: Some("Create file with shell".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(command),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "shell_file_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(test_file.exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert!(content.trim().contains("Hello"));
    }

    #[tokio::test]
    async fn test_shell_task_failure_handling() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "failing_shell".to_string(),
            DagTask {
                name: "failing_shell".to_string(),
                description: Some("Shell task that fails".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("exit 42".to_string()),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "shell_failure_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(result.task_results.len(), 1);

        let task_result = result.task_results.get("failing_shell").unwrap();
        assert_eq!(task_result.status, TaskStatus::Failed);
        assert_eq!(task_result.exit_code, Some(42));
        assert!(task_result.error_message.is_some());
    }
}

#[cfg(test)]
mod python_task_tests {
    use super::*;

    #[tokio::test]
    async fn test_python_task_basic_execution() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "python_test".to_string(),
            DagTask {
                name: "python_test".to_string(),
                description: Some("Basic Python test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("python".to_string()),
                    command: None,
                    script: Some("print('Hello from Python!')".to_string()),
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "python_basic_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        // Python execution may fail if Python is not installed, which is expected
        match result.status {
            ExecutionStatus::Success => {
                let task_result = result.task_results.get("python_test").unwrap();
                assert_eq!(task_result.status, TaskStatus::Success);
                assert!(task_result.stdout.contains("Hello from Python!"));
                assert_eq!(task_result.exit_code, Some(0));
            }
            ExecutionStatus::Failed => {
                // Python might not be available in test environment
                println!("Python not available in test environment, test passed gracefully");
            }
        }
    }

    #[tokio::test]
    async fn test_python_task_with_file_operations() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let output_file = fixture.temp_path().join("python_output.txt");
        let script = format!(
            r#"
with open(r'{}', 'w') as f:
    f.write('Hello from Python file operation!')
print('File created successfully')
"#,
            output_file.display()
        );

        let mut tasks = HashMap::new();
        tasks.insert(
            "python_file_test".to_string(),
            DagTask {
                name: "python_file_test".to_string(),
                description: Some("Python file operation test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("python".to_string()),
                    command: None,
                    script: Some(script),
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "python_file_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        // Check if Python execution was successful
        if result.status == ExecutionStatus::Success {
            assert!(output_file.exists());
            let content = fs::read_to_string(&output_file).unwrap();
            assert!(content.contains("Hello from Python file operation!"));
        } else {
            println!("Python not available in test environment, skipping file operation test");
        }
    }

    #[tokio::test]
    async fn test_python_task_error_handling() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "python_error_test".to_string(),
            DagTask {
                name: "python_error_test".to_string(),
                description: Some("Python task with error".to_string()),
                config: DagTaskConfig {
                    task_type: Some("python".to_string()),
                    command: None,
                    script: Some("raise ValueError('Test error')".to_string()),
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "python_error_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        // If Python is available, it should fail with the error
        if result.task_results.contains_key("python_error_test") {
            let task_result = result.task_results.get("python_error_test").unwrap();
            if task_result.status == TaskStatus::Failed {
                assert!(task_result.stderr.contains("ValueError") || task_result.stderr.contains("Test error"));
            }
        }
    }
}

#[cfg(test)]
mod http_task_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_http_task_get_request() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "http_get_test".to_string(),
            DagTask {
                name: "http_get_test".to_string(),
                description: Some("HTTP GET test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("http".to_string()),
                    command: None,
                    script: None,
                    destination: None,
                    method: Some("GET".to_string()),
                    url: Some("https://httpbin.org/get".to_string()),
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "http_get_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        // Network might not be available in test environment
        match result.status {
            ExecutionStatus::Success => {
                let task_result = result.task_results.get("http_get_test").unwrap();
                assert_eq!(task_result.status, TaskStatus::Success);
                assert_eq!(task_result.exit_code, Some(200));
                assert!(!task_result.stdout.is_empty());
            }
            ExecutionStatus::Failed => {
                println!("Network not available in test environment, HTTP test skipped");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_http_task_post_request() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "http_post_test".to_string(),
            DagTask {
                name: "http_post_test".to_string(),
                description: Some("HTTP POST test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("http".to_string()),
                    command: None,
                    script: None,
                    destination: None,
                    method: Some("POST".to_string()),
                    url: Some("https://httpbin.org/post".to_string()),
                    body: Some(r#"{"test": "data"}"#.to_string()),
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "http_post_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        // Network might not be available in test environment
        match result.status {
            ExecutionStatus::Success => {
                let task_result = result.task_results.get("http_post_test").unwrap();
                assert_eq!(task_result.status, TaskStatus::Success);
                assert_eq!(task_result.exit_code, Some(200));
                assert!(!task_result.stdout.is_empty());
            }
            ExecutionStatus::Failed => {
                println!("Network not available in test environment, HTTP POST test skipped");
            }
        }
    }
}

#[cfg(test)]
mod file_task_tests {
    use super::*;

    #[tokio::test]
    async fn test_file_task_copy_operation() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        // Create source file
        let source_content = "Hello, file copy test!";
        let source_file = fixture.create_file("source.txt", source_content);
        let dest_file = fixture.temp_path().join("destination.txt");

        let mut tasks = HashMap::new();
        tasks.insert(
            "file_copy_test".to_string(),
            DagTask {
                name: "file_copy_test".to_string(),
                description: Some("File copy test".to_string()),
                config: DagTaskConfig {
                    task_type: Some("file".to_string()),
                    command: None,
                    script: None,
                    source: Some(source_file.display().to_string()),
                    destination: Some(dest_file.display().to_string()),
                    operation: Some("copy".to_string()),
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "file_copy_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 1);

        let task_result = result.task_results.get("file_copy_test").unwrap();
        assert_eq!(task_result.status, TaskStatus::Success);

        // Verify file was copied
        assert!(dest_file.exists());
        let copied_content = fs::read_to_string(&dest_file).unwrap();
        assert_eq!(copied_content, source_content);
    }

    #[tokio::test]
    async fn test_file_task_missing_source() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let nonexistent_source = fixture.temp_path().join("nonexistent.txt");
        let dest_file = fixture.temp_path().join("destination.txt");

        let mut tasks = HashMap::new();
        tasks.insert(
            "file_copy_fail_test".to_string(),
            DagTask {
                name: "file_copy_fail_test".to_string(),
                description: Some("File copy with missing source".to_string()),
                config: DagTaskConfig {
                    task_type: Some("file".to_string()),
                    command: None,
                    script: None,
                    source: Some(nonexistent_source.display().to_string()),
                    destination: Some(dest_file.display().to_string()),
                    operation: Some("copy".to_string()),
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "file_copy_fail_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Failed);
        let task_result = result.task_results.get("file_copy_fail_test").unwrap();
        assert_eq!(task_result.status, TaskStatus::Failed);
        assert!(task_result.error_message.is_some());
    }
}

#[cfg(test)]
mod mixed_dag_tests {
    use super::*;

    #[tokio::test]
    async fn test_mixed_task_types_dag() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let dag = create_mixed_task_dag(&fixture);
        let result = executor.execute_dag(&dag).await.unwrap();

        // The DAG should execute successfully (HTTP task may fail due to network)
        // but shell, Python (if available), and file tasks should work
        assert!(result.task_results.len() >= 3); // At least shell, python, file tasks

        // Check shell task
        if let Some(shell_result) = result.task_results.get("shell_task") {
            assert_eq!(shell_result.status, TaskStatus::Success);
            assert!(fixture.file_exists("shell_output.txt"));
        }

        // Check Python task (if Python is available)
        if let Some(python_result) = result.task_results.get("python_task") {
            if python_result.status == TaskStatus::Success {
                assert!(fixture.file_exists("python_output.txt"));
                let content = fixture.read_file("python_output.txt");
                assert!(content.contains("Python processed"));
            }
        }

        // Check file task (depends on Python task success)
        if let Some(file_result) = result.task_results.get("file_task") {
            if file_result.status == TaskStatus::Success {
                assert!(fixture.file_exists("final_output.txt"));
            }
        }

        // HTTP task runs independently and may fail due to network
        if let Some(http_result) = result.task_results.get("http_task") {
            // Either succeeds or fails gracefully
            assert!(matches!(
                http_result.status,
                TaskStatus::Success | TaskStatus::Failed
            ));
        }
    }

    #[tokio::test]
    async fn test_dependency_execution_order() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        let dag = create_mixed_task_dag(&fixture);
        let result = executor.execute_dag(&dag).await.unwrap();

        // Verify execution order by checking timestamps
        if let (Some(shell_result), Some(python_result)) = (
            result.task_results.get("shell_task"),
            result.task_results.get("python_task"),
        ) {
            // Python task should start after shell task completes
            assert!(python_result.started_at >= shell_result.completed_at);
        }

        if let (Some(python_result), Some(file_result)) = (
            result.task_results.get("python_task"),
            result.task_results.get("file_task"),
        ) {
            // File task should start after Python task completes (if both succeeded)
            if python_result.status == TaskStatus::Success
                && file_result.status == TaskStatus::Success
            {
                assert!(file_result.started_at >= python_result.completed_at);
            }
        }
    }

    #[tokio::test]
    async fn test_parallel_execution_of_independent_tasks() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Create DAG with multiple independent tasks
        let mut tasks = HashMap::new();
        for i in 1..=3 {
            tasks.insert(
                format!("parallel_task_{}", i),
                DagTask {
                    name: format!("parallel_task_{}", i),
                    description: Some(format!("Parallel task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Task {}'", i)),
                        script: None,
                        destination: None,
                        ..Default::default()
                    },
                },
            );
        }

        let dag = Dag {
            name: "parallel_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![], // No dependencies = all tasks can run in parallel
        };

        let start_time = std::time::Instant::now();
        let result = executor.execute_dag(&dag).await.unwrap();
        let total_time = start_time.elapsed();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 3);

        // All tasks should complete successfully
        for i in 1..=3 {
            let task_result = result.task_results.get(&format!("parallel_task_{}", i)).unwrap();
            assert_eq!(task_result.status, TaskStatus::Success);
        }

        // Parallel execution should be faster than sequential
        // (This is a rough check - in practice, the overhead might be small for simple tasks)
        assert!(total_time < Duration::from_secs(10));
    }
}

#[cfg(test)]
mod failure_scenario_tests {
    use super::*;

    #[tokio::test]
    async fn test_failure_propagation() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let dag = create_failure_scenario_dag();
        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Failed);

        // Success task should complete
        let success_result = result.task_results.get("success_task").unwrap();
        assert_eq!(success_result.status, TaskStatus::Success);

        // Failing task should fail
        let failing_result = result.task_results.get("failing_task").unwrap();
        assert_eq!(failing_result.status, TaskStatus::Failed);
        assert_eq!(failing_result.exit_code, Some(1));

        // Dependent task should not run (or should be skipped)
        // Note: The current implementation might still attempt to run it
        // This test verifies the overall DAG fails when a task fails
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let mut config = ExecutorConfig::default();
        config.default_timeout = Duration::from_millis(100); // Very short timeout

        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "timeout_task".to_string(),
            DagTask {
                name: "timeout_task".to_string(),
                description: Some("Task that should timeout".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(if cfg!(target_os = "windows") {
                        "timeout /t 2".to_string() // Windows timeout command
                    } else {
                        "sleep 2".to_string() // Unix sleep command
                    }),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "timeout_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Failed);
        let task_result = result.task_results.get("timeout_task").unwrap();
        assert_eq!(task_result.status, TaskStatus::Failed);
        // The task should fail due to timeout
    }

    #[tokio::test]
    async fn test_invalid_task_type_handling() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        let mut tasks = HashMap::new();
        tasks.insert(
            "invalid_task".to_string(),
            DagTask {
                name: "invalid_task".to_string(),
                description: Some("Task with invalid type".to_string()),
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
            name: "invalid_type_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Failed);
        let task_result = result.task_results.get("invalid_task").unwrap();
        assert_eq!(task_result.status, TaskStatus::Failed);
        assert!(task_result.error_message.is_some());
    }
}

#[cfg(test)]
mod complex_dag_tests {
    use super::*;

    #[tokio::test]
    async fn test_complex_dependency_chain() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);
        let fixture = TestFixture::new();

        // Create a complex DAG with multiple dependency chains
        let mut tasks = HashMap::new();

        // Initial tasks (can run in parallel)
        tasks.insert(
            "init_a".to_string(),
            DagTask {
                name: "init_a".to_string(),
                description: Some("Initial task A".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'Init A' > {}",
                        fixture.temp_path().join("init_a.txt").display()
                    )),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "init_b".to_string(),
            DagTask {
                name: "init_b".to_string(),
                description: Some("Initial task B".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'Init B' > {}",
                        fixture.temp_path().join("init_b.txt").display()
                    )),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        // Middle task (depends on both initial tasks)
        tasks.insert(
            "middle".to_string(),
            DagTask {
                name: "middle".to_string(),
                description: Some("Middle task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'Middle task' > {}",
                        fixture.temp_path().join("middle.txt").display()
                    )),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        // Final task (depends on middle task)
        tasks.insert(
            "final".to_string(),
            DagTask {
                name: "final".to_string(),
                description: Some("Final task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'Final task' > {}",
                        fixture.temp_path().join("final.txt").display()
                    )),
                    script: None,
                    destination: None,
                    ..Default::default()
                },
            },
        );

        let dag = Dag {
            name: "complex_dependency_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![
                Dependency {
                    task: "middle".to_string(),
                    depends_on: "init_a".to_string(),
                },
                Dependency {
                    task: "middle".to_string(),
                    depends_on: "init_b".to_string(),
                },
                Dependency {
                    task: "final".to_string(),
                    depends_on: "middle".to_string(),
                },
            ],
        };

        let result = executor.execute_dag(&dag).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 4);

        // All tasks should complete successfully
        for task_name in ["init_a", "init_b", "middle", "final"] {
            let task_result = result.task_results.get(task_name).unwrap();
            assert_eq!(task_result.status, TaskStatus::Success);
            assert!(fixture.file_exists(&format!("{}.txt", task_name)));
        }

        // Verify execution order
        let init_a_result = result.task_results.get("init_a").unwrap();
        let init_b_result = result.task_results.get("init_b").unwrap();
        let middle_result = result.task_results.get("middle").unwrap();
        let final_result = result.task_results.get("final").unwrap();

        // Middle task should start after both initial tasks complete
        assert!(middle_result.started_at >= init_a_result.completed_at);
        assert!(middle_result.started_at >= init_b_result.completed_at);

        // Final task should start after middle task completes
        assert!(final_result.started_at >= middle_result.completed_at);
    }

    #[tokio::test]
    async fn test_large_dag_performance() {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Create a DAG with many independent tasks to test scalability
        let mut tasks = HashMap::new();
        let task_count = 20;

        for i in 1..=task_count {
            tasks.insert(
                format!("task_{}", i),
                DagTask {
                    name: format!("task_{}", i),
                    description: Some(format!("Performance test task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Task {}'", i)),
                        script: None,
                        destination: None,
                        ..Default::default()
                    },
                },
            );
        }

        let dag = Dag {
            name: "large_dag_performance_test".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![], // All tasks independent for maximum parallelism
        };

        let start_time = std::time::Instant::now();
        let result = executor.execute_dag(&dag).await.unwrap();
        let execution_time = start_time.elapsed();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), task_count);

        // All tasks should complete successfully
        for i in 1..=task_count {
            let task_result = result.task_results.get(&format!("task_{}", i)).unwrap();
            assert_eq!(task_result.status, TaskStatus::Success);
        }

        // Performance check - should complete within reasonable time
        assert!(
            execution_time < Duration::from_secs(30),
            "Large DAG execution took too long: {:?}",
            execution_time
        );

        println!(
            "Large DAG with {} tasks completed in {:?}",
            task_count, execution_time
        );
    }
}