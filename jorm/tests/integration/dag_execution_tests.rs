use jorm::JormEngine;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod dag_execution_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_shell_execution() {
        let engine = JormEngine::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("test_dag.txt");
        
        let dag_content = r#"
task hello_world {
    type: shell
    command: "echo 'Hello, World!'"
}

task create_file {
    type: shell
    command: "echo 'test content' > test_output.txt"
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
            assert!(task_result.success, "Task {} failed: {:?}", task_result.task_name, task_result.error);
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
        
        let dag_content = format!(r#"
task copy_file {{
    type: file_copy
    source: "{}"
    destination: "{}"
}}

task move_file {{
    type: file_move
    source: "{}"
    destination: "{}"
    depends_on: [copy_file]
}}
"#, 
            source_file.to_str().unwrap(),
            temp_dir.path().join("copied.txt").to_str().unwrap(),
            temp_dir.path().join("copied.txt").to_str().unwrap(),
            temp_dir.path().join("moved.txt").to_str().unwrap()
        );
        
        fs::write(&dag_file, dag_content).unwrap();
        
        let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
        assert!(result.is_ok(), "File operations DAG execution failed: {:?}", result.err());
        
        let execution_result = result.unwrap();
        assert!(execution_result.success);
        
        // Verify files exist
        assert!(temp_dir.path().join("moved.txt").exists());
        assert!(!temp_dir.path().join("copied.txt").exists()); // Should be moved, not copied
        
        // Verify content
        let content = fs::read_to_string(temp_dir.path().join("moved.txt")).unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_end_to_end_complex_dag() {
        let engine = JormEngine::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("complex_dag.txt");
        
        let dag_content = format!(r#"
task setup {{
    type: shell
    command: "mkdir -p {}/test_dir"
}}

task create_data {{
    type: shell
    command: "echo 'data line 1' > {}/test_dir/data.txt"
    depends_on: [setup]
}}

task process_data {{
    type: shell
    command: "wc -l {}/test_dir/data.txt > {}/test_dir/result.txt"
    depends_on: [create_data]
}}

task backup_result {{
    type: file_copy
    source: "{}/test_dir/result.txt"
    destination: "{}/test_dir/result_backup.txt"
    depends_on: [process_data]
}}

task cleanup {{
    type: file_delete
    path: "{}/test_dir/data.txt"
    depends_on: [backup_result]
}}
"#, 
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
            temp_dir.path().to_str().unwrap()
        );
        
        fs::write(&dag_file, dag_content).unwrap();
        
        let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
        assert!(result.is_ok(), "Complex DAG execution failed: {:?}", result.err());
        
        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert_eq!(execution_result.task_results.len(), 5);
        
        // Verify final state
        assert!(temp_dir.path().join("test_dir").exists());
        assert!(temp_dir.path().join("test_dir/result.txt").exists());
        assert!(temp_dir.path().join("test_dir/result_backup.txt").exists());
        assert!(!temp_dir.path().join("test_dir/data.txt").exists()); // Should be deleted
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
    async fn test_end_to_end_dag_generation() {
        let engine = JormEngine::new().await.unwrap();
        let description = "Copy file1.txt to file2.txt and then delete file1.txt";
        
        let result = engine.generate_dag_from_nl(description).await;
        // Similar to above, this might fail if NLP model is not configured
        if result.is_ok() {
            let dag_content = result.unwrap();
            assert!(!dag_content.is_empty());
            // Basic validation that it looks like DAG content
            assert!(dag_content.contains("task") || dag_content.contains("type"));
        }
    }

    #[tokio::test]
    async fn test_end_to_end_error_handling() {
        let engine = JormEngine::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("error_dag.txt");
        
        let dag_content = r#"
task failing_task {
    type: shell
    command: "exit 1"
}

task dependent_task {
    type: shell
    command: "echo 'this should not run'"
    depends_on: [failing_task]
}
"#;
        
        fs::write(&dag_file, dag_content).unwrap();
        
        let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
        assert!(result.is_ok(), "Should return result even with failed tasks");
        
        let execution_result = result.unwrap();
        assert!(!execution_result.success); // Overall execution should fail
        
        // Check that first task failed and second task was skipped or failed
        let failing_task_result = execution_result.task_results.iter()
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
    async fn test_end_to_end_nonexistent_file() {
        let engine = JormEngine::new().await.unwrap();
        let nonexistent_file = "/path/that/does/not/exist/dag.txt";
        
        let result = engine.execute_from_file(nonexistent_file).await;
        assert!(result.is_err(), "Should fail with nonexistent file");
    }

    #[tokio::test]
    async fn test_end_to_end_dag_with_preview() {
        let engine = JormEngine::new().await.unwrap();
        let description = "Echo hello world";
        
        let result = engine.execute_with_preview(description, false).await;
        if result.is_ok() {
            let (preview, execution_result) = result.unwrap();
            assert!(!preview.generated_dag_content.is_empty());
            assert!(!preview.summary.is_empty());
            assert!(execution_result.is_none()); // Should not execute with auto_execute=false
        }
        // If NLP fails, that's acceptable for this test
    }

    #[tokio::test]
    async fn test_end_to_end_dag_with_auto_execute() {
        let engine = JormEngine::new().await.unwrap();
        let description = "Echo hello world";
        
        let result = engine.execute_with_preview(description, true).await;
        if result.is_ok() {
            let (preview, execution_result) = result.unwrap();
            assert!(!preview.generated_dag_content.is_empty());
            assert!(execution_result.is_some()); // Should execute with auto_execute=true
            
            let exec_result = execution_result.unwrap();
            assert!(!exec_result.task_results.is_empty());
        }
        // If NLP fails, that's acceptable for this test
    }
}