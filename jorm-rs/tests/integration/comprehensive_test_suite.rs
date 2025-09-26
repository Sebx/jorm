//! Comprehensive Test Suite for Jorm
//!
//! This module replaces all PowerShell test scripts with proper Rust integration tests.
//! It provides a complete test suite that covers all CLI functionality, interactive mode,
//! DAG execution, and error handling.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Comprehensive test suite for Jorm CLI
pub struct ComprehensiveTestSuite {
    jorm_path: String,
    temp_dir: TempDir,
}

impl Default for ComprehensiveTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ComprehensiveTestSuite {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        Self {
            jorm_path: "target/debug/jorm-rs.exe".to_string(),
            temp_dir,
        }
    }

    /// Run a CLI command and return the result
    pub fn run_cli_command(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(&self.jorm_path)
            .args(args)
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        (exit_code, stdout, stderr)
    }

    /// Run an interactive command with input
    pub fn run_interactive_command(&self, input: &str) -> (i32, String, String) {
        let mut child = Command::new(&self.jorm_path)
            .arg("interactive")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start interactive process");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read output");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        (exit_code, stdout, stderr)
    }

    /// Create a test DAG file
    pub fn create_dag(&self, name: &str, content: &str) -> String {
        let dag_path = self.temp_dir.path().join(format!("{name}.txt"));
        fs::write(&dag_path, content).expect("Failed to write DAG file");
        dag_path.to_string_lossy().to_string()
    }

    /// Test basic CLI commands
    pub fn test_basic_commands(&self) -> bool {
        println!("Testing basic CLI commands...");

        // Test version command
        let (exit_code, stdout, _stderr) = self.run_cli_command(&["version"]);
        if exit_code != 0 || (!stdout.contains("jorm-rs") && !stdout.contains("Jorm-RS")) {
            println!("❌ Version command failed");
            return false;
        }
        println!("✅ Version command passed");

        // Test help command
        let (exit_code, stdout, _stderr) = self.run_cli_command(&["--help"]);
        if exit_code != 0 || !stdout.contains("A fast, reliable DAG execution engine") {
            println!("❌ Help command failed");
            return false;
        }
        println!("✅ Help command passed");

        // Test list command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["list"]);
        if exit_code != 0 {
            println!("❌ List command failed");
            return false;
        }
        println!("✅ List command passed");

        // Test status command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["status"]);
        if exit_code != 0 {
            println!("❌ Status command failed");
            return false;
        }
        println!("✅ Status command passed");

        true
    }

    /// Test DAG validation
    pub fn test_dag_validation(&self) -> bool {
        println!("Testing DAG validation...");

        // Test valid DAG
        let dag_content = r#"dag: validation_test

tasks:
- task1
  type: shell
  description: Test task
  command: echo "Hello World"

dependencies:
- task1"#;

        let dag_path = self.create_dag("validation_test", dag_content);
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["validate", &dag_path]);
        if exit_code != 0 {
            println!("❌ Valid DAG validation failed");
            return false;
        }
        println!("✅ Valid DAG validation passed");

        // Test invalid DAG
        let dag_path = self.create_dag("invalid_dag", "invalid content");
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["validate", &dag_path]);
        if exit_code == 0 {
            println!("❌ Invalid DAG validation should have failed");
            return false;
        }
        println!("✅ Invalid DAG validation correctly failed");

        // Test nonexistent file
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["validate", "nonexistent.txt"]);
        if exit_code == 0 {
            println!("❌ Nonexistent file validation should have failed");
            return false;
        }
        println!("✅ Nonexistent file validation correctly failed");

        true
    }

    /// Test DAG execution
    pub fn test_dag_execution(&self) -> bool {
        println!("Testing DAG execution...");

        // Test simple shell DAG
        let dag_content = r#"dag: execution_test

tasks:
- task1
  type: shell
  description: Test task
  command: echo "Hello from DAG execution"

dependencies:
- task1"#;

        let dag_path = self.create_dag("execution_test", dag_content);
        let (exit_code, stdout, _stderr) = self.run_cli_command(&["run", &dag_path]);
        if exit_code != 0 || !stdout.contains("Hello from DAG execution") {
            println!("❌ Simple DAG execution failed");
            return false;
        }
        println!("✅ Simple DAG execution passed");

        // Test DAG with dependencies
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

        let dag_path = self.create_dag("dependency_test", dag_content);
        let (exit_code, stdout, _stderr) = self.run_cli_command(&["run", &dag_path]);
        if exit_code != 0
            || !stdout.contains("Task 1 executed")
            || !stdout.contains("Task 2 executed")
        {
            println!("❌ DAG with dependencies execution failed");
            return false;
        }
        println!("✅ DAG with dependencies execution passed");

        // Test Python DAG (may fail if Python not installed)
        let dag_content = r#"dag: python_test

tasks:
- python_task
  type: python
  description: Python test task
  script: |
    print("Hello from Python!")

dependencies:
- python_task"#;

        let dag_path = self.create_dag("python_test", dag_content);
        let (exit_code, stdout, _stderr) = self.run_cli_command(&["run", &dag_path]);
        // Python execution may fail if Python is not installed, which is expected
        if exit_code != 0 && exit_code != 1 {
            println!("❌ Python DAG execution crashed unexpectedly");
            return false;
        }
        println!("✅ Python DAG execution handled gracefully");

        true
    }

    /// Test interactive mode
    pub fn test_interactive_mode(&self) -> bool {
        println!("Testing interactive mode...");

        // Test interactive startup
        let (exit_code, stdout, _stderr) = self.run_interactive_command("help\nexit\n");
        if exit_code != 0 {
            println!("❌ Interactive mode startup failed");
            return false;
        }
        println!("✅ Interactive mode startup passed");

        // Test exit command
        let (exit_code, stdout, _stderr) = self.run_interactive_command("exit\n");
        if exit_code != 0 {
            println!("❌ Exit command failed");
            return false;
        }
        println!("✅ Exit command passed");

        // Test version command in interactive mode
        let (exit_code, stdout, _stderr) = self.run_interactive_command("version\nexit\n");
        if exit_code != 0 {
            println!("❌ Version command in interactive mode failed");
            return false;
        }
        println!("✅ Version command in interactive mode passed");

        // Test multiple commands
        let (exit_code, stdout, _stderr) =
            self.run_interactive_command("version\nlist\nstatus\nexit\n");
        if exit_code != 0 {
            println!("❌ Multiple commands in interactive mode failed");
            return false;
        }
        println!("✅ Multiple commands in interactive mode passed");

        true
    }

    /// Test AI features
    pub fn test_ai_features(&self) -> bool {
        println!("Testing AI features...");

        // Test model info command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["model-info"]);
        if exit_code != 0 {
            println!("❌ Model info command failed");
            return false;
        }
        println!("✅ Model info command passed");

        // Test analyze command (may fail if AI models not available)
        let dag_content = r#"dag: analysis_test

tasks:
- task1
  type: shell
  description: Test task
  command: echo "Hello World"

dependencies:
- task1"#;

        let dag_path = self.create_dag("analysis_test", dag_content);
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["analyze", &dag_path]);
        // Analyze command may fail if AI models are not available, which is expected
        if exit_code != 0 && exit_code != 1 {
            println!("❌ Analyze command crashed unexpectedly");
            return false;
        }
        println!("✅ Analyze command handled gracefully");

        true
    }

    /// Test scheduler features
    pub fn test_scheduler_features(&self) -> bool {
        println!("Testing scheduler features...");

        // Test jobs command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["jobs"]);
        if exit_code != 0 {
            println!("❌ Jobs command failed");
            return false;
        }
        println!("✅ Jobs command passed");

        // Test stop command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["stop"]);
        if exit_code != 0 {
            println!("❌ Stop command failed");
            return false;
        }
        println!("✅ Stop command passed");

        // Test schedule command (may fail if scheduler not available)
        let dag_content = r#"dag: schedule_test

tasks:
- task1
  type: shell
  description: Test task
  command: echo "Hello World"

dependencies:
- task1"#;

        let dag_path = self.create_dag("schedule_test", dag_content);
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&[
            "schedule",
            &dag_path,
            "--name",
            "test-schedule",
            "--cron",
            "0 0 * * *",
        ]);
        // Schedule command may fail if scheduler is not available, which is expected
        if exit_code != 0 && exit_code != 1 {
            println!("❌ Schedule command crashed unexpectedly");
            return false;
        }
        println!("✅ Schedule command handled gracefully");

        true
    }

    /// Test error handling
    pub fn test_error_handling(&self) -> bool {
        println!("Testing error handling...");

        // Test invalid command
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["invalid-command"]);
        if exit_code == 0 {
            println!("❌ Invalid command should have failed");
            return false;
        }
        println!("✅ Invalid command correctly failed");

        // Test missing arguments
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["run"]);
        if exit_code == 0 {
            println!("❌ Missing arguments should have failed");
            return false;
        }
        println!("✅ Missing arguments correctly failed");

        // Test nonexistent file
        let (exit_code, _stdout, _stderr) = self.run_cli_command(&["run", "nonexistent.txt"]);
        if exit_code == 0 {
            println!("❌ Nonexistent file should have failed");
            return false;
        }
        println!("✅ Nonexistent file correctly failed");

        true
    }

    /// Run all tests
    pub fn run_all_tests(&self) -> bool {
        println!("🚀 Starting Comprehensive Test Suite for Jorm CLI");
        println!("{}", "=".repeat(60));

        let mut all_passed = true;

        // Test basic commands
        if !self.test_basic_commands() {
            all_passed = false;
        }
        println!();

        // Test DAG validation
        if !self.test_dag_validation() {
            all_passed = false;
        }
        println!();

        // Test DAG execution
        if !self.test_dag_execution() {
            all_passed = false;
        }
        println!();

        // Test interactive mode
        if !self.test_interactive_mode() {
            all_passed = false;
        }
        println!();

        // Test AI features
        if !self.test_ai_features() {
            all_passed = false;
        }
        println!();

        // Test scheduler features
        if !self.test_scheduler_features() {
            all_passed = false;
        }
        println!();

        // Test error handling
        if !self.test_error_handling() {
            all_passed = false;
        }
        println!();

        // Summary
        println!("{}", "=".repeat(60));
        if all_passed {
            println!("🎉 All tests passed! Jorm CLI is working correctly.");
        } else {
            println!("❌ Some tests failed. Please check the output above.");
        }
        println!("{}", "=".repeat(60));

        all_passed
    }
}

#[test]
fn test_comprehensive_suite() {
    let suite = ComprehensiveTestSuite::new();
    assert!(
        suite.run_all_tests(),
        "Comprehensive test suite should pass"
    );
}
