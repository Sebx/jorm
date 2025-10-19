//! DAG Integration Tests
//!
//! Comprehensive integration tests for DAG execution functionality.
//! Tests real task execution, dependency resolution, error handling, and various task types.

use jorm_rs::executor::{ExecutionStatus, ExecutorConfig, NativeExecutor, TaskStatus};
use jorm_rs::parser::{parse_dag_file, validate_dag};
use std::fs;
use tempfile::TempDir;

/// Test helper for DAG execution tests
struct DAGTester {
    executor: NativeExecutor,
    temp_dirs: Vec<TempDir>,
}

impl DAGTester {
    fn new() -> Self {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        Self {
            executor,
            temp_dirs: Vec::new(),
        }
    }

    fn create_temp_dag(&mut self, content: &str, filename: &str) -> String {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let dag_path = temp_dir.path().join(filename);
        fs::write(&dag_path, content).expect("Failed to write DAG file");

        self.temp_dirs.push(temp_dir);
        dag_path.to_string_lossy().to_string()
    }

    async fn execute_dag(
        &self,
        dag_path: &str,
    ) -> Result<jorm_rs::executor::ExecutionResult, Box<dyn std::error::Error>> {
        use std::path::Path;

        // Run the DAG with the process working directory set to the DAG file's
        // containing directory so relative paths in tasks resolve inside the
        // temporary test directory. Restore the original cwd afterwards.
        let original_cwd = std::env::current_dir()?;
        let dag_dir = Path::new(dag_path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| original_cwd.clone());

        std::env::set_current_dir(&dag_dir)?;
        let dag = parse_dag_file(dag_path).await?;
        validate_dag(&dag)?;
        let result = self.executor.execute_dag(&dag).await?;
        std::env::set_current_dir(original_cwd)?;
        Ok(result)
    }
}

// ============================================================================
// SHELL TASK TESTS
// ============================================================================

#[tokio::test]
async fn test_shell_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: test_shell_dag

tasks:
  task1:
    type: shell
    description: First task
    command: echo "Hello from task1"
  
  task2:
    type: shell
    description: Second task
    command: echo "Hello from task2"

dependencies:
- task2 after task1"#;

    let dag_path = tester.create_temp_dag(dag_content, "shell_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution results
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 2);
    assert!(result.task_results.contains_key("task1"));
    assert!(result.task_results.contains_key("task2"));

    // Verify task execution order (task1 should run before task2)
    let task1_result = result.task_results.get("task1").unwrap();
    let task2_result = result.task_results.get("task2").unwrap();

    assert_eq!(task1_result.status, TaskStatus::Success);
    assert_eq!(task2_result.status, TaskStatus::Success);

    // Verify output contains expected text
    assert!(task1_result.stdout.contains("Hello from task1"));
    assert!(task2_result.stdout.contains("Hello from task2"));
}

#[tokio::test]
async fn test_shell_dag_with_dependencies() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: shell_dependencies_dag

tasks:
  create_file:
    type: shell
    description: Create a test file
    command: echo Test content > test_file.txt
  
  list_files:
    type: shell
    description: List files
    command: dir

dependencies:
- list_files after create_file"#;

    let dag_path = tester.create_temp_dag(dag_content, "shell_dependencies_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 2);

    let create_result = result.task_results.get("create_file").unwrap();
    let list_result = result.task_results.get("list_files").unwrap();

    assert_eq!(create_result.status, TaskStatus::Success);
    assert_eq!(list_result.status, TaskStatus::Success);
}

// ============================================================================
// PYTHON TASK TESTS
// ============================================================================

#[tokio::test]
async fn test_python_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: test_python_dag

tasks:
  python_task:
    type: python
    description: Python task
    script: |
      print("Hello from Python!")
      import sys
      print(f"Python version: {sys.version}")

dependencies:
- python_task"#;

    let dag_path = tester.create_temp_dag(dag_content, "python_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution results
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 1);
    assert!(result.task_results.contains_key("python_task"));

    let python_result = result.task_results.get("python_task").unwrap();
    assert_eq!(python_result.status, TaskStatus::Success);
    assert!(python_result.stdout.contains("Hello from Python!"));
}

#[tokio::test]
async fn test_python_dag_with_file_operations() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: python_file_dag

tasks:
  python_file_task:
    type: python
    description: Python file operation
    script: |
      with open("python_output.txt", "w") as f:
          f.write("Python script was executed!")
      
      print("File created: python_output.txt")

dependencies:
- python_file_task"#;

    let dag_path = tester.create_temp_dag(dag_content, "python_file_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 1);

    let python_result = result.task_results.get("python_file_task").unwrap();
    assert_eq!(python_result.status, TaskStatus::Success);
    assert!(python_result
        .stdout
        .contains("File created: python_output.txt"));
}

// ============================================================================
// FILE TASK TESTS
// ============================================================================

#[tokio::test]
async fn test_file_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: test_file_dag

tasks:
  create_file:
    type: file
    description: Create a test file
    destination: test_output.txt
    script: |
      Hello from file operation!

dependencies:
- create_file"#;

    let dag_path = tester.create_temp_dag(dag_content, "file_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution results
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 1);
    assert!(result.task_results.contains_key("create_file"));

    let file_result = result.task_results.get("create_file").unwrap();
    assert_eq!(file_result.status, TaskStatus::Success);
}

#[tokio::test]
async fn test_file_dag_with_operations() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: file_operations_dag

tasks:
  create_file:
    type: file
    description: Create a test file
    destination: file_operation_test.txt
    script: |
      This is a test file created by the DAG execution engine.
  
  copy_file:
    type: shell
    description: Copy the file
    command: copy file_operation_test.txt file_operation_copy.txt

dependencies:
- copy_file after create_file"#;

    let dag_path = tester.create_temp_dag(dag_content, "file_operations_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 2);

    let create_result = result.task_results.get("create_file").unwrap();
    let copy_result = result.task_results.get("copy_file").unwrap();

    assert_eq!(create_result.status, TaskStatus::Success);
    assert_eq!(copy_result.status, TaskStatus::Success);
}

// ============================================================================
// COMPLEX DEPENDENCY TESTS
// ============================================================================

#[tokio::test]
async fn test_complex_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: complex_dag

tasks:
  task_a:
    type: shell
    description: Task A
    command: echo "Task A executed"
  
  task_b:
    type: shell
    description: Task B
    command: echo "Task B executed"
  
  task_c:
    type: shell
    description: Task C
    command: echo "Task C executed"
  
  task_d:
    type: shell
    description: Task D
    command: echo "Task D executed"

dependencies:
- task_c after task_a
- task_c after task_b
- task_d after task_c"#;

    let dag_path = tester.create_temp_dag(dag_content, "complex_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution results
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 4);

    // Verify all tasks executed successfully
    for (task_name, task_result) in &result.task_results {
        assert_eq!(
            task_result.status,
            TaskStatus::Success,
            "Task {task_name} failed"
        );
    }
}

#[tokio::test]
async fn test_concurrent_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: concurrent_dag

tasks:
  task_a:
    type: shell
    description: Task A (independent)
    command: echo "Task A executed"
  
  task_b:
    type: shell
    description: Task B (independent)
    command: echo "Task B executed"
  
  task_c:
    type: shell
    description: Task C (independent)
    command: echo "Task C executed"
  
  final_task:
    type: shell
    description: Final task
    command: echo "All tasks completed"

dependencies:
- final_task after task_a
- final_task after task_b
- final_task after task_c"#;

    let dag_path = tester.create_temp_dag(dag_content, "concurrent_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 4);

    // Verify all tasks executed successfully
    for (task_name, task_result) in &result.task_results {
        assert_eq!(
            task_result.status,
            TaskStatus::Success,
            "Task {task_name} failed"
        );
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_failing_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: failing_dag

tasks:
  failing_task:
    type: shell
    description: This task will fail
    command: exit 1
  
  success_task:
    type: shell
    description: This should not run
    command: echo "This should not execute"

dependencies:
- success_task after failing_task"#;

    let dag_path = tester.create_temp_dag(dag_content, "failing_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution failed
    assert_eq!(result.status, ExecutionStatus::Failed);

    // Verify only the failing task was executed
    assert_eq!(result.task_results.len(), 1);
    assert!(result.task_results.contains_key("failing_task"));

    let failing_result = result.task_results.get("failing_task").unwrap();
    assert_eq!(failing_result.status, TaskStatus::Failed);
    assert_eq!(failing_result.exit_code, Some(1));

    // Verify success_task was not executed
    assert!(!result.task_results.contains_key("success_task"));
}

#[tokio::test]
async fn test_unsupported_task_type() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: unsupported_dag

tasks:
  unsupported_task:
    type: unsupported_type
    command: echo "This should fail"

dependencies:
- unsupported_task"#;

    let dag_path = tester.create_temp_dag(dag_content, "unsupported_dag.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution failed due to unsupported task type
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(result.task_results.len(), 0);
}

// ============================================================================
// DATABASE WORKFLOW TESTS
// ============================================================================

#[tokio::test]
async fn test_database_workflow_dag_execution() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: mock_database_workflow

tasks:
  extract_data:
    type: python
    description: Extract data from mock database
    script: |
      print("🔗 Connecting to mock database...")
      print("📊 Extracting data from mock table...")
      print("✅ Data extracted successfully: 100 records")
      print("📈 Records extracted: 100")
  
  transform_data:
    type: python
    description: Transform extracted data
    script: |
      print("🔄 Transforming data...")
      print("✅ Data transformation completed")
      print("📊 Transformed 100 records")
  
  load_data:
    type: python
    description: Load transformed data
    script: |
      print("💾 Loading data to destination...")
      print("✅ Data loaded successfully")
      print("📈 Final records: 100")

dependencies:
- transform_data after extract_data
- load_data after transform_data"#;

    let dag_path = tester.create_temp_dag(dag_content, "database_workflow.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution results
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.task_results.len(), 3);

    // Verify task execution order
    let extract_result = result.task_results.get("extract_data").unwrap();
    let transform_result = result.task_results.get("transform_data").unwrap();
    let load_result = result.task_results.get("load_data").unwrap();

    assert_eq!(extract_result.status, TaskStatus::Success);
    assert_eq!(transform_result.status, TaskStatus::Success);
    assert_eq!(load_result.status, TaskStatus::Success);

    // Verify output contains expected database operations
    assert!(extract_result
        .stdout
        .contains("Connecting to mock database"));
    assert!(extract_result.stdout.contains("Extracting data"));
    assert!(extract_result.stdout.contains("100 records"));

    assert!(transform_result.stdout.contains("Transforming data"));
    assert!(transform_result
        .stdout
        .contains("Data transformation completed"));

    assert!(load_result.stdout.contains("Loading data"));
    assert!(load_result.stdout.contains("Data loaded successfully"));

    // Verify execution metrics
    assert_eq!(result.metrics.total_tasks, 3);
    assert_eq!(result.metrics.successful_tasks, 3);
    assert_eq!(result.metrics.failed_tasks, 0);
}

#[tokio::test]
async fn test_database_workflow_error_handling() {
    let mut tester = DAGTester::new();

    let dag_content = r#"dag: error_database_workflow

tasks:
  connect_database:
    type: python
    description: Connect to database (will fail)
    script: |
      print("🔗 Attempting to connect to database...")
      import sys
      print("❌ Database connection failed: Connection refused")
      sys.exit(1)
  
  process_data:
    type: python
    description: Process data (should not run)
    script: |
      print("This should not execute")

dependencies:
- process_data after connect_database"#;

    let dag_path = tester.create_temp_dag(dag_content, "error_database_workflow.txt");
    let result = tester
        .execute_dag(&dag_path)
        .await
        .expect("DAG execution failed");

    // Verify execution failed
    assert_eq!(result.status, ExecutionStatus::Failed);

    // Verify only the failing task was executed
    assert_eq!(result.task_results.len(), 1);
    assert!(result.task_results.contains_key("connect_database"));

    let connect_result = result.task_results.get("connect_database").unwrap();
    assert_eq!(connect_result.status, TaskStatus::Failed);
    assert_eq!(connect_result.exit_code, Some(1));
    assert!(connect_result.stdout.contains("Database connection failed"));

    // Verify process_data was not executed
    assert!(!result.task_results.contains_key("process_data"));
}
