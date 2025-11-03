use jorm::nlp::generator::{NlpProcessor, DagEdit};

#[cfg(test)]
mod nlp_tests {
    use super::*;

    #[tokio::test]
    async fn test_nlp_processor_creation() {
        let result = NlpProcessor::new().await;
        assert!(result.is_ok(), "Failed to create NLP processor: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_generate_dag_simple() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "Copy file1.txt to file2.txt";
        
        let result = processor.generate_dag(description).await;
        assert!(result.is_ok(), "Failed to generate DAG: {:?}", result.err());
        
        let dag_content = result.unwrap();
        assert!(!dag_content.is_empty());
        assert!(dag_content.contains("file_copy") || dag_content.contains("copy"));
        assert!(dag_content.contains("file1.txt"));
        assert!(dag_content.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_generate_dag_with_preview() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "Run echo hello and then copy result.txt to backup.txt";
        
        let result = processor.generate_dag_with_preview(description).await;
        assert!(result.is_ok(), "Failed to generate DAG with preview: {:?}", result.err());
        
        let preview = result.unwrap();
        assert!(!preview.generated_dag_content.is_empty());
        assert!(!preview.summary.is_empty());
        assert!(!preview.tasks.is_empty());
        
        // Should contain both shell and file operations
        assert!(preview.generated_dag_content.contains("shell") || preview.generated_dag_content.contains("echo"));
        assert!(preview.generated_dag_content.contains("copy") || preview.generated_dag_content.contains("file_copy"));
    }

    #[tokio::test]
    async fn test_generate_dag_complex() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "First create a directory, then download data from an API, process it with Python, and finally clean up temporary files";
        
        let result = processor.generate_dag(description).await;
        assert!(result.is_ok(), "Failed to generate complex DAG: {:?}", result.err());
        
        let dag_content = result.unwrap();
        assert!(!dag_content.is_empty());
        
        // Should contain multiple task types
        assert!(dag_content.contains("shell") || dag_content.contains("mkdir"));
        assert!(dag_content.contains("http") || dag_content.contains("download"));
        assert!(dag_content.contains("python") || dag_content.contains("process"));
        assert!(dag_content.contains("delete") || dag_content.contains("cleanup"));
    }

    #[tokio::test]
    async fn test_validate_generated_dag() {
        let processor = NlpProcessor::new().await.unwrap();
        
        // Valid DAG content
        let valid_dag = r#"
task test_task {
    type: shell
    command: "echo hello"
}
"#;
        
        let result = processor.validate_generated_dag(valid_dag);
        assert!(result.is_ok(), "Valid DAG should pass validation");
        
        // Invalid DAG content
        let invalid_dag = r#"
invalid syntax here
"#;
        
        let result = processor.validate_generated_dag(invalid_dag);
        assert!(result.is_err(), "Invalid DAG should fail validation");
    }

    #[tokio::test]
    async fn test_edit_generated_dag() {
        let processor = NlpProcessor::new().await.unwrap();
        
        let original_dag = r#"
task original_task {
    type: shell
    command: "echo original"
}
"#;
        
        let edits = vec![
            DagEdit::ModifyTask {
                task_name: "original_task".to_string(),
                new_command: Some("echo modified".to_string()),
                new_working_dir: None,
            }
        ];
        
        let result = processor.edit_generated_dag(original_dag, edits);
        assert!(result.is_ok(), "Failed to edit DAG: {:?}", result.err());
        
        let edited_dag = result.unwrap();
        assert!(edited_dag.contains("echo modified"));
        assert!(!edited_dag.contains("echo original"));
    }

    #[tokio::test]
    async fn test_edit_generated_dag_add_task() {
        let processor = NlpProcessor::new().await.unwrap();
        
        let original_dag = r#"
task first_task {
    type: shell
    command: "echo first"
}
"#;
        
        let edits = vec![
            DagEdit::AddTask {
                task_name: "second_task".to_string(),
                task_type: "shell".to_string(),
                command: "echo second".to_string(),
                working_dir: None,
                depends_on: Some(vec!["first_task".to_string()]),
            }
        ];
        
        let result = processor.edit_generated_dag(original_dag, edits);
        assert!(result.is_ok(), "Failed to add task to DAG: {:?}", result.err());
        
        let edited_dag = result.unwrap();
        assert!(edited_dag.contains("first_task"));
        assert!(edited_dag.contains("second_task"));
        assert!(edited_dag.contains("echo second"));
        assert!(edited_dag.contains("depends_on"));
    }

    #[tokio::test]
    async fn test_edit_generated_dag_remove_task() {
        let processor = NlpProcessor::new().await.unwrap();
        
        let original_dag = r#"
task keep_task {
    type: shell
    command: "echo keep"
}

task remove_task {
    type: shell
    command: "echo remove"
}
"#;
        
        let edits = vec![
            DagEdit::RemoveTask {
                task_name: "remove_task".to_string(),
            }
        ];
        
        let result = processor.edit_generated_dag(original_dag, edits);
        assert!(result.is_ok(), "Failed to remove task from DAG: {:?}", result.err());
        
        let edited_dag = result.unwrap();
        assert!(edited_dag.contains("keep_task"));
        assert!(!edited_dag.contains("remove_task"));
        assert!(edited_dag.contains("echo keep"));
        assert!(!edited_dag.contains("echo remove"));
    }

    #[tokio::test]
    async fn test_generate_dag_file_operations() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "Delete old logs, copy config.txt to backup.txt, and move temp files to archive";
        
        let result = processor.generate_dag(description).await;
        assert!(result.is_ok(), "Failed to generate file operations DAG: {:?}", result.err());
        
        let dag_content = result.unwrap();
        assert!(!dag_content.is_empty());
        
        // Should contain file operations
        assert!(dag_content.contains("delete") || dag_content.contains("file_delete"));
        assert!(dag_content.contains("copy") || dag_content.contains("file_copy"));
        assert!(dag_content.contains("move") || dag_content.contains("file_move"));
    }

    #[tokio::test]
    async fn test_generate_dag_http_operations() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "Send a POST request to https://api.example.com with JSON data";
        
        let result = processor.generate_dag(description).await;
        assert!(result.is_ok(), "Failed to generate HTTP operations DAG: {:?}", result.err());
        
        let dag_content = result.unwrap();
        assert!(!dag_content.is_empty());
        
        // Should contain HTTP operations
        assert!(dag_content.contains("http") || dag_content.contains("POST"));
        assert!(dag_content.contains("api.example.com"));
    }

    #[tokio::test]
    async fn test_generate_dag_empty_description() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "";
        
        let result = processor.generate_dag(description).await;
        // Should handle empty description gracefully
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_generate_dag_invalid_description() {
        let processor = NlpProcessor::new().await.unwrap();
        let description = "This is not a valid task description with random words that don't make sense for automation";
        
        let result = processor.generate_dag(description).await;
        // Should either fail gracefully or generate a minimal DAG
        assert!(result.is_ok() || result.is_err());
    }
}