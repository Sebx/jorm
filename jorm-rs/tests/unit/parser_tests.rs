//! Parser Unit Tests
//! 
//! This module contains unit tests for the parser functionality.

use jorm_rs::parser::{parse_dag_file, validate_dag, Dag, Task, TaskConfig, Dependency};
use tempfile::TempDir;
use std::fs;
use std::collections::HashMap;

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_txt_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("simple.txt");
        
        let content = r#"dag: simple
schedule: every 10 minutes

tasks:
- extract_sales
- transform_data
- load_data

dependencies:
- transform_data after extract_sales
- load_data after transform_data"#;
        
        fs::write(&file_path, content).unwrap();
        
        let dag = parse_dag_file(file_path.to_str().unwrap()).await.unwrap();
        assert_eq!(dag.name, "simple");
        assert_eq!(dag.schedule, Some("every 10 minutes".to_string()));
        assert_eq!(dag.tasks.len(), 3);
        assert!(dag.tasks.contains_key("extract_sales"));
        assert!(dag.tasks.contains_key("transform_data"));
        assert!(dag.tasks.contains_key("load_data"));
        assert_eq!(dag.dependencies.len(), 2);
    }

    #[tokio::test]
    async fn test_parse_yaml_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("simple.yaml");
        
        let content = r#"name: simple_yaml
schedule: "0 0 * * *"

tasks:
  extract_sales:
    type: shell
    command: "echo 'extracting sales data'"
  transform_data:
    type: python
    script: "print('transforming data')"
  load_data:
    type: shell
    command: "echo 'loading data'"

dependencies:
  - task: transform_data
    depends_on: extract_sales
  - task: load_data
    depends_on: transform_data"#;
        
        fs::write(&file_path, content).unwrap();
        
        let dag = parse_dag_file(file_path.to_str().unwrap()).await.unwrap();
        assert_eq!(dag.name, "simple_yaml");
        assert_eq!(dag.schedule, Some("0 0 * * *".to_string()));
        assert_eq!(dag.tasks.len(), 3);
        assert!(dag.tasks.contains_key("extract_sales"));
        assert!(dag.tasks.contains_key("transform_data"));
        assert!(dag.tasks.contains_key("load_data"));
        assert_eq!(dag.dependencies.len(), 2);
    }

    #[tokio::test]
    async fn test_parse_complex_dag() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("complex.txt");
        
        let content = r#"dag: complex_workflow
schedule: every 1 hour

tasks:
- task_a
- task_b
- task_c
- task_d

dependencies:
- task_b after task_a
- task_c after task_a
- task_d after task_b
- task_d after task_c"#;
        
        fs::write(&file_path, content).unwrap();
        
        let dag = parse_dag_file(file_path.to_str().unwrap()).await.unwrap();
        assert_eq!(dag.name, "complex_workflow");
        assert_eq!(dag.schedule, Some("every 1 hour".to_string()));
        assert_eq!(dag.tasks.len(), 4);
        assert_eq!(dag.dependencies.len(), 4);
    }

    #[test]
    fn test_dag_validation_success() {
        let dag = Dag {
            name: "test_dag".to_string(),
            schedule: None,
            tasks: {
                let mut tasks = HashMap::new();
                tasks.insert("task1".to_string(), Task {
                    name: "task1".to_string(),
                    description: Some("Test task 1".to_string()),
                    config: TaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some("echo 'Hello'".to_string()),
                        ..Default::default()
                    },
                });
                tasks
            },
            dependencies: vec![],
        };
        
        let errors = validate_dag(&dag).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_dag_validation_with_dependencies() {
        let dag = Dag {
            name: "test_dag_with_deps".to_string(),
            schedule: None,
            tasks: {
                let mut tasks = HashMap::new();
                tasks.insert("task_a".to_string(), Task {
                    name: "task_a".to_string(),
                    description: Some("Task A".to_string()),
                    config: TaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some("echo 'A'".to_string()),
                        ..Default::default()
                    },
                });
                tasks.insert("task_b".to_string(), Task {
                    name: "task_b".to_string(),
                    description: Some("Task B".to_string()),
                    config: TaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some("echo 'B'".to_string()),
                        ..Default::default()
                    },
                });
                tasks
            },
            dependencies: vec![
                Dependency {
                    task: "task_b".to_string(),
                    depends_on: "task_a".to_string(),
                },
            ],
        };
        
        let errors = validate_dag(&dag).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_dag_validation_missing_task() {
        let dag = Dag {
            name: "invalid_dag".to_string(),
            schedule: None,
            tasks: {
                let mut tasks = HashMap::new();
                tasks.insert("task1".to_string(), Task {
                    name: "task1".to_string(),
                    description: Some("Task 1".to_string()),
                    config: TaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some("echo 'Hello'".to_string()),
                        ..Default::default()
                    },
                });
                tasks
            },
            dependencies: vec![
                Dependency {
                    task: "task1".to_string(),
                    depends_on: "nonexistent_task".to_string(),
                },
            ],
        };
        
        let errors = validate_dag(&dag).unwrap();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_dag_validation_empty_name() {
        let dag = Dag {
            name: "".to_string(),
            schedule: None,
            tasks: HashMap::new(),
            dependencies: vec![],
        };
        
        let errors = validate_dag(&dag).unwrap();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_task_config_creation() {
        let shell_config = TaskConfig {
            task_type: Some("shell".to_string()),
            command: Some("echo 'hello'".to_string()),
            ..Default::default()
        };
        
        let python_config = TaskConfig {
            task_type: Some("python".to_string()),
            script: Some("print('hello')".to_string()),
            ..Default::default()
        };
        
        let http_config = TaskConfig {
            task_type: Some("http".to_string()),
            method: Some("GET".to_string()),
            url: Some("https://api.example.com".to_string()),
            ..Default::default()
        };
        
        let file_config = TaskConfig {
            task_type: Some("file".to_string()),
            script: Some("Hello, World!".to_string()),
            destination: Some("output.txt".to_string()),
            ..Default::default()
        };
        
        // Test that configs are created successfully
        assert!(matches!(shell_config.task_type, Some(ref t) if t == "shell"));
        assert!(matches!(python_config.task_type, Some(ref t) if t == "python"));
        assert!(matches!(http_config.task_type, Some(ref t) if t == "http"));
        assert!(matches!(file_config.task_type, Some(ref t) if t == "file"));
    }

    #[tokio::test]
    async fn test_parse_nonexistent_file() {
        let result = parse_dag_file("/nonexistent/file.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.txt");
        
        let content = "this is not a valid DAG file";
        fs::write(&file_path, content).unwrap();
        
        let result = parse_dag_file(file_path.to_str().unwrap()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency {
            task: "task2".to_string(),
            depends_on: "task1".to_string(),
        };
        
        assert_eq!(dep.task, "task2");
        assert_eq!(dep.depends_on, "task1");
    }

    #[test]
    fn test_dag_creation() {
        let dag = Dag {
            name: "test_dag".to_string(),
            schedule: Some("every 1 minute".to_string()),
            tasks: {
                let mut tasks = HashMap::new();
                tasks.insert("task1".to_string(), Task {
                    name: "task1".to_string(),
                    description: Some("Task 1".to_string()),
                    config: TaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some("echo 'Hello'".to_string()),
                        ..Default::default()
                    },
                });
                tasks
            },
            dependencies: vec![],
        };
        
        assert_eq!(dag.name, "test_dag");
        assert_eq!(dag.schedule, Some("every 1 minute".to_string()));
        assert_eq!(dag.tasks.len(), 1);
        assert_eq!(dag.dependencies.len(), 0);
    }
}