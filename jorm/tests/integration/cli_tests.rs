//! CLI Integration Tests
//!
//! Comprehensive integration tests for the Jorm CLI functionality.
//! Tests all CLI commands, error handling, and command-line interface behavior.

#![allow(unused_variables)]

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Test helper for running CLI commands
struct CLITester {
    jorm_path: String,
    temp_dir: TempDir,
}

impl CLITester {
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

    fn run_command_with_input(&self, args: &[&str], input: &str) -> (i32, String, String) {
        let mut child = Command::new(&self.jorm_path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read output");
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
// BASIC CLI COMMAND TESTS
// ============================================================================

#[test]
fn test_version_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["version"]);

    assert_eq!(exit_code, 0, "Version command should succeed");
    assert!(
        stdout.contains("JORM") || stdout.contains("jorm"),
        "Version output should contain 'JORM' or 'jorm'"
    );
    assert!(
        stderr.is_empty(),
        "Version command should not produce stderr"
    );
}

#[test]
fn test_help_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["help"]);

    assert_eq!(exit_code, 0, "Help command should succeed");
    assert!(
        stdout.contains("Usage") || stdout.contains("Commands"),
        "Help output should contain usage information"
    );
    assert!(stderr.is_empty(), "Help command should not produce stderr");
}

#[test]
fn test_list_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["list"]);

    // List command might return 0 (success) or 1 (no DAGs found)
    assert!(
        exit_code == 0 || exit_code == 1,
        "List command should return 0 or 1"
    );
    // Don't assert on stdout content as it depends on directory contents
}

#[test]
fn test_status_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["status"]);

    // Status command should always succeed
    assert_eq!(exit_code, 0, "Status command should succeed");
    // Don't assert on stdout content as it depends on system state
}

// ============================================================================
// DAG VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_valid_dag() {
    let tester = CLITester::new();

    let dag_content = r#"dag: test_dag

tasks:
- task1
  type: shell
  description: Test task
  command: echo "Hello"

dependencies:
- task1"#;

    let dag_path = tester.create_test_dag(dag_content, "valid_dag.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["validate", &dag_path]);

    assert_eq!(exit_code, 0, "Valid DAG should pass validation");
    assert!(
        stdout.contains("valid") || stdout.contains("success"),
        "Validation should report success"
    );
}

#[test]
fn test_validate_invalid_dag() {
    let tester = CLITester::new();

    let dag_content = r#"invalid dag content
this is not a valid DAG"#;

    let dag_path = tester.create_test_dag(dag_content, "invalid_dag.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["validate", &dag_path]);

    // Invalid DAG should fail validation
    assert!(exit_code != 0, "Invalid DAG should fail validation");
}

#[test]
fn test_validate_nonexistent_file() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["validate", "nonexistent.txt"]);

    // Non-existent file should fail
    assert!(exit_code != 0, "Non-existent file should fail validation");
}

// ============================================================================
// DAG EXECUTION TESTS
// ============================================================================

#[test]
fn test_run_simple_dag() {
    let tester = CLITester::new();

    let dag_content = r#"dag: simple_test

tasks:
- hello_task
  type: shell
  description: Say hello
  command: echo "Hello from DAG execution"

dependencies:
- hello_task"#;

    let dag_path = tester.create_test_dag(dag_content, "simple_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path]);

    // DAG execution should succeed
    assert_eq!(exit_code, 0, "Simple DAG should execute successfully");
    assert!(
        stdout.contains("Hello from DAG execution"),
        "DAG output should contain expected text"
    );
}

#[test]
fn test_run_dag_with_dependencies() {
    let tester = CLITester::new();

    let dag_content = r#"dag: dependency_test

tasks:
- task1
  type: shell
  description: First task
  command: echo "Task 1 executed"

- task2
  type: shell
  description: Second task
  command: echo "Task 2 executed"

dependencies:
- task2 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "dependency_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path]);

    assert_eq!(
        exit_code, 0,
        "DAG with dependencies should execute successfully"
    );
    assert!(
        stdout.contains("Task 1 executed"),
        "First task should execute"
    );
    assert!(
        stdout.contains("Task 2 executed"),
        "Second task should execute"
    );
}

#[test]
fn test_run_failing_dag() {
    let tester = CLITester::new();

    let dag_content = r#"dag: failing_test

tasks:
- failing_task
  type: shell
  description: This will fail
  command: exit 1

dependencies:
- failing_task"#;

    let dag_path = tester.create_test_dag(dag_content, "failing_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path]);

    // Failing DAG should return non-zero exit code
    assert!(
        exit_code != 0,
        "Failing DAG should return non-zero exit code"
    );
}

// ============================================================================
// DAG DESCRIPTION TESTS
// ============================================================================

#[test]
fn test_describe_dag() {
    let tester = CLITester::new();

    let dag_content = r#"dag: describe_test

tasks:
- task1
  type: shell
  description: First task
  command: echo "Hello"

- task2
  type: shell
  description: Second task
  command: echo "World"

dependencies:
- task2 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "describe_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["describe", &dag_path]);

    assert_eq!(exit_code, 0, "Describe command should succeed");
    assert!(
        stdout.contains("describe_test"),
        "Description should contain DAG name"
    );
    assert!(
        stdout.contains("task1") || stdout.contains("task2"),
        "Description should contain task names"
    );
}

// ============================================================================
// AI FEATURE TESTS
// ============================================================================

#[test]
fn test_model_info_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["model-info"]);

    // Model info might succeed or fail depending on AI setup
    // Just verify it doesn't crash
    assert!(
        exit_code == 0 || exit_code == 1,
        "Model info should not crash"
    );
}

#[test]
fn test_analyze_command() {
    let tester = CLITester::new();

    let dag_content = r#"dag: analyze_test

tasks:
- task1
  type: shell
  command: echo "Hello"

dependencies:
- task1"#;

    let dag_path = tester.create_test_dag(dag_content, "analyze_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["analyze", &dag_path]);

    // Analyze might succeed or fail depending on AI setup
    assert!(exit_code == 0 || exit_code == 1, "Analyze should not crash");
}

#[test]
fn test_generate_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) =
        tester.run_command(&["generate", "simple data processing pipeline"]);

    // Generate might succeed or fail depending on AI setup
    assert!(
        exit_code == 0 || exit_code == 1,
        "Generate should not crash"
    );
}

// ============================================================================
// SCHEDULER FEATURE TESTS
// ============================================================================

#[test]
fn test_schedule_command() {
    let tester = CLITester::new();

    let dag_content = r#"dag: schedule_test

tasks:
- task1
  type: shell
  command: echo "Scheduled task"

dependencies:
- task1"#;

    let dag_path = tester.create_test_dag(dag_content, "schedule_test.txt");
    let (exit_code, stdout, stderr) =
        tester.run_command(&["schedule", &dag_path, "--cron", "every 1 minute"]);

    // Schedule might succeed or fail depending on scheduler setup
    assert!(
        exit_code == 0 || exit_code == 1,
        "Schedule should not crash"
    );
}

#[test]
fn test_daemon_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["daemon"]);

    // Daemon might succeed or fail depending on system setup
    assert!(exit_code == 0 || exit_code == 1, "Daemon should not crash");
}

#[test]
fn test_jobs_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["jobs"]);

    // Jobs command should always succeed
    assert_eq!(exit_code, 0, "Jobs command should succeed");
}

#[test]
fn test_trigger_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["trigger", "nonexistent_job"]);

    // Trigger should fail for non-existent job
    assert!(exit_code != 0, "Trigger should fail for non-existent job");
}

#[test]
fn test_stop_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["stop"]);

    // Stop might succeed or fail depending on daemon state
    assert!(exit_code == 0 || exit_code == 1, "Stop should not crash");
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_invalid_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["invalid_command"]);

    // Invalid command should fail
    assert!(exit_code != 0, "Invalid command should fail");
}

#[test]
fn test_missing_arguments() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["run"]);

    // Missing arguments should fail
    assert!(exit_code != 0, "Missing arguments should fail");
}

#[test]
fn test_help_for_specific_command() {
    let tester = CLITester::new();
    let (exit_code, stdout, stderr) = tester.run_command(&["help", "run"]);

    // Help for specific command should succeed
    assert_eq!(exit_code, 0, "Help for specific command should succeed");
    assert!(
        stdout.contains("run") || stdout.contains("usage"),
        "Help should contain command information"
    );
}

// ============================================================================
// INTERACTIVE MODE TESTS
// ============================================================================

#[test]
fn test_interactive_mode_startup() {
    let tester = CLITester::new();

    // Test interactive mode startup (should not crash)
    let (exit_code, stdout, stderr) = tester.run_command_with_input(&["interactive"], "exit\n");

    // Interactive mode should start successfully
    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive mode should not crash"
    );
}

#[test]
fn test_interactive_help_command() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "help\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive help should not crash"
    );
    assert!(
        stdout.contains("help") || stdout.contains("command"),
        "Interactive help should show help information"
    );
}

#[test]
fn test_interactive_version_command() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "version\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive version should not crash"
    );
    assert!(
        stdout.contains("JORM") || stdout.contains("jorm"),
        "Interactive version should show version"
    );
}

#[test]
fn test_interactive_list_command() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "list\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive list should not crash"
    );
}

#[test]
fn test_interactive_dag_operations() {
    let tester = CLITester::new();

    let dag_content = r#"dag: interactive_test

tasks:
- task1
  type: shell
  command: echo "Interactive test"

dependencies:
- task1"#;

    let dag_path = tester.create_test_dag(dag_content, "interactive_test.txt");

    let input = format!("run {dag_path}\nexit\n");
    let (exit_code, stdout, stderr) = tester.run_command_with_input(&["interactive"], &input);

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive DAG operations should not crash"
    );
}

#[test]
fn test_interactive_ai_features() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "analyze\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive AI features should not crash"
    );
}

#[test]
fn test_interactive_scheduler_operations() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "jobs\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive scheduler operations should not crash"
    );
}

#[test]
fn test_interactive_natural_language() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "create a simple data pipeline\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive natural language should not crash"
    );
}

#[test]
fn test_interactive_error_handling() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "invalid_command\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive error handling should not crash"
    );
}

#[test]
fn test_interactive_features() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "history\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive features should not crash"
    );
}

#[test]
fn test_interactive_clear_command() {
    let tester = CLITester::new();

    let (exit_code, stdout, stderr) =
        tester.run_command_with_input(&["interactive"], "clear\nexit\n");

    assert!(
        exit_code == 0 || exit_code == 1,
        "Interactive clear should not crash"
    );
}
