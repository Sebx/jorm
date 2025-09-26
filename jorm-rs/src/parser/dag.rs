use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete DAG definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub name: String,
    pub schedule: Option<String>,
    pub tasks: HashMap<String, Task>,
    pub dependencies: Vec<Dependency>,
}

/// Represents a task within a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    pub config: TaskConfig,
}

/// Task configuration that can be serialized to Python format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    
    // Shell task fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    
    // Python task fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kwargs: Option<HashMap<String, serde_json::Value>>,
    
    // HTTP task fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    
    // File task fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    
    // Common fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
}

/// Represents a dependency between tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub task: String,
    pub depends_on: String,
}

impl Dag {
    /// Create a new empty DAG
    pub fn new(name: String) -> Self {
        Self {
            name,
            schedule: None,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
        }
    }
    
    /// Add a task to the DAG
    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.name.clone(), task);
    }
    
    /// Add a dependency between tasks
    pub fn add_dependency(&mut self, task: String, depends_on: String) {
        self.dependencies.push(Dependency { task, depends_on });
    }
    
    /// Convert to Python-compatible format for execution
    pub fn to_python_format(&self) -> serde_json::Value {
        let mut result = serde_json::Map::new();
        
        result.insert("name".to_string(), serde_json::Value::String(self.name.clone()));
        
        if let Some(schedule) = &self.schedule {
            result.insert("schedule".to_string(), serde_json::Value::String(schedule.clone()));
        }
        
        // Convert tasks to list format (Python expects list, not map)
        let task_names: Vec<String> = self.tasks.keys().cloned().collect();
        result.insert("tasks".to_string(), serde_json::Value::Array(
            task_names.iter().map(|name| serde_json::Value::String(name.clone())).collect()
        ));
        
        // Convert dependencies to tuple format
        let deps: Vec<serde_json::Value> = self.dependencies.iter()
            .map(|dep| serde_json::Value::Array(vec![
                serde_json::Value::String(dep.task.clone()),
                serde_json::Value::String(dep.depends_on.clone())
            ]))
            .collect();
        result.insert("dependencies".to_string(), serde_json::Value::Array(deps));
        
        // Add task configurations if they exist
        if self.tasks.values().any(|task| task.config.task_type.is_some()) {
            let mut task_configs = serde_json::Map::new();
            for (name, task) in &self.tasks {
                if task.config.task_type.is_some() {
                    task_configs.insert(name.clone(), serde_json::to_value(&task.config).unwrap());
                }
            }
            if !task_configs.is_empty() {
                result.insert("task_configs".to_string(), serde_json::Value::Object(task_configs));
            }
        }
        
        serde_json::Value::Object(result)
    }
    
    /// Get tasks in topological order
    pub fn get_topological_order(&self) -> Result<Vec<String>, String> {
        use std::collections::{HashMap, VecDeque};
        
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        
        // Initialize
        for task_name in self.tasks.keys() {
            in_degree.insert(task_name.clone(), 0);
            graph.insert(task_name.clone(), Vec::new());
        }
        
        // Build graph and calculate in-degrees
        for dep in &self.dependencies {
            graph.get_mut(&dep.depends_on).unwrap().push(dep.task.clone());
            *in_degree.get_mut(&dep.task).unwrap() += 1;
        }
        
        // Kahn's algorithm
        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(task, _)| task.clone())
            .collect();
        
        let mut result = Vec::new();
        
        while let Some(task) = queue.pop_front() {
            result.push(task.clone());
            
            if let Some(neighbors) = graph.get(&task) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        
        if result.len() != self.tasks.len() {
            Err("Cyclic dependencies detected".to_string())
        } else {
            Ok(result)
        }
    }
}

impl Task {
    /// Create a new task with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            config: TaskConfig::default(),
        }
    }
    
    /// Create a shell task
    pub fn shell(name: String, command: String) -> Self {
        Self {
            name,
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some(command),
                ..Default::default()
            },
        }
    }
    
    /// Create a Python task
    pub fn python(name: String, module: String, function: String) -> Self {
        Self {
            name,
            description: None,
            config: TaskConfig {
                task_type: Some("python".to_string()),
                module: Some(module),
                function: Some(function),
                ..Default::default()
            },
        }
    }
    
    /// Get the effective timeout for this task
    pub fn effective_timeout(&self, default_timeout: std::time::Duration) -> std::time::Duration {
        if let Some(timeout_secs) = self.config.timeout {
            std::time::Duration::from_secs(timeout_secs)
        } else {
            default_timeout
        }
    }
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            task_type: None,
            command: None,
            module: None,
            function: None,
            script: None,
            args: None,
            kwargs: None,
            method: None,
            url: None,
            headers: None,
            data: None,
            operation: None,
            source: None,
            dest: None,
            destination: None,
            target: None,
            timeout: None,
            retry_count: None,
            backoff_strategy: None,
            depends_on: None,
        }
    }
}