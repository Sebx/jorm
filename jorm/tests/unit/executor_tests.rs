use jorm::executor::TaskExecutor;
use jorm::core::{dag::Dag, task::{Task, TaskType}};
use std::collections::HashMap;

#[cfg(test)]
mod executor_tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_execution_order_linear() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("linear_dag".to_string());
        
        // Create linear dependency chain: A -> B -> C
        let task_a = Task::new(
            "task_a".to_string(),
            TaskType::Shell {
                command: "echo 'task a'".to_string(),
                working_dir: None,
            },
        );
        
        let task_b = Task::new(
            "task_b".to_string(),
            TaskType::Shell {
                command: "echo 'task b'".to_string(),
                working_dir: None,
            },
        );
        
        let task_c = Task::new(
            "task_c".to_string(),
            TaskType::Shell {
                command: "echo 'task c'".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task_a);
        dag.add_task(task_b);
        dag.add_task(task_c);
        
        dag.add_dependency("task_b".to_string(), vec!["task_a".to_string()]);
        dag.add_dependency("task_c".to_string(), vec!["task_b".to_string()]);
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 3);
        assert_eq!(execution_order[0], "task_a");
        assert_eq!(execution_order[1], "task_b");
        assert_eq!(execution_order[2], "task_c");
    }

    #[tokio::test]
    async fn test_resolve_execution_order_diamond() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("diamond_dag".to_string());
        
        // Create diamond dependency: root -> (left, right) -> final
        let root = Task::new(
            "root".to_string(),
            TaskType::Shell {
                command: "echo 'root'".to_string(),
                working_dir: None,
            },
        );
        
        let left = Task::new(
            "left".to_string(),
            TaskType::Shell {
                command: "echo 'left'".to_string(),
                working_dir: None,
            },
        );
        
        let right = Task::new(
            "right".to_string(),
            TaskType::Shell {
                command: "echo 'right'".to_string(),
                working_dir: None,
            },
        );
        
        let final_task = Task::new(
            "final".to_string(),
            TaskType::Shell {
                command: "echo 'final'".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(root);
        dag.add_task(left);
        dag.add_task(right);
        dag.add_task(final_task);
        
        dag.add_dependency("left".to_string(), vec!["root".to_string()]);
        dag.add_dependency("right".to_string(), vec!["root".to_string()]);
        dag.add_dependency("final".to_string(), vec!["left".to_string(), "right".to_string()]);
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 4);
        assert_eq!(execution_order[0], "root");
        assert_eq!(execution_order[3], "final");
        
        // left and right can be in any order in the middle
        let middle_tasks: std::collections::HashSet<_> = execution_order[1..3].iter().collect();
        assert!(middle_tasks.contains(&&"left".to_string()));
        assert!(middle_tasks.contains(&&"right".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_execution_order_independent() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("independent_dag".to_string());
        
        // Create independent tasks
        let task_a = Task::new(
            "task_a".to_string(),
            TaskType::Shell {
                command: "echo 'task a'".to_string(),
                working_dir: None,
            },
        );
        
        let task_b = Task::new(
            "task_b".to_string(),
            TaskType::Shell {
                command: "echo 'task b'".to_string(),
                working_dir: None,
            },
        );
        
        let task_c = Task::new(
            "task_c".to_string(),
            TaskType::Shell {
                command: "echo 'task c'".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task_a);
        dag.add_task(task_b);
        dag.add_task(task_c);
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 3);
        // All tasks should be included, order doesn't matter for independent tasks
        let task_set: std::collections::HashSet<_> = execution_order.iter().collect();
        assert!(task_set.contains(&&"task_a".to_string()));
        assert!(task_set.contains(&&"task_b".to_string()));
        assert!(task_set.contains(&&"task_c".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_execution_order_complex() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("complex_dag".to_string());
        
        // Create complex dependency graph
        let tasks = vec![
            ("setup", vec![]),
            ("download_a", vec!["setup"]),
            ("download_b", vec!["setup"]),
            ("process_a", vec!["download_a"]),
            ("process_b", vec!["download_b"]),
            ("merge", vec!["process_a", "process_b"]),
            ("cleanup", vec!["merge"]),
        ];
        
        for (name, _) in &tasks {
            let task = Task::new(
                name.to_string(),
                TaskType::Shell {
                    command: format!("echo '{}'", name),
                    working_dir: None,
                },
            );
            dag.add_task(task);
        }
        
        for (name, deps) in &tasks {
            if !deps.is_empty() {
                dag.add_dependency(
                    name.to_string(),
                    deps.iter().map(|s| s.to_string()).collect(),
                );
            }
        }
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 7);
        
        // Verify ordering constraints
        let get_index = |task: &str| execution_order.iter().position(|t| t == task).unwrap();
        
        assert!(get_index("setup") < get_index("download_a"));
        assert!(get_index("setup") < get_index("download_b"));
        assert!(get_index("download_a") < get_index("process_a"));
        assert!(get_index("download_b") < get_index("process_b"));
        assert!(get_index("process_a") < get_index("merge"));
        assert!(get_index("process_b") < get_index("merge"));
        assert!(get_index("merge") < get_index("cleanup"));
    }

    #[tokio::test]
    async fn test_resolve_execution_order_empty_dag() {
        let executor = TaskExecutor::new();
        let dag = Dag::new("empty_dag".to_string());
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        assert!(execution_order.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_execution_order_single_task() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("single_task_dag".to_string());
        
        let task = Task::new(
            "only_task".to_string(),
            TaskType::Shell {
                command: "echo 'only task'".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task);
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 1);
        assert_eq!(execution_order[0], "only_task");
    }

    #[tokio::test]
    async fn test_task_executor_creation() {
        let executor = TaskExecutor::new();
        // Just verify that the executor can be created
        // More detailed testing would require actual task execution
        assert!(true); // Placeholder assertion
    }

    #[tokio::test]
    async fn test_multiple_dependency_resolution() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("multi_dep_dag".to_string());
        
        // Create tasks where one task depends on multiple others
        let task_a = Task::new(
            "task_a".to_string(),
            TaskType::Shell {
                command: "echo 'task a'".to_string(),
                working_dir: None,
            },
        );
        
        let task_b = Task::new(
            "task_b".to_string(),
            TaskType::Shell {
                command: "echo 'task b'".to_string(),
                working_dir: None,
            },
        );
        
        let task_c = Task::new(
            "task_c".to_string(),
            TaskType::Shell {
                command: "echo 'task c'".to_string(),
                working_dir: None,
            },
        );
        
        let task_d = Task::new(
            "task_d".to_string(),
            TaskType::Shell {
                command: "echo 'task d'".to_string(),
                working_dir: None,
            },
        );
        
        dag.add_task(task_a);
        dag.add_task(task_b);
        dag.add_task(task_c);
        dag.add_task(task_d);
        
        // task_d depends on all others
        dag.add_dependency("task_d".to_string(), vec![
            "task_a".to_string(),
            "task_b".to_string(),
            "task_c".to_string(),
        ]);
        
        let execution_order = executor.resolve_execution_order(&dag).unwrap();
        
        assert_eq!(execution_order.len(), 4);
        assert_eq!(execution_order[3], "task_d"); // task_d should be last
        
        // First three can be in any order
        let first_three: std::collections::HashSet<_> = execution_order[0..3].iter().collect();
        assert!(first_three.contains(&&"task_a".to_string()));
        assert!(first_three.contains(&&"task_b".to_string()));
        assert!(first_three.contains(&&"task_c".to_string()));
    }
}