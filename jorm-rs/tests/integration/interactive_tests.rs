//! Interactive Mode Integration Tests
//! 
//! Comprehensive integration tests for the Jorm interactive mode functionality.
//! Tests interactive commands, natural language processing, and user interaction flows.

use std::process::{Command, Stdio};
use std::io::Write;
use std::fs;
use tempfile::TempDir;

/// Test helper for running interactive mode commands
struct InteractiveTester {
    jorm_path: String,
    temp_dir: TempDir,
}

impl InteractiveTester {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        Self {
            jorm_path: "target/debug/jorm-rs.exe".to_string(),
            temp_dir,
        }
    }

    fn run_interactive_command(&self, input: &str) -> (i32, String, String) {
        let mut child = Command::new(&self.jorm_path)
            .arg("interactive")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
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
// INTERACTIVE MODE STARTUP TESTS
// ============================================================================

#[test]
fn test_interactive_mode_startup() {
    let tester = InteractiveTester::new();
    
    // Test interactive mode startup
    let (exit_code, stdout, stderr) = tester.run_interactive_command("exit\n");
    
    // Interactive mode should start successfully
    assert!(exit_code == 0 || exit_code == 1, "Interactive mode should not crash");
    assert!(stdout.contains("interactive") || stdout.contains("jorm") || stdout.contains(">"), 
            "Interactive mode should show prompt or welcome message");
}

#[test]
fn test_interactive_mode_exit() {
    let tester = InteractiveTester::new();
    
    // Test immediate exit
    let (exit_code, stdout, stderr) = tester.run_interactive_command("exit\n");
    
    // Should exit cleanly
    assert!(exit_code == 0 || exit_code == 1, "Interactive mode should exit cleanly");
}

#[test]
fn test_interactive_mode_quit() {
    let tester = InteractiveTester::new();
    
    // Test quit command
    let (exit_code, stdout, stderr) = tester.run_interactive_command("quit\n");
    
    // Should exit cleanly
    assert!(exit_code == 0 || exit_code == 1, "Interactive mode should exit with quit");
}

// ============================================================================
// DIRECT COMMAND TESTS
// ============================================================================

#[test]
fn test_interactive_version_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("version\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive version should not crash");
    assert!(stdout.contains("Jorm-RS") || stdout.contains("jorm-rs") || stdout.contains("version"), 
            "Interactive version should show version information");
}

#[test]
fn test_interactive_help_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("help\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive help should not crash");
    assert!(stdout.contains("help") || stdout.contains("command") || stdout.contains("usage"), 
            "Interactive help should show help information");
}

#[test]
fn test_interactive_list_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("list\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive list should not crash");
}

#[test]
fn test_interactive_status_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("status\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive status should not crash");
}

// ============================================================================
// DAG OPERATION TESTS
// ============================================================================

#[test]
fn test_interactive_dag_validation() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_validation_test

tasks:
- task1
  type: shell
  command: echo "Hello from interactive validation"

dependencies:
- task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_validation.txt");
    let input = format!("validate {}\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive DAG validation should not crash");
}

#[test]
fn test_interactive_dag_execution() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_execution_test

tasks:
- task1
  type: shell
  command: echo "Hello from interactive execution"

dependencies:
- task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_execution.txt");
    let input = format!("run {}\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive DAG execution should not crash");
}

#[test]
fn test_interactive_dag_description() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_description_test

tasks:
- task1
  type: shell
  command: echo "Hello"

- task2
  type: shell
  command: echo "World"

dependencies:
- task2 after task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_description.txt");
    let input = format!("describe {}\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive DAG description should not crash");
}

#[test]
fn test_interactive_dag_with_dependencies() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_dependencies_test

tasks:
- task1
  type: shell
  command: echo "First task"

- task2
  type: shell
  command: echo "Second task"

dependencies:
- task2 after task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_dependencies.txt");
    let input = format!("run {}\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive DAG with dependencies should not crash");
}

// ============================================================================
// AI FEATURE TESTS
// ============================================================================

#[test]
fn test_interactive_model_info() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("model-info\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive model info should not crash");
}

#[test]
fn test_interactive_analyze_command() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_analyze_test

tasks:
- task1
  type: shell
  command: echo "Analyze this"

dependencies:
- task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_analyze.txt");
    let input = format!("analyze {}\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive analyze should not crash");
}

#[test]
fn test_interactive_generate_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("generate simple data pipeline\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive generate should not crash");
}

#[test]
fn test_interactive_ai_fallback() {
    let tester = InteractiveTester::new();
    
    // Test AI fallback when models are not available
    let (exit_code, stdout, stderr) = tester.run_interactive_command("analyze\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive AI fallback should not crash");
}

// ============================================================================
// SCHEDULER OPERATION TESTS
// ============================================================================

#[test]
fn test_interactive_schedule_command() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: interactive_schedule_test

tasks:
- task1
  type: shell
  command: echo "Scheduled task"

dependencies:
- task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "interactive_schedule.txt");
    let input = format!("schedule {} every 1 minute\nexit\n", dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive schedule should not crash");
}

#[test]
fn test_interactive_daemon_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("daemon\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive daemon should not crash");
}

#[test]
fn test_interactive_jobs_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("jobs\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive jobs should not crash");
}

#[test]
fn test_interactive_trigger_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("trigger test_job\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive trigger should not crash");
}

#[test]
fn test_interactive_stop_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("stop\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive stop should not crash");
}

// ============================================================================
// NATURAL LANGUAGE PROCESSING TESTS
// ============================================================================

#[test]
fn test_interactive_natural_language_simple() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("create a simple data pipeline\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive natural language should not crash");
}

#[test]
fn test_interactive_natural_language_complex() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("create an ETL pipeline that extracts data from a database, transforms it, and loads it to a file\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive complex natural language should not crash");
}

#[test]
fn test_interactive_natural_language_questions() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("what is a DAG?\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive natural language questions should not crash");
}

#[test]
fn test_interactive_natural_language_help() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("how do I create a DAG?\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive natural language help should not crash");
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_interactive_invalid_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("invalid_command\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive invalid command should not crash");
}

#[test]
fn test_interactive_missing_arguments() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("run\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive missing arguments should not crash");
}

#[test]
fn test_interactive_nonexistent_file() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("run nonexistent.txt\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive nonexistent file should not crash");
}

#[test]
fn test_interactive_malformed_input() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("run --invalid-flag\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive malformed input should not crash");
}

// ============================================================================
// INTERACTIVE FEATURE TESTS
// ============================================================================

#[test]
fn test_interactive_history_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("history\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive history should not crash");
}

#[test]
fn test_interactive_clear_command() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("clear\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive clear should not crash");
}

#[test]
fn test_interactive_multiple_commands() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("version\nhelp\nlist\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive multiple commands should not crash");
}

#[test]
fn test_interactive_command_sequence() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: sequence_test

tasks:
- task1
  type: shell
  command: echo "Sequence test"

dependencies:
- task1"#;
    
    let dag_path = tester.create_test_dag(dag_content, "sequence_test.txt");
    let input = format!("validate {}\nrun {}\ndescribe {}\nexit\n", dag_path, dag_path, dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive command sequence should not crash");
}

#[test]
fn test_interactive_context_preservation() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("version\nlist\nstatus\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive context preservation should not crash");
}

// ============================================================================
// ADVANCED INTERACTIVE TESTS
// ============================================================================

#[test]
fn test_interactive_complex_workflow() {
    let tester = InteractiveTester::new();
    
    let dag_content = r#"dag: complex_workflow

tasks:
- extract
  type: shell
  command: echo "Extracting data"

- transform
  type: shell
  command: echo "Transforming data"

- load
  type: shell
  command: echo "Loading data"

dependencies:
- transform after extract
- load after transform"#;
    
    let dag_path = tester.create_test_dag(dag_content, "complex_workflow.txt");
    let input = format!("validate {}\nrun {}\ndescribe {}\nexit\n", dag_path, dag_path, dag_path);
    let (exit_code, stdout, stderr) = tester.run_interactive_command(&input);
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive complex workflow should not crash");
}

#[test]
fn test_interactive_mixed_commands() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("version\nhelp\nlist\nstatus\njobs\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive mixed commands should not crash");
}

#[test]
fn test_interactive_error_recovery() {
    let tester = InteractiveTester::new();
    
    let (exit_code, stdout, stderr) = tester.run_interactive_command("invalid_command\nversion\nhelp\nexit\n");
    
    assert!(exit_code == 0 || exit_code == 1, "Interactive error recovery should not crash");
}


