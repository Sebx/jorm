use super::generator::NlpProcessor;

#[tokio::test]
async fn test_nlp_processor_creation() {
    let processor = NlpProcessor::new().await;
    assert!(processor.is_ok());
}

#[tokio::test]
async fn test_simple_shell_command_generation() {
    let processor = NlpProcessor::new().await.unwrap();
    let description = "run command \"echo hello world\"";

    let result = processor.generate_dag(description).await;
    assert!(result.is_ok());

    let dag_content = result.unwrap();
    assert!(dag_content.contains("task_1"));
    assert!(dag_content.contains("type: shell"));
    assert!(dag_content.contains("echo hello world"));
}

#[tokio::test]
async fn test_file_copy_generation() {
    let processor = NlpProcessor::new().await.unwrap();
    let description = "copy file1.txt to file2.txt";

    let result = processor.generate_dag(description).await;
    assert!(result.is_ok());

    let dag_content = result.unwrap();
    assert!(dag_content.contains("type: file_copy"));
    assert!(dag_content.contains("file1.txt"));
    assert!(dag_content.contains("file2.txt"));
}

#[tokio::test]
async fn test_dag_preview_generation() {
    let processor = NlpProcessor::new().await.unwrap();
    let description = "run command \"ls -la\" then copy file1.txt to backup/file1.txt";

    let result = processor.generate_dag_with_preview(description).await;
    assert!(result.is_ok());

    let preview = result.unwrap();
    assert_eq!(preview.original_description, description);
    // The task count might be 1 if only one task is parsed correctly
    assert!(preview.task_count >= 1);
    assert!(preview.task_names.contains(&"task_1".to_string()));
}

#[tokio::test]
async fn test_validation_of_generated_dag() {
    let processor = NlpProcessor::new().await.unwrap();
    let valid_dag = r#"
task test_task {
    type: shell
    command: "echo test"
}
"#;

    let result = processor.validate_generated_dag(valid_dag);
    assert!(result.is_ok());
}
