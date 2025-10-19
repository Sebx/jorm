//! Test fixtures for various DAG formats and configurations
//!
//! This module provides reusable test fixtures for different DAG formats,
//! complex dependency scenarios, and error conditions.

use jorm::parser::{Dag, Dependency, Task as DagTask, TaskConfig as DagTaskConfig};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture manager for creating various DAG configurations
pub struct DagFixtures {
    temp_dir: TempDir,
}

impl DagFixtures {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    pub fn temp_path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create a simple linear DAG (task1 -> task2 -> task3)
    pub fn create_linear_dag(&self) -> Dag {
        let mut tasks = HashMap::new();

        tasks.insert(
            "task1".to_string(),
            DagTask {
                name: "task1".to_string(),
                description: Some("First task in linear chain".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Task 1 executed'".to_string()),
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "task2".to_string(),
            DagTask {
                name: "task2".to_string(),
                description: Some("Second task in linear chain".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Task 2 executed'".to_string()),
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "task3".to_string(),
            DagTask {
                name: "task3".to_string(),
                description: Some("Third task in linear chain".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Task 3 executed'".to_string()),
                    ..Default::default()
                },
            },
        );

        Dag {
            name: "linear_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![
                Dependency {
                    task: "task2".to_string(),
                    depends_on: "task1".to_string(),
                },
                Dependency {
                    task: "task3".to_string(),
                    depends_on: "task2".to_string(),
                },
            ],
        }
    }

    /// Create a diamond-shaped DAG (A -> B,C -> D)
    pub fn create_diamond_dag(&self) -> Dag {
        let mut tasks = HashMap::new();

        tasks.insert(
            "start".to_string(),
            DagTask {
                name: "start".to_string(),
                description: Some("Starting task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Start task'".to_string()),
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "branch_a".to_string(),
            DagTask {
                name: "branch_a".to_string(),
                description: Some("Branch A task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Branch A'".to_string()),
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "branch_b".to_string(),
            DagTask {
                name: "branch_b".to_string(),
                description: Some("Branch B task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Branch B'".to_string()),
                    ..Default::default()
                },
            },
        );

        tasks.insert(
            "end".to_string(),
            DagTask {
                name: "end".to_string(),
                description: Some("End task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'End task'".to_string()),
                    ..Default::default()
                },
            },
        );

        Dag {
            name: "diamond_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![
                Dependency {
                    task: "branch_a".to_string(),
                    depends_on: "start".to_string(),
                },
                Dependency {
                    task: "branch_b".to_string(),
                    depends_on: "start".to_string(),
                },
                Dependency {
                    task: "end".to_string(),
                    depends_on: "branch_a".to_string(),
                },
                Dependency {
                    task: "end".to_string(),
                    depends_on: "branch_b".to_string(),
                },
            ],
        }
    }

    /// Create a DAG with all supported task types
    pub fn create_multi_type_dag(&self) -> Dag {
        let mut tasks = HashMap::new();

        // Shell task
        tasks.insert(
            "shell_task".to_string(),
            DagTask {
                name: "shell_task".to_string(),
                description: Some("Shell command task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'Shell task output' > {}",
                        self.temp_path().join("shell_output.txt").display()
                    )),
                    ..Default::default()
                },
            },
        );

        // Python script task
        tasks.insert(
            "python_task".to_string(),
            DagTask {
                name: "python_task".to_string(),
                description: Some("Python script task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("python".to_string()),
                    script: Some(format!(
                        r#"
import os
print("Python task executing")
with open(r'{}', 'w') as f:
    f.write('Python task output')
print("Python task completed")
"#,
                        self.temp_path().join("python_output.txt").display()
                    )),
                    ..Default::default()
                },
            },
        );

        // File operation task
        tasks.insert(
            "file_task".to_string(),
            DagTask {
                name: "file_task".to_string(),
                description: Some("File operation task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("file".to_string()),
                    source: Some(self.temp_path().join("shell_output.txt").display().to_string()),
                    destination: Some(self.temp_path().join("copied_file.txt").display().to_string()),
                    operation: Some("copy".to_string()),
                    ..Default::default()
                },
            },
        );

        // HTTP task (may fail if no network)
        tasks.insert(
            "http_task".to_string(),
            DagTask {
                name: "http_task".to_string(),
                description: Some("HTTP request task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("http".to_string()),
                    method: Some("GET".to_string()),
                    url: Some("https://httpbin.org/get".to_string()),
                    ..Default::default()
                },
            },
        );

        Dag {
            name: "multi_type_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![
                Dependency {
                    task: "python_task".to_string(),
                    depends_on: "shell_task".to_string(),
                },
                Dependency {
                    task: "file_task".to_string(),
                    depends_on: "shell_task".to_string(),
                },
                // HTTP task runs independently
            ],
        }
    } 
   /// Create a DAG with error scenarios
    pub fn create_error_scenario_dag(&self) -> Dag {
        let mut tasks = HashMap::new();

        // Successful task
        tasks.insert(
            "success_task".to_string(),
            DagTask {
                name: "success_task".to_string(),
                description: Some("Task that succeeds".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Success'".to_string()),
                    ..Default::default()
                },
            },
        );

        // Task that fails with non-zero exit code
        tasks.insert(
            "exit_code_failure".to_string(),
            DagTask {
                name: "exit_code_failure".to_string(),
                description: Some("Task that fails with exit code".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("exit 42".to_string()),
                    ..Default::default()
                },
            },
        );

        // Task with invalid command
        tasks.insert(
            "invalid_command".to_string(),
            DagTask {
                name: "invalid_command".to_string(),
                description: Some("Task with invalid command".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("nonexistent_command_xyz".to_string()),
                    ..Default::default()
                },
            },
        );

        // Task with unsupported type
        tasks.insert(
            "unsupported_type".to_string(),
            DagTask {
                name: "unsupported_type".to_string(),
                description: Some("Task with unsupported type".to_string()),
                config: DagTaskConfig {
                    task_type: Some("unsupported_task_type".to_string()),
                    command: Some("echo 'test'".to_string()),
                    ..Default::default()
                },
            },
        );

        // Python task with syntax error
        tasks.insert(
            "python_syntax_error".to_string(),
            DagTask {
                name: "python_syntax_error".to_string(),
                description: Some("Python task with syntax error".to_string()),
                config: DagTaskConfig {
                    task_type: Some("python".to_string()),
                    script: Some("print('Hello' # Missing closing parenthesis".to_string()),
                    ..Default::default()
                },
            },
        );

        // File task with missing source
        tasks.insert(
            "file_missing_source".to_string(),
            DagTask {
                name: "file_missing_source".to_string(),
                description: Some("File task with missing source".to_string()),
                config: DagTaskConfig {
                    task_type: Some("file".to_string()),
                    source: Some("/nonexistent/path/file.txt".to_string()),
                    destination: Some(self.temp_path().join("dest.txt").display().to_string()),
                    operation: Some("copy".to_string()),
                    ..Default::default()
                },
            },
        );

        Dag {
            name: "error_scenario_dag".to_string(),
            schedule: None,
            tasks,
            dependencies: vec![],
        }
    }

    /// Create a large DAG for performance testing
    pub fn create_large_dag(&self, task_count: usize) -> Dag {
        let mut tasks = HashMap::new();

        for i in 1..=task_count {
            tasks.insert(
                format!("task_{}", i),
                DagTask {
                    name: format!("task_{}", i),
                    description: Some(format!("Large DAG task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Task {} of {}'", i, task_count)),
                        ..Default::default()
                    },
                },
            );
        }

        Dag {
            name: format!("large_dag_{}_tasks", task_count),
            schedule: None,
            tasks,
            dependencies: vec![], // Independent tasks for maximum parallelism
        }
    }

    /// Create a DAG with complex dependencies (fan-out and fan-in)
    pub fn create_complex_dependency_dag(&self) -> Dag {
        let mut tasks = HashMap::new();
        let mut dependencies = Vec::new();

        // Root task
        tasks.insert(
            "root".to_string(),
            DagTask {
                name: "root".to_string(),
                description: Some("Root task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Root task'".to_string()),
                    ..Default::default()
                },
            },
        );

        // Fan-out: multiple tasks depend on root
        for i in 1..=3 {
            let task_name = format!("fanout_{}", i);
            tasks.insert(
                task_name.clone(),
                DagTask {
                    name: task_name.clone(),
                    description: Some(format!("Fan-out task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Fan-out task {}'", i)),
                        ..Default::default()
                    },
                },
            );
            dependencies.push(Dependency {
                task: task_name,
                depends_on: "root".to_string(),
            });
        }

        // Middle layer: tasks that depend on fan-out tasks
        for i in 1..=2 {
            let task_name = format!("middle_{}", i);
            tasks.insert(
                task_name.clone(),
                DagTask {
                    name: task_name.clone(),
                    description: Some(format!("Middle task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Middle task {}'", i)),
                        ..Default::default()
                    },
                },
            );
            // Each middle task depends on multiple fan-out tasks
            dependencies.push(Dependency {
                task: task_name.clone(),
                depends_on: "fanout_1".to_string(),
            });
            dependencies.push(Dependency {
                task: task_name,
                depends_on: "fanout_2".to_string(),
            });
        }

        // Fan-in: final task depends on all middle tasks
        tasks.insert(
            "final".to_string(),
            DagTask {
                name: "final".to_string(),
                description: Some("Final task".to_string()),
                config: DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'Final task'".to_string()),
                    ..Default::default()
                },
            },
        );
        dependencies.push(Dependency {
            task: "final".to_string(),
            depends_on: "middle_1".to_string(),
        });
        dependencies.push(Dependency {
            task: "final".to_string(),
            depends_on: "middle_2".to_string(),
        });

        Dag {
            name: "complex_dependency_dag".to_string(),
            schedule: None,
            tasks,
            dependencies,
        }
    }

    /// Create DAG configuration files in different formats
    pub fn create_dag_files(&self) -> HashMap<String, PathBuf> {
        let mut files = HashMap::new();

        // TXT format DAG
        let txt_content = r#"DAG: test_dag_txt

task1: shell echo "Hello from TXT format"
task2: python print("Hello from Python in TXT")
task3: shell echo "Final task"

task2 after task1
task3 after task2
"#;
        let txt_path = self.temp_path().join("test_dag.txt");
        fs::write(&txt_path, txt_content).expect("Failed to write TXT DAG file");
        files.insert("txt".to_string(), txt_path);

        // YAML format DAG
        let yaml_content = r#"dag: test_dag_yaml
schedule: null

tasks:
  task1:
    type: shell
    description: "Hello from YAML format"
    command: echo "Hello from YAML format"
  
  task2:
    type: python
    description: "Python task in YAML"
    script: |
      print("Hello from Python in YAML")
  
  task3:
    type: shell
    description: "Final task in YAML"
    command: echo "Final task"

dependencies:
  - task: task2
    depends_on: task1
  - task: task3
    depends_on: task2
"#;
        let yaml_path = self.temp_path().join("test_dag.yaml");
        fs::write(&yaml_path, yaml_content).expect("Failed to write YAML DAG file");
        files.insert("yaml".to_string(), yaml_path);

        // Markdown format DAG
        let md_content = r#"# Test DAG in Markdown Format

This is a test DAG defined in Markdown format.

## Tasks

### task1
- Type: shell
- Command: `echo "Hello from Markdown format"`
- Description: Hello from Markdown format

### task2
- Type: python
- Script:
```python
print("Hello from Python in Markdown")
```
- Description: Python task in Markdown
- Depends on: task1

### task3
- Type: shell
- Command: `echo "Final task"`
- Description: Final task in Markdown
- Depends on: task2
"#;
        let md_path = self.temp_path().join("test_dag.md");
        fs::write(&md_path, md_content).expect("Failed to write MD DAG file");
        files.insert("md".to_string(), md_path);

        files
    }

    /// Create test files for file operations
    pub fn create_test_files(&self) -> HashMap<String, PathBuf> {
        let mut files = HashMap::new();

        // Simple text file
        let simple_content = "Hello, this is a test file for file operations.";
        let simple_path = self.temp_path().join("simple.txt");
        fs::write(&simple_path, simple_content).expect("Failed to write simple test file");
        files.insert("simple".to_string(), simple_path);

        // JSON file
        let json_content = r#"{
  "name": "test_data",
  "version": "1.0.0",
  "items": [
    {"id": 1, "value": "first"},
    {"id": 2, "value": "second"}
  ]
}"#;
        let json_path = self.temp_path().join("test_data.json");
        fs::write(&json_path, json_content).expect("Failed to write JSON test file");
        files.insert("json".to_string(), json_path);

        // CSV file
        let csv_content = "id,name,value\n1,first,100\n2,second,200\n3,third,300";
        let csv_path = self.temp_path().join("test_data.csv");
        fs::write(&csv_path, csv_content).expect("Failed to write CSV test file");
        files.insert("csv".to_string(), csv_path);

        // Binary file (small)
        let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC];
        let binary_path = self.temp_path().join("binary_data.bin");
        fs::write(&binary_path, binary_data).expect("Failed to write binary test file");
        files.insert("binary".to_string(), binary_path);

        files
    }

    /// Create a subdirectory structure for testing
    pub fn create_directory_structure(&self) -> PathBuf {
        let base_dir = self.temp_path().join("test_structure");
        
        // Create nested directories
        let dirs = [
            "level1",
            "level1/level2a",
            "level1/level2b",
            "level1/level2a/level3",
        ];

        for dir in &dirs {
            let dir_path = base_dir.join(dir);
            fs::create_dir_all(&dir_path).expect("Failed to create test directory");
            
            // Create a file in each directory
            let file_path = dir_path.join("file.txt");
            fs::write(&file_path, format!("File in {}", dir)).expect("Failed to write test file");
        }

        base_dir
    }
}

impl Default for DagFixtures {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn test_linear_dag_creation() {
        let fixtures = DagFixtures::new();
        let dag = fixtures.create_linear_dag();

        assert_eq!(dag.name, "linear_dag");
        assert_eq!(dag.tasks.len(), 3);
        assert_eq!(dag.dependencies.len(), 2);

        // Verify task names
        assert!(dag.tasks.contains_key("task1"));
        assert!(dag.tasks.contains_key("task2"));
        assert!(dag.tasks.contains_key("task3"));

        // Verify dependencies
        assert!(dag.dependencies.iter().any(|d| d.task == "task2" && d.depends_on == "task1"));
        assert!(dag.dependencies.iter().any(|d| d.task == "task3" && d.depends_on == "task2"));
    }

    #[test]
    fn test_diamond_dag_creation() {
        let fixtures = DagFixtures::new();
        let dag = fixtures.create_diamond_dag();

        assert_eq!(dag.name, "diamond_dag");
        assert_eq!(dag.tasks.len(), 4);
        assert_eq!(dag.dependencies.len(), 4);

        // Verify diamond structure
        assert!(dag.dependencies.iter().any(|d| d.task == "branch_a" && d.depends_on == "start"));
        assert!(dag.dependencies.iter().any(|d| d.task == "branch_b" && d.depends_on == "start"));
        assert!(dag.dependencies.iter().any(|d| d.task == "end" && d.depends_on == "branch_a"));
        assert!(dag.dependencies.iter().any(|d| d.task == "end" && d.depends_on == "branch_b"));
    }

    #[test]
    fn test_multi_type_dag_creation() {
        let fixtures = DagFixtures::new();
        let dag = fixtures.create_multi_type_dag();

        assert_eq!(dag.name, "multi_type_dag");
        assert_eq!(dag.tasks.len(), 4);

        // Verify different task types
        let shell_task = dag.tasks.get("shell_task").unwrap();
        assert_eq!(shell_task.config.task_type, Some("shell".to_string()));

        let python_task = dag.tasks.get("python_task").unwrap();
        assert_eq!(python_task.config.task_type, Some("python".to_string()));

        let file_task = dag.tasks.get("file_task").unwrap();
        assert_eq!(file_task.config.task_type, Some("file".to_string()));

        let http_task = dag.tasks.get("http_task").unwrap();
        assert_eq!(http_task.config.task_type, Some("http".to_string()));
    }

    #[test]
    fn test_error_scenario_dag_creation() {
        let fixtures = DagFixtures::new();
        let dag = fixtures.create_error_scenario_dag();

        assert_eq!(dag.name, "error_scenario_dag");
        assert!(dag.tasks.len() >= 5);

        // Verify error scenario tasks exist
        assert!(dag.tasks.contains_key("success_task"));
        assert!(dag.tasks.contains_key("exit_code_failure"));
        assert!(dag.tasks.contains_key("invalid_command"));
        assert!(dag.tasks.contains_key("unsupported_type"));
    }

    #[test]
    fn test_large_dag_creation() {
        let fixtures = DagFixtures::new();
        let task_count = 50;
        let dag = fixtures.create_large_dag(task_count);

        assert_eq!(dag.tasks.len(), task_count);
        assert_eq!(dag.dependencies.len(), 0); // Independent tasks

        // Verify task naming
        for i in 1..=task_count {
            assert!(dag.tasks.contains_key(&format!("task_{}", i)));
        }
    }

    #[test]
    fn test_dag_file_creation() {
        let fixtures = DagFixtures::new();
        let files = fixtures.create_dag_files();

        assert!(files.contains_key("txt"));
        assert!(files.contains_key("yaml"));
        assert!(files.contains_key("md"));

        // Verify files exist
        for (format, path) in &files {
            assert!(path.exists(), "DAG file for format {} does not exist", format);
            let content = fs::read_to_string(path).expect("Failed to read DAG file");
            assert!(!content.is_empty(), "DAG file for format {} is empty", format);
        }
    }

    #[test]
    fn test_test_file_creation() {
        let fixtures = DagFixtures::new();
        let files = fixtures.create_test_files();

        assert!(files.contains_key("simple"));
        assert!(files.contains_key("json"));
        assert!(files.contains_key("csv"));
        assert!(files.contains_key("binary"));

        // Verify files exist and have content
        for (name, path) in &files {
            assert!(path.exists(), "Test file {} does not exist", name);
            let metadata = fs::metadata(path).expect("Failed to get file metadata");
            assert!(metadata.len() > 0, "Test file {} is empty", name);
        }
    }

    #[test]
    fn test_directory_structure_creation() {
        let fixtures = DagFixtures::new();
        let base_dir = fixtures.create_directory_structure();

        assert!(base_dir.exists());
        assert!(base_dir.join("level1").exists());
        assert!(base_dir.join("level1/level2a").exists());
        assert!(base_dir.join("level1/level2b").exists());
        assert!(base_dir.join("level1/level2a/level3").exists());

        // Verify files in directories
        assert!(base_dir.join("level1/file.txt").exists());
        assert!(base_dir.join("level1/level2a/file.txt").exists());
        assert!(base_dir.join("level1/level2a/level3/file.txt").exists());
    }
}