// Test fixtures and utilities
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub struct TestFixtures;

impl TestFixtures {
    /// Create a simple shell DAG for testing
    pub fn create_simple_shell_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
task hello {
    type: shell
    command: "echo 'Hello, World!'"
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with file operations
    pub fn create_file_operations_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let source_file = temp_dir.path().join("source.txt");
        let dest_file = temp_dir.path().join("dest.txt");
        
        // Create source file
        fs::write(&source_file, "test content").unwrap();
        
        let content = format!(r#"
task copy_file {{
    type: file_copy
    source: "{}"
    destination: "{}"
}}

task delete_source {{
    type: file_delete
    path: "{}"
    depends_on: [copy_file]
}}
"#, 
            source_file.to_str().unwrap(),
            dest_file.to_str().unwrap(),
            source_file.to_str().unwrap()
        );
        
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with dependencies
    pub fn create_dependency_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
task task_a {
    type: shell
    command: "echo 'task a'"
}

task task_b {
    type: shell
    command: "echo 'task b'"
    depends_on: [task_a]
}

task task_c {
    type: shell
    command: "echo 'task c'"
    depends_on: [task_a, task_b]
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with HTTP tasks
    pub fn create_http_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
task http_get {
    type: http
    method: "GET"
    url: "https://httpbin.org/get"
}

task http_post {
    type: http
    method: "POST"
    url: "https://httpbin.org/post"
    headers: {
        "Content-Type": "application/json"
    }
    body: '{"test": "data"}'
    depends_on: [http_get]
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with Python tasks
    pub fn create_python_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let python_script = temp_dir.path().join("test_script.py");
        
        // Create Python script
        let script_content = r#"#!/usr/bin/env python3
import sys
print(f"Hello from Python! Args: {sys.argv[1:]}")
"#;
        fs::write(&python_script, script_content).unwrap();
        
        let content = format!(r#"
task python_task {{
    type: python
    script: "{}"
    args: ["arg1", "arg2"]
}}
"#, python_script.to_str().unwrap());
        
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with Rust tasks
    pub fn create_rust_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
task rust_check {
    type: rust
    command: "cargo check"
    working_dir: "."
}

task rust_test {
    type: rust
    command: "cargo test"
    working_dir: "."
    depends_on: [rust_check]
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create an invalid DAG for error testing
    pub fn create_invalid_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
this is not valid DAG syntax
task without proper format {
    invalid: content
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a DAG with circular dependencies
    pub fn create_circular_dependency_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let content = r#"
task task_a {
    type: shell
    command: "echo 'task a'"
    depends_on: [task_b]
}

task task_b {
    type: shell
    command: "echo 'task b'"
    depends_on: [task_a]
}
"#;
        fs::write(&dag_file, content).unwrap();
        dag_file
    }

    /// Create a complex multi-type DAG
    pub fn create_complex_dag(temp_dir: &TempDir, filename: &str) -> std::path::PathBuf {
        let dag_file = temp_dir.path().join(filename);
        let data_file = temp_dir.path().join("data.txt");
        let result_file = temp_dir.path().join("result.txt");
        
        // Create initial data file
        fs::write(&data_file, "initial data").unwrap();
        
        let content = format!(r#"
task setup {{
    type: shell
    command: "echo 'Setting up environment'"
}}

task backup_data {{
    type: file_copy
    source: "{}"
    destination: "{}.backup"
    depends_on: [setup]
}}

task process_data {{
    type: shell
    command: "wc -l {} > {}"
    depends_on: [backup_data]
}}

task http_notify {{
    type: http
    method: "POST"
    url: "https://httpbin.org/post"
    headers: {{
        "Content-Type": "application/json"
    }}
    body: '{{"status": "completed"}}'
    depends_on: [process_data]
}}

task cleanup {{
    type: file_delete
    path: "{}.backup"
    depends_on: [http_notify]
}}
"#, 
            data_file.to_str().unwrap(),
            data_file.to_str().unwrap(),
            data_file.to_str().unwrap(),
            result_file.to_str().unwrap(),
            data_file.to_str().unwrap()
        );
        
        fs::write(&dag_file, content).unwrap();
        dag_file
    }
}