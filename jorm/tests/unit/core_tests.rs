use jorm::core::{dag::Dag, task::{Task, TaskType}, error::JormError};
use std::collections::HashMap;

#[cfg(test)]
mod dag_tests {
    use super::*;

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
    fn test_add_dependency() {
        let mut dag = Dag::new("test_dag".to_string());
        dag.add_dependency("task_b".to_string(), vec!["task_a".to_string()]);
        
        assert_eq!(dag.dependencies.len(), 1);
        assert_eq!(dag.dependencies.get("task_b").unwrap(), &vec!["task_a".to_string()]);
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
    fn test_validate_valid_dag() {
        let mut dag = Dag::new("valid_dag".to_string());
        let task = Task::new(
            "test_task".to_string(),
            TaskType::Shell {
                command: "echo hello".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task);
        let result = dag.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_circular_dependency() {
        let mut dag = Dag::new("circular_dag".to_string());
        
        let task_a = Task::new(
            "task_a".to_string(),
            TaskType::Shell {
                command: "echo a".to_string(),
                working_dir: None,
            },
        );
        
        let task_b = Task::new(
            "task_b".to_string(),
            TaskType::Shell {
                command: "echo b".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task_a);
        dag.add_task(task_b);
        dag.add_dependency("task_a".to_string(), vec!["task_b".to_string()]);
        dag.add_dependency("task_b".to_string(), vec!["task_a".to_string()]);
        
        let result = dag.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            JormError::ParseError(msg) => assert!(msg.contains("Circular dependency")),
            _ => panic!("Expected ParseError for circular dependency"),
        }
    }

    #[test]
    fn test_validate_missing_dependency() {
        let mut dag = Dag::new("missing_dep_dag".to_string());
        
        let task = Task::new(
            "task_a".to_string(),
            TaskType::Shell {
                command: "echo a".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task);
        dag.add_dependency("task_a".to_string(), vec!["nonexistent_task".to_string()]);
        
        let result = dag.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            JormError::ParseError(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected ParseError for missing dependency"),
        }
    }
}

#[cfg(test)]
mod task_tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "test_task".to_string(),
            TaskType::Shell {
                command: "echo hello".to_string(),
                working_dir: Some("/tmp".to_string()),
            },
        );
        
        assert_eq!(task.name, "test_task");
        assert!(task.parameters.is_empty());
        
        match task.task_type {
            TaskType::Shell { command, working_dir } => {
                assert_eq!(command, "echo hello");
                assert_eq!(working_dir.unwrap(), "/tmp");
            }
            _ => panic!("Expected Shell task type"),
        }
    }

    #[test]
    fn test_task_with_parameter() {
        let task = Task::new(
            "test_task".to_string(),
            TaskType::Shell {
                command: "echo hello".to_string(),
                working_dir: None,
            },
        ).with_parameter("timeout".to_string(), "30".to_string());
        
        assert_eq!(task.parameters.len(), 1);
        assert_eq!(task.parameters.get("timeout").unwrap(), "30");
    }

    #[test]
    fn test_validate_shell_task() {
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

    #[test]
    fn test_validate_http_task() {
        let valid_task = Task::new(
            "valid_http".to_string(),
            TaskType::Http {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: None,
                body: None,
            },
        );
        assert!(valid_task.validate().is_ok());
        
        let invalid_task = Task::new(
            "invalid_http".to_string(),
            TaskType::Http {
                method: "".to_string(),
                url: "https://example.com".to_string(),
                headers: None,
                body: None,
            },
        );
        assert!(invalid_task.validate().is_err());
    }

    #[test]
    fn test_validate_file_operations() {
        let valid_copy = Task::new(
            "valid_copy".to_string(),
            TaskType::FileCopy {
                source: "file1.txt".to_string(),
                destination: "file2.txt".to_string(),
            },
        );
        assert!(valid_copy.validate().is_ok());
        
        let invalid_copy = Task::new(
            "invalid_copy".to_string(),
            TaskType::FileCopy {
                source: "".to_string(),
                destination: "file2.txt".to_string(),
            },
        );
        assert!(invalid_copy.validate().is_err());
        
        let valid_delete = Task::new(
            "valid_delete".to_string(),
            TaskType::FileDelete {
                path: "file.txt".to_string(),
            },
        );
        assert!(valid_delete.validate().is_ok());
        
        let invalid_delete = Task::new(
            "invalid_delete".to_string(),
            TaskType::FileDelete {
                path: "".to_string(),
            },
        );
        assert!(invalid_delete.validate().is_err());
    }
}