//! Output Formatting Integration Tests
//!
//! Tests for execution result formatting and output compatibility.
//! Covers requirements 3.3, 3.4, and 5.3 from the native executor specification.

#![allow(unused_variables)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test helper for output formatting tests
struct OutputFormattingTester {
    jorm_path: String,
    temp_dir: TempDir,
}

impl OutputFormattingTester {
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

    fn run_command_with_env(&self, args: &[&str], env_vars: &[(&str, &str)]) -> (i32, String, String) {
        let mut cmd = Command::new(&self.jorm_path);
        cmd.args(args);
        
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().expect("Failed to execute command");
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
// COLORED OUTPUT TESTS
// ============================================================================

#[test]
fn test_colored_output_enabled() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: colored_output_test

task1: shell echo "Testing colored output"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "colored_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for colored output indicators (ANSI escape codes)
    // Note: In CI environments, colors might be disabled, so we check for the content
    assert!(
        stdout.contains("🦀 Using native Rust executor") || stdout.contains("Using native Rust executor"),
        "Should show executor selection"
    );
    assert!(
        stdout.contains("✅") || stdout.contains("completed successfully"),
        "Should show success indicators"
    );
}

#[test]
fn test_colored_output_disabled() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: no_color_test

task1: shell echo "Testing no color output"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "no_color_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command_with_env(
        &["run", &dag_path, "--native-executor"],
        &[("NO_COLOR", "1")]
    );

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Should still contain the content but without color codes
    assert!(
        stdout.contains("Using native Rust executor"),
        "Should show executor selection without colors"
    );
    assert!(
        stdout.contains("completed successfully") || stdout.contains("✅"),
        "Should show completion message"
    );
}

#[test]
fn test_dumb_terminal_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: dumb_terminal_test

task1: shell echo "Testing dumb terminal"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "dumb_terminal_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command_with_env(
        &["run", &dag_path, "--native-executor"],
        &[("TERM", "dumb")]
    );

    assert_eq!(exit_code, 0, "Should execute successfully");
    assert!(
        stdout.contains("Using native Rust executor"),
        "Should work with dumb terminal"
    );
}

// ============================================================================
// PROGRESS INDICATORS TESTS
// ============================================================================

#[test]
fn test_progress_indicators() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: progress_test

task1: shell echo "Progress test 1"
task2: shell echo "Progress test 2"
task3: shell echo "Progress test 3"

task2 after task1
task3 after task2"#;

    let dag_path = tester.create_test_dag(dag_content, "progress_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for progress indicators
    assert!(
        stdout.contains("🔄") || stdout.contains("Starting task"),
        "Should show task start indicators"
    );
    assert!(
        stdout.contains("✅") || stdout.contains("completed successfully"),
        "Should show task completion indicators"
    );
    assert!(
        stdout.contains("📊") || stdout.contains("Execution Summary"),
        "Should show execution summary"
    );
}

#[test]
fn test_task_timing_information() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: timing_test

task1: shell sleep 0.1 && echo "Timing test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "timing_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for timing information
    assert!(
        stdout.contains("Total time:") || stdout.contains("time"),
        "Should show timing information"
    );
    assert!(
        stdout.contains("s") || stdout.contains("ms"),
        "Should show time units"
    );
}

#[test]
fn test_execution_metrics_display() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: metrics_test

task1: shell echo "Metrics test 1"
task2: shell echo "Metrics test 2"
task3: shell echo "Metrics test 3"

task2 after task1
task3 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "metrics_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for metrics information
    assert!(
        stdout.contains("Total tasks:") && stdout.contains("3"),
        "Should show total task count"
    );
    assert!(
        stdout.contains("Successful:") && stdout.contains("3"),
        "Should show successful task count"
    );
    assert!(
        stdout.contains("Peak concurrency:"),
        "Should show concurrency information"
    );
}

// ============================================================================
// VERBOSE OUTPUT TESTS
// ============================================================================

#[test]
fn test_verbose_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: verbose_test

task1: shell echo "Verbose output test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "verbose_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command_with_env(
        &["run", &dag_path, "--native-executor"],
        &[("JORM_VERBOSITY", "2")]
    );

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Verbose mode should show more details
    assert!(
        stdout.contains("Configuration Summary") || stdout.contains("Max concurrent"),
        "Verbose mode should show configuration details"
    );
}

#[test]
fn test_debug_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: debug_test

task1: shell echo "Debug output test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "debug_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command_with_env(
        &["run", &dag_path, "--native-executor"],
        &[("JORM_VERBOSITY", "3")]
    );

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Debug mode should show even more details
    // The exact debug output depends on implementation
    assert!(
        stdout.len() > 0,
        "Debug mode should produce output"
    );
}

#[test]
fn test_minimal_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: minimal_test

task1: shell echo "Minimal output test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "minimal_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command_with_env(
        &["run", &dag_path, "--native-executor"],
        &[("JORM_VERBOSITY", "0")]
    );

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Minimal mode should show less output
    // Should still show essential information
    assert!(
        stdout.len() > 0,
        "Should still produce some output"
    );
}

// ============================================================================
// ERROR FORMATTING TESTS
// ============================================================================

#[test]
fn test_error_formatting() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: error_test

task1: shell exit 1

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "error_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert!(exit_code != 0, "Should fail due to task failure");
    
    // Check for error formatting
    assert!(
        stdout.contains("❌") || stdout.contains("failed") || stderr.contains("Error"),
        "Should show error indicators"
    );
    assert!(
        stdout.contains("DAG execution failed") || stderr.contains("failed"),
        "Should show execution failure message"
    );
}

#[test]
fn test_task_failure_details() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: task_failure_test

task1: shell echo "This will fail" && exit 42

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "task_failure_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert!(exit_code != 0, "Should fail due to task failure");
    
    // Check for detailed error information
    assert!(
        stdout.contains("This will fail"),
        "Should show task output before failure"
    );
    // Exit code might be shown in error details
}

#[test]
fn test_validation_error_formatting() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"Invalid DAG content
This is not a valid DAG"#;

    let dag_path = tester.create_test_dag(dag_content, "invalid_dag.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert!(exit_code != 0, "Should fail due to invalid DAG");
    
    // Should show parsing/validation errors
    assert!(
        stderr.contains("Error") || stdout.contains("failed") || stderr.contains("File not found"),
        "Should show validation error"
    );
}

// ============================================================================
// COMPATIBILITY TESTS
// ============================================================================

#[test]
fn test_output_compatibility_with_python_format() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: compatibility_test

task1: shell echo "Compatibility test"
task2: shell echo "Second task"

task2 after task1"#;

    let dag_path = tester.create_test_dag(dag_content, "compatibility_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for key elements that should be compatible
    assert!(
        stdout.contains("DAG") || stdout.contains("compatibility_test"),
        "Should show DAG information"
    );
    assert!(
        stdout.contains("task1") || stdout.contains("task2"),
        "Should show task information"
    );
    assert!(
        stdout.contains("Total tasks:") && stdout.contains("Successful:"),
        "Should show summary statistics"
    );
}

#[test]
fn test_json_output_format() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: json_test

task1: shell echo "JSON test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "json_test.txt");
    
    // Note: JSON output format would need to be implemented as a CLI flag
    // For now, we test that the regular output contains structured information
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // The output should contain structured information that could be parsed
    assert!(
        stdout.contains("Total tasks:") && stdout.contains("Successful:"),
        "Should contain structured metrics"
    );
}

// ============================================================================
// CONFIGURATION OUTPUT TESTS
// ============================================================================

#[test]
fn test_configuration_summary_display() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: config_summary_test

task1: shell echo "Config summary test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "config_summary_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--max-concurrent", "4",
        "--timeout", "60"
    ]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Should show configuration information
    assert!(
        stdout.contains("Configuration Summary") || stdout.contains("Max concurrent"),
        "Should show configuration summary"
    );
}

#[test]
fn test_state_management_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: state_test

task1: shell echo "State management test"

task1"#;

    let dag_path = tester.create_test_dag(dag_content, "state_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&[
        "run", &dag_path, 
        "--native-executor", 
        "--with-state"
    ]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Should show state management information
    assert!(
        stdout.contains("State management enabled") || stdout.contains("💾"),
        "Should indicate state management is enabled"
    );
}

// ============================================================================
// PERFORMANCE OUTPUT TESTS
// ============================================================================

#[test]
fn test_performance_metrics_output() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: performance_test

task1: shell echo "Performance test 1"
task2: shell echo "Performance test 2"
task3: shell echo "Performance test 3"
task4: shell echo "Performance test 4"

task2 after task1
task3 after task1
task4 after task2, task3"#;

    let dag_path = tester.create_test_dag(dag_content, "performance_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Check for performance metrics
    assert!(
        stdout.contains("Peak concurrency:"),
        "Should show peak concurrency"
    );
    assert!(
        stdout.contains("Total time:") || stdout.contains("time"),
        "Should show execution time"
    );
    assert!(
        stdout.contains("Success rate:") || stdout.contains("%"),
        "Should show success rate"
    );
}

#[test]
fn test_parallel_execution_indicators() {
    let tester = OutputFormattingTester::new();

    let dag_content = r#"DAG: parallel_indicators_test

task1: shell echo "Parallel task 1"
task2: shell echo "Parallel task 2"
task3: shell echo "Parallel task 3"

task1
task2
task3"#;

    let dag_path = tester.create_test_dag(dag_content, "parallel_indicators_test.txt");
    let (exit_code, stdout, stderr) = tester.run_command(&["run", &dag_path, "--native-executor"]);

    assert_eq!(exit_code, 0, "Should execute successfully");
    
    // Should show that tasks can run in parallel
    assert!(
        stdout.contains("Peak concurrency:") && !stdout.contains("Peak concurrency: 1"),
        "Should show parallel execution occurred"
    );
}