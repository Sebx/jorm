#[cfg(test)]
mod tests {
    use crate::parser::dag_parser::DagParser;
    use crate::core::task::TaskType;

    #[tokio::test]
    async fn test_parse_simple_dag() {
        let content = r#"
# Simple test DAG
task hello {
    type: shell
    command: "echo hello world"
    working_dir: "/tmp"
}

task copy_file {
    type: file_copy
    source: "file1.txt"
    destination: "file2.txt"
    depends_on: [hello]
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok(), "Failed to parse DAG: {:?}", result.err());
        
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 2);
        assert!(dag.tasks.contains_key("hello"));
        assert!(dag.tasks.contains_key("copy_file"));
        
        // Check task types
        if let Some(hello_task) = dag.tasks.get("hello") {
            match &hello_task.task_type {
                TaskType::Shell { command, working_dir } => {
                    assert_eq!(command, "echo hello world");
                    assert_eq!(working_dir.as_ref().unwrap(), "/tmp");
                }
                _ => panic!("Expected shell task type"),
            }
        }
        
        // Check dependencies
        assert!(dag.dependencies.contains_key("copy_file"));
        let deps = dag.dependencies.get("copy_file").unwrap();
        assert_eq!(deps, &vec!["hello".to_string()]);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let content = r#"
task task_a {
    type: shell
    command: "echo a"
    depends_on: [task_b]
}

task task_b {
    type: shell
    command: "echo b"
    depends_on: [task_a]
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_err(), "Expected circular dependency error");
        let error = result.err().unwrap();
        assert!(error.to_string().contains("Circular dependency"));
    }

    #[tokio::test]
    async fn test_missing_dependency() {
        let content = r#"
task task_a {
    type: shell
    command: "echo a"
    depends_on: [nonexistent_task]
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_err(), "Expected missing dependency error");
        let error = result.err().unwrap();
        assert!(error.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_invalid_task_type() {
        let content = r#"
task task_a {
    type: invalid_type
    command: "echo a"
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_err(), "Expected invalid task type error");
        let error = result.err().unwrap();
        assert!(error.to_string().contains("Invalid task type"));
    }

    #[tokio::test]
    async fn test_missing_required_parameter() {
        let content = r#"
task task_a {
    type: shell
    # Missing command parameter
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_err(), "Expected missing parameter error");
        let error = result.err().unwrap();
        assert!(error.to_string().contains("missing command"));
    }
}