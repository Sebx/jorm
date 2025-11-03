// Unit tests for core components
mod unit {
    mod core_tests {
        use jorm::core::{dag::Dag, task::{Task, TaskType}, error::JormError};


        #[test]
        fn test_dag_creation() {
            let dag = Dag::new("test_dag".to_string());
            assert_eq!(dag.name, "test_dag");
            assert!(dag.tasks.is_empty());
            assert!(dag.dependencies.is_empty());
        }

        #[test]
        fn test_add_task() {
            let mut dag = Dag::new("test_dag".to_string());
            let task = Task::new(
                "test_task".to_string(),
                TaskType::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            );
            
            dag.add_task(task);
            assert_eq!(dag.tasks.len(), 1);
            assert!(dag.tasks.contains_key("test_task"));
        }

        #[test]
        fn test_validate_empty_dag() {
            let dag = Dag::new("empty_dag".to_string());
            let result = dag.validate();
            
            assert!(result.is_err());
            match result.unwrap_err() {
                JormError::ParseError(msg) => assert!(msg.contains("at least one task")),
                _ => panic!("Expected ParseError"),
            }
        }

        #[test]
        fn test_task_validation() {
            let valid_task = Task::new(
                "valid_shell".to_string(),
                TaskType::Shell {
                    command: "echo hello".to_string(),
                    working_dir: None,
                },
            );
            assert!(valid_task.validate().is_ok());
            
            let invalid_task = Task::new(
                "invalid_shell".to_string(),
                TaskType::Shell {
                    command: "".to_string(),
                    working_dir: None,
                },
            );
            assert!(invalid_task.validate().is_err());
        }
    }

    mod parser_tests {
        use jorm::parser::dag_parser::DagParser;
        use jorm::core::task::TaskType;

        #[tokio::test]
        async fn test_parse_shell_task() {
            let content = r#"
task simple_shell {
    type: shell
    command: "echo hello world"
    working_dir: "/tmp"
}
"#;

            let parser = DagParser::new();
            let result = parser.parse_content(content);
            
            assert!(result.is_ok());
            let dag = result.unwrap();
            assert_eq!(dag.tasks.len(), 1);
            
            let task = dag.tasks.get("simple_shell").unwrap();
            match &task.task_type {
                TaskType::Shell { command, working_dir } => {
                    assert_eq!(command, "echo hello world");
                    assert_eq!(working_dir.as_ref().unwrap(), "/tmp");
                }
                _ => panic!("Expected shell task type"),
            }
        }

        #[tokio::test]
        async fn test_parse_file_operations() {
            let content = r#"
task copy_file {
    type: file_copy
    source: "source.txt"
    destination: "dest.txt"
}
"#;

            let parser = DagParser::new();
            let result = parser.parse_content(content);
            
            assert!(result.is_ok());
            let dag = result.unwrap();
            assert_eq!(dag.tasks.len(), 1);
            
            let copy_task = dag.tasks.get("copy_file").unwrap();
            match &copy_task.task_type {
                TaskType::FileCopy { source, destination } => {
                    assert_eq!(source, "source.txt");
                    assert_eq!(destination, "dest.txt");
                }
                _ => panic!("Expected FileCopy task type"),
            }
        }
    }

    mod executor_tests {
        use jorm::executor::TaskExecutor;
        use jorm::core::{dag::Dag, task::{Task, TaskType}};

        #[tokio::test]
        async fn test_executor_creation() {
            let executor = TaskExecutor::new();
            // Test that executor can be created
            assert!(true);
        }

        #[tokio::test]
        async fn test_execute_simple_dag() {
            let executor = TaskExecutor::new();
            let mut dag = Dag::new("simple_dag".to_string());
            
            let task = Task::new(
                "echo_task".to_string(),
                TaskType::Shell {
                    command: "cmd /c echo hello world".to_string(),
                    working_dir: None,
                },
            );
            
            dag.add_task(task);
            
            let result = executor.execute_dag(&dag).await;
            assert!(result.is_ok(), "Simple DAG execution should succeed");
            
            let execution_result = result.unwrap();
            assert_eq!(execution_result.task_results.len(), 1);
            assert_eq!(execution_result.task_results[0].task_name, "echo_task");
        }
    }

    mod nlp_tests {
        use jorm::nlp::generator::NlpProcessor;

        #[tokio::test]
        async fn test_nlp_processor_creation() {
            let result = NlpProcessor::new().await;
            assert!(result.is_ok(), "Failed to create NLP processor: {:?}", result.err());
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
        }
    }
}