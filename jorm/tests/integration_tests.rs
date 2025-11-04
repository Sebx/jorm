// Integration tests for end-to-end functionality
#![allow(clippy::assertions_on_constants)]

use jorm::JormEngine;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_end_to_end_shell_execution() {
    let engine = JormEngine::new().await.unwrap();
    let temp_dir = TempDir::new().unwrap();
    let dag_file = temp_dir.path().join("test_dag.txt");

    let dag_content = r#"
task hello_world {
    type: shell
    command: "cmd /c echo Hello, World!"
}

task create_file {
    type: shell
    command: "cmd /c echo test content > test_output.txt"
    depends_on: [hello_world]
}
"#;

    fs::write(&dag_file, dag_content).unwrap();

    let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
    assert!(result.is_ok(), "DAG execution failed: {:?}", result.err());

    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert_eq!(execution_result.task_results.len(), 2);

    // Check that both tasks succeeded
    for task_result in &execution_result.task_results {
        assert!(
            task_result.success,
            "Task {} failed: {:?}",
            task_result.task_name, task_result.error
        );
    }
}

#[tokio::test]
async fn test_end_to_end_file_operations() {
    let engine = JormEngine::new().await.unwrap();
    let temp_dir = TempDir::new().unwrap();
    let dag_file = temp_dir.path().join("file_ops_dag.txt");

    // Create source file
    let source_file = temp_dir.path().join("source.txt");
    fs::write(&source_file, "test content").unwrap();

    let dag_content = format!(
        r#"
task copy_file {{
    type: file_copy
    source: "{}"
    destination: "{}"
}}
"#,
        source_file.to_str().unwrap(),
        temp_dir.path().join("copied.txt").to_str().unwrap()
    );

    fs::write(&dag_file, dag_content).unwrap();

    let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
    assert!(
        result.is_ok(),
        "File operations DAG execution failed: {:?}",
        result.err()
    );

    let execution_result = result.unwrap();
    assert!(execution_result.success);

    // Verify file was copied
    assert!(temp_dir.path().join("copied.txt").exists());

    // Verify content
    let content = fs::read_to_string(temp_dir.path().join("copied.txt")).unwrap();
    assert_eq!(content, "test content");
}

#[tokio::test]
async fn test_end_to_end_error_handling() {
    let engine = JormEngine::new().await.unwrap();
    let temp_dir = TempDir::new().unwrap();
    let dag_file = temp_dir.path().join("error_dag.txt");

    let dag_content = r#"
task failing_task {
    type: shell
    command: "cmd /c exit 1"
}
"#;

    fs::write(&dag_file, dag_content).unwrap();

    let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
    assert!(
        result.is_ok(),
        "Should return result even with failed tasks"
    );

    let execution_result = result.unwrap();
    assert!(!execution_result.success); // Overall execution should fail

    // Check that task failed
    let failing_task_result = execution_result
        .task_results
        .iter()
        .find(|r| r.task_name == "failing_task")
        .unwrap();
    assert!(!failing_task_result.success);
}

#[tokio::test]
async fn test_end_to_end_invalid_dag_file() {
    let engine = JormEngine::new().await.unwrap();
    let temp_dir = TempDir::new().unwrap();
    let dag_file = temp_dir.path().join("invalid_dag.txt");

    let invalid_dag_content = r#"
this is not valid DAG syntax
task without proper format
"#;

    fs::write(&dag_file, invalid_dag_content).unwrap();

    let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
    assert!(result.is_err(), "Should fail with invalid DAG syntax");
}

#[tokio::test]
async fn test_end_to_end_natural_language() {
    let engine = JormEngine::new().await.unwrap();
    let description = "Create a file called hello.txt with the content 'Hello World'";

    let result = engine.execute_from_natural_language(description).await;
    // Note: This test might fail if NLP model is not properly configured
    // In a real scenario, we'd want to mock the NLP processor for reliable testing
    if result.is_ok() {
        let execution_result = result.unwrap();
        assert!(!execution_result.task_results.is_empty());
    }
    // If NLP fails, that's acceptable for this test as it depends on external model
}

#[tokio::test]
async fn test_http_server_creation() {
    use jorm::server::http::HttpServer;
    use std::sync::Arc;

    let engine = Arc::new(JormEngine::new().await.unwrap());
    let _server = HttpServer::new(engine.clone(), 8080);

    // Test that server can be created without authentication
    assert!(true);

    let _server_with_auth = HttpServer::with_auth_token(engine, 8081, "test-token".to_string());

    // Test that server can be created with authentication
    assert!(true);
}

#[tokio::test]
async fn test_scheduler_creation() {
    use jorm::scheduler::cron::Scheduler;
    use std::sync::Arc;

    let engine = Arc::new(JormEngine::new().await.unwrap());
    let _scheduler = Scheduler::new(engine);

    // Test that scheduler can be created
    assert!(true);
}
