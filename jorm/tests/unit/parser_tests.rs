use jorm::parser::dag_parser::DagParser;
use jorm::core::task::TaskType;

#[cfg(test)]
mod parser_tests {
    use super::*;

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
    async fn test_parse_http_task() {
        let content = r#"
task http_request {
    type: http
    method: "POST"
    url: "https://api.example.com/webhook"
    headers: {
        "Content-Type": "application/json",
        "Authorization": "Bearer token"
    }
    body: '{"message": "hello"}'
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        
        let task = dag.tasks.get("http_request").unwrap();
        match &task.task_type {
            TaskType::Http { method, url, headers, body } => {
                assert_eq!(method, "POST");
                assert_eq!(url, "https://api.example.com/webhook");
                assert!(headers.is_some());
                assert!(body.is_some());
                
                let headers = headers.as_ref().unwrap();
                assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
                assert_eq!(headers.get("Authorization").unwrap(), "Bearer token");
                assert_eq!(body.as_ref().unwrap(), r#"{"message": "hello"}"#);
            }
            _ => panic!("Expected HTTP task type"),
        }
    }

    #[tokio::test]
    async fn test_parse_python_task() {
        let content = r#"
task python_script {
    type: python
    script: "process_data.py"
    args: ["--input", "data.csv", "--output", "result.json"]
    working_dir: "./scripts"
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        
        let task = dag.tasks.get("python_script").unwrap();
        match &task.task_type {
            TaskType::Python { script, args, working_dir } => {
                assert_eq!(script, "process_data.py");
                assert!(args.is_some());
                assert_eq!(working_dir.as_ref().unwrap(), "./scripts");
                
                let args = args.as_ref().unwrap();
                assert_eq!(args.len(), 4);
                assert_eq!(args[0], "--input");
                assert_eq!(args[1], "data.csv");
            }
            _ => panic!("Expected Python task type"),
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

task move_file {
    type: file_move
    source: "temp.txt"
    destination: "final.txt"
}

task delete_file {
    type: file_delete
    path: "unwanted.txt"
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 3);
        
        // Test file copy
        let copy_task = dag.tasks.get("copy_file").unwrap();
        match &copy_task.task_type {
            TaskType::FileCopy { source, destination } => {
                assert_eq!(source, "source.txt");
                assert_eq!(destination, "dest.txt");
            }
            _ => panic!("Expected FileCopy task type"),
        }
        
        // Test file move
        let move_task = dag.tasks.get("move_file").unwrap();
        match &move_task.task_type {
            TaskType::FileMove { source, destination } => {
                assert_eq!(source, "temp.txt");
                assert_eq!(destination, "final.txt");
            }
            _ => panic!("Expected FileMove task type"),
        }
        
        // Test file delete
        let delete_task = dag.tasks.get("delete_file").unwrap();
        match &delete_task.task_type {
            TaskType::FileDelete { path } => {
                assert_eq!(path, "unwanted.txt");
            }
            _ => panic!("Expected FileDelete task type"),
        }
    }

    #[tokio::test]
    async fn test_parse_dependencies() {
        let content = r#"
task task_a {
    type: shell
    command: "echo a"
}

task task_b {
    type: shell
    command: "echo b"
    depends_on: [task_a]
}

task task_c {
    type: shell
    command: "echo c"
    depends_on: [task_a, task_b]
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 3);
        
        // Check dependencies
        assert!(dag.dependencies.contains_key("task_b"));
        assert!(dag.dependencies.contains_key("task_c"));
        
        let task_b_deps = dag.dependencies.get("task_b").unwrap();
        assert_eq!(task_b_deps, &vec!["task_a".to_string()]);
        
        let task_c_deps = dag.dependencies.get("task_c").unwrap();
        assert_eq!(task_c_deps.len(), 2);
        assert!(task_c_deps.contains(&"task_a".to_string()));
        assert!(task_c_deps.contains(&"task_b".to_string()));
    }

    #[tokio::test]
    async fn test_parse_comments_and_whitespace() {
        let content = r#"
# This is a comment
# Another comment

task test_task {
    # Comment inside task
    type: shell
    command: "echo test"
    # More comments
}

# Final comment
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 1);
        assert!(dag.tasks.contains_key("test_task"));
    }

    #[tokio::test]
    async fn test_parse_syntax_errors() {
        // Missing task type
        let content1 = r#"
task invalid_task {
    command: "echo test"
}
"#;
        let parser = DagParser::new();
        let result1 = parser.parse_content(content1);
        assert!(result1.is_err());

        // Invalid task type
        let content2 = r#"
task invalid_task {
    type: invalid_type
    command: "echo test"
}
"#;
        let result2 = parser.parse_content(content2);
        assert!(result2.is_err());

        // Missing required parameter
        let content3 = r#"
task invalid_task {
    type: shell
    # Missing command
}
"#;
        let result3 = parser.parse_content(content3);
        assert!(result3.is_err());
    }

    #[tokio::test]
    async fn test_parse_complex_dag() {
        let content = r#"
# Complex DAG with multiple task types and dependencies
task setup {
    type: shell
    command: "mkdir -p /tmp/test"
}

task download_data {
    type: http
    method: "GET"
    url: "https://api.example.com/data"
    depends_on: [setup]
}

task process_data {
    type: python
    script: "process.py"
    args: ["--input", "/tmp/data.json"]
    working_dir: "/tmp/test"
    depends_on: [download_data]
}

task build_project {
    type: rust
    command: "cargo build --release"
    working_dir: "./project"
    depends_on: [process_data]
}

task cleanup {
    type: file_delete
    path: "/tmp/test"
    depends_on: [build_project]
}
"#;

        let parser = DagParser::new();
        let result = parser.parse_content(content);
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 5);
        
        // Validate the DAG structure
        assert!(dag.validate().is_ok());
        
        // Check that all dependencies are properly set
        assert_eq!(dag.dependencies.len(), 4);
        assert!(dag.dependencies.get("download_data").unwrap().contains(&"setup".to_string()));
        assert!(dag.dependencies.get("process_data").unwrap().contains(&"download_data".to_string()));
        assert!(dag.dependencies.get("build_project").unwrap().contains(&"process_data".to_string()));
        assert!(dag.dependencies.get("cleanup").unwrap().contains(&"build_project".to_string()));
    }
}