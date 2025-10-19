//! Native Executor CLI Integration Tests
//!
//! Tests for the native executor CLI flag integration and fallback behavior.
//! Covers requirements 5.1 and 5.4 from the native executor specification.

#![allow(unused_variables)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test helper for running CLI commands with native executor
struct NativeExecutorCLITester {
    jorm_path: String,
    temp_dir: TempDir,
}

impl NativeExecutorCLITester {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        Self {
            jorm_path: "target/debug/jorm.exe".to_string(),
            temp_dir,
        }
    }

    fn run_command(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(&self.jorm_path)
            .args(args)
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(1);

        (exit_code, stdout, stderr)
    }

    fn create_test_dag(&self, content: &str, filename: &str) -> String {
        let dag_path = self.temp_dir.path().join(filename);
        fs::write(&dag_path, content).expect("Failed to write DAG file");
        dag_path.to_string_lossy().to_string()
    }
}

// ============================================================================
// NATIVE EXECUTOR FLAG TESTS
// ============================================================================

#[test]
fn test_native_executor_flag_explicit() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: native_executor_test

task1: shell echo "Testing native executor"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "native_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Native executor should execute successfully");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should indicate native executor usage"
    );
    assert!(
        stdout.contains("Testing native executor"),
        "Should execute the task successfully"
    );
}

#[test]
fn test_native_executor_default_behavior() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: default_executor_test

task1: shell echo "Testing default executor"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "default_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path]);

    assert_eq!(exit_code, 0, "Default should use native executor");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should default to native executor"
    );
    assert!(
        stdout.contains("Testing default executor"),
        "Should execute the task successfully"
    );
}

#[test]
fn test_python_executor_flag() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: python_executor_test

task1: shell echo "Testing Python executor"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "python_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--python"]);

    // Python executor is not fully implemented, so this should fail gracefully
    assert!(exit_code != 0, "Python executor should fail gracefully");
    assert!(
        stdout.contains("🐍 Using Python executor") || stderr.contains("Python executor not yet implemented"),
        "Should indicate Python executor usage or failure"
    );
}

#[test]
fn test_conflicting_executor_flags() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: conflict_test

task1: shell echo "Testing conflicting flags"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "conflict_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor", "--python"]);

    // Python flag should take precedence over native-executor flag
    assert!(exit_code != 0, "Should fail with Python executor");
    assert!(
        stdout.contains("🐍 Using Python executor") || stderr.contains("Python executor not yet implemented"),
        "Python flag should take precedence"
    );
}

// ============================================================================
// FALLBACK BEHAVIOR TESTS
// ============================================================================

#[test]
fn test_native_executor_with_valid_dag() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: valid_native_test

task1: shell echo "Hello from native executor"
task2: shell echo "Second task"

task2 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "valid_native.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Valid DAG should execute successfully");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("Hello from native executor"),
        "Should execute first task"
    );
    assert!(
        stdout.contains("Second task"),
        "Should execute second task"
    );
    assert!(
        stdout.contains("✅ DAG execution completed successfully"),
        "Should complete successfully"
    );
}

#[test]
fn test_native_executor_with_shell_tasks() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: shell_tasks_test

task1: shell echo "Shell task 1"
task2: shell echo "Shell task 2"
task3: shell echo "Shell task 3"

task2 after task1
task3 after task2"#;

    let dag_path = tester.create_test_dag(dag_content, "shell_tasks.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Shell tasks should execute successfully");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("Shell task 1") && stdout.contains("Shell task 2") && stdout.contains("Shell task 3"),
        "All shell tasks should execute"
    );
}

#[test]
fn test_native_executor_with_parallel_tasks() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: parallel_test

task1: shell echo "Parallel task 1"
task2: shell echo "Parallel task 2"
task3: shell echo "Parallel task 3"
task4: shell echo "Final task"

task4 after task1, task2, task3"#;

    let dag_path = tester.create_test_dag(dag_content, "parallel_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Parallel tasks should execute successfully");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("Parallel task 1") && stdout.contains("Parallel task 2") && stdout.contains("Parallel task 3"),
        "All parallel tasks should execute"
    );
    assert!(
        stdout.contains("Final task"),
        "Final task should execute after dependencies"
    );
}

// ============================================================================
// CONFIGURATION INTEGRATION TESTS
// ============================================================================

#[test]
fn test_native_executor_with_max_concurrent() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: concurrent_test

task1: shell echo "Concurrent task 1"
task2: shell echo "Concurrent task 2"
task3: shell echo "Concurrent task 3"

task1
task2
task3"#;

    let dag_path = tester.create_test_dag(dag_content, "concurrent_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--max-concurrent", "2"
    ]);

    assert_eq!(exit_code, 0, "Should execute with concurrency limit");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("Configuration Summary"),
        "Should show configuration summary"
    );
}

#[test]
fn test_native_executor_with_timeout() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: timeout_test

task1: shell echo "Quick task"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "timeout_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--timeout", "30"
    ]);

    assert_eq!(exit_code, 0, "Should execute with timeout setting");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("Quick task"),
        "Should execute the task"
    );
}

#[test]
fn test_native_executor_with_state_management() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: state_test

task1: shell echo "State management test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "state_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--with-state"
    ]);

    assert_eq!(exit_code, 0, "Should execute with state management");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("💾 State management enabled"),
        "Should enable state management"
    );
}

// ============================================================================
// ERROR HANDLING AND VALIDATION TESTS
// ============================================================================

#[test]
fn test_native_executor_with_invalid_dag() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"Invalid DAG content
This is not a valid DAG format"#;

    let dag_path = tester.create_test_dag(dag_content, "invalid.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert!(exit_code != 0, "Invalid DAG should fail");
    // Should fail during parsing, before executor selection
}

#[test]
fn test_native_executor_with_failing_task() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: failing_task_test

task1: shell exit 1

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "failing_task.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert!(exit_code != 0, "Failing task should cause DAG failure");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    assert!(
        stdout.contains("❌ DAG execution failed") || stdout.contains("failed"),
        "Should report execution failure"
    );
}

#[test]
fn test_native_executor_with_nonexistent_file() {
    let tester = NativeExecutorCLITester::new();

    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", "nonexistent_dag.txt", "--native-executor"
    ]);

    assert!(exit_code != 0, "Nonexistent file should fail");
    assert!(
        stderr.contains("File not found") || stderr.contains("No such file"),
        "Should report file not found error"
    );
}

// ============================================================================
// COMPATIBILITY TESTS
// ============================================================================

#[test]
fn test_native_executor_txt_format() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: txt_format_test

task1: shell echo "TXT format test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "txt_format.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "TXT format should be supported");
    assert!(
        stdout.contains("TXT format test"),
        "Should execute TXT format DAG"
    );
}

#[test]
fn test_native_executor_md_format() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"# DAG: md_format_test

## Tasks
- task1: shell echo "MD format test"

## Dependencies
- task1"#;

    let dag_path = tester.create_test_dag(dag_content, "md_format.md");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "MD format should be supported");
    assert!(
        stdout.contains("MD format test"),
        "Should execute MD format DAG"
    );
}

#[test]
fn test_native_executor_yaml_format() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"dag: yaml_format_test
tasks:
  task1:
    type: shell
    command: echo "YAML format test"
dependencies:
  - task1"#;

    let dag_path = tester.create_test_dag(dag_content, "yaml_format.yaml");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "YAML format should be supported");
    assert!(
        stdout.contains("YAML format test"),
        "Should execute YAML format DAG"
    );
}

// ============================================================================
// PERFORMANCE AND METRICS TESTS
// ============================================================================

#[test]
fn test_native_executor_execution_metrics() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: metrics_test

task1: shell echo "Metrics test task 1"
task2: shell echo "Metrics test task 2"
task3: shell echo "Metrics test task 3"

task2 after task1
task3 after task2"#;

    let dag_path = tester.create_test_dag(dag_content, "metrics_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    assert!(
        stdout.contains("📊 Execution Summary"),
        "Should show execution summary"
    );
    assert!(
        stdout.contains("Total tasks:") && stdout.contains("Successful:"),
        "Should show task metrics"
    );
    assert!(
        stdout.contains("Total time:"),
        "Should show timing information"
    );
}

#[test]
fn test_native_executor_progress_indicators() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: progress_test

task1: shell echo "Progress test 1"
task2: shell echo "Progress test 2"

task2 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "progress_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    assert!(
        stdout.contains("🔄 Starting task:") || stdout.contains("Starting"),
        "Should show task start indicators"
    );
    assert!(
        stdout.contains("✅") || stdout.contains("completed"),
        "Should show completion indicators"
    );
}

// ============================================================================
// INTEGRATION WITH OTHER CLI FEATURES
// ============================================================================

#[test]
fn test_native_executor_with_validation_skip() {
    let tester = NativeExecutorCLITester::new();

    let dag_content = r#"DAG: validation_skip_test

task1: shell echo "Validation skip test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "validation_skip.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--no-validate"
    ]);

    assert_eq!(exit_code, 0, "Should execute without validation");
    assert!(
        stdout.contains("🦀 Using native Rust executor"),
        "Should use native executor"
    );
    // Should not contain validation messages
}

#[test]
fn test_native_executor_help_integration() {
    let tester = NativeExecutorCLITester::new();

    let (exit_code, stdout, stderr) = tester.run_command(&["help", "run"]);

    assert_eq!(exit_code, 0, "Help should work");
    assert!(
        stdout.contains("native-executor") || stdout.contains("native"),
        "Help should mention native executor flag"
    );
    assert!(
        stdout.contains("python"),
        "Help should mention python flag"
    );
}