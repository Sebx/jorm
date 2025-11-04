#[cfg(test)]
mod executor_tests {
    use crate::core::{
        dag::Dag,
        task::{Task, TaskType},
    };
    use crate::executor::TaskExecutor;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_dependency_resolution_simple() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("test_dag".to_string());

        // Create tasks: task_a -> task_b -> task_c
        let task_a = Task {
            name: "task_a".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'task a'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let task_b = Task {
            name: "task_b".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'task b'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let task_c = Task {
            name: "task_c".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'task c'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        dag.add_task(task_a);
        dag.add_task(task_b);
        dag.add_task(task_c);

        // task_b depends on task_a, task_c depends on task_b
        dag.add_dependency("task_b".to_string(), vec!["task_a".to_string()]);
        dag.add_dependency("task_c".to_string(), vec!["task_b".to_string()]);

        let execution_order = executor.resolve_execution_order(&dag).unwrap();

        assert_eq!(execution_order, vec!["task_a", "task_b", "task_c"]);
    }

    #[tokio::test]
    async fn test_dependency_resolution_no_dependencies() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("test_dag".to_string());

        // Create independent tasks
        let task_a = Task {
            name: "task_a".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'task a'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let task_b = Task {
            name: "task_b".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'task b'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        dag.add_task(task_a);
        dag.add_task(task_b);

        let execution_order = executor.resolve_execution_order(&dag).unwrap();

        // Both tasks should be included, order doesn't matter for independent tasks
        assert_eq!(execution_order.len(), 2);
        assert!(execution_order.contains(&"task_a".to_string()));
        assert!(execution_order.contains(&"task_b".to_string()));
    }

    #[tokio::test]
    async fn test_dependency_resolution_parallel_branches() {
        let executor = TaskExecutor::new();
        let mut dag = Dag::new("test_dag".to_string());

        // Create diamond dependency: root -> (branch_a, branch_b) -> final
        let root = Task {
            name: "root".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'root'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let branch_a = Task {
            name: "branch_a".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'branch a'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let branch_b = Task {
            name: "branch_b".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'branch b'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        let final_task = Task {
            name: "final".to_string(),
            task_type: TaskType::Shell {
                command: "echo 'final'".to_string(),
                working_dir: None,
            },
            parameters: HashMap::new(),
        };

        dag.add_task(root);
        dag.add_task(branch_a);
        dag.add_task(branch_b);
        dag.add_task(final_task);

        dag.add_dependency("branch_a".to_string(), vec!["root".to_string()]);
        dag.add_dependency("branch_b".to_string(), vec!["root".to_string()]);
        dag.add_dependency(
            "final".to_string(),
            vec!["branch_a".to_string(), "branch_b".to_string()],
        );

        let execution_order = executor.resolve_execution_order(&dag).unwrap();

        // Root should be first, final should be last
        assert_eq!(execution_order[0], "root");
        assert_eq!(execution_order[3], "final");

        // branch_a and branch_b should be in the middle (order doesn't matter)
        let middle_tasks: std::collections::HashSet<_> = execution_order[1..3].iter().collect();
        assert!(middle_tasks.contains(&&"branch_a".to_string()));
        assert!(middle_tasks.contains(&&"branch_b".to_string()));
    }
}
