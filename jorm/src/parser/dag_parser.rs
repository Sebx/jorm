use crate::core::{dag::Dag, task::{Task, TaskType}, error::JormError};
use std::collections::HashMap;
use tokio::fs;
use regex::Regex;

pub struct DagParser;

impl DagParser {
    pub fn new() -> Self {
        Self
    }

    pub async fn parse_file(&self, path: &str) -> Result<Dag, JormError> {
        let content = fs::read_to_string(path).await
            .map_err(|e| JormError::FileError(format!("Failed to read DAG file '{}': {}", path, e)))?;
        
        self.parse_content(&content)
    }

    pub fn parse_content(&self, content: &str) -> Result<Dag, JormError> {
        let mut dag = Dag::new("dag".to_string());
        let mut current_task: Option<TaskBuilder> = None;
        let mut line_number = 0;

        for line in content.lines() {
            line_number += 1;
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse task declaration
            if let Some(task_name) = self.parse_task_declaration(trimmed)? {
                // Finalize previous task if exists
                if let Some(builder) = current_task.take() {
                    let task = builder.build(line_number)?;
                    dag.add_task(task);
                }
                current_task = Some(TaskBuilder::new(task_name));
                continue;
            }

            // Parse task properties
            if let Some(ref mut builder) = current_task {
                if trimmed == "}" {
                    // End of task block
                    let builder = current_task.take().unwrap();
                    let task = builder.build(line_number)?;
                    dag.add_task(task);
                } else {
                    self.parse_task_property(builder, trimmed, line_number)?;
                }
            } else {
                return Err(JormError::ParseError(
                    format!("Line {}: Unexpected content outside task block: {}", line_number, trimmed)
                ));
            }
        }

        // Finalize last task if exists
        if let Some(builder) = current_task {
            let task = builder.build(line_number)?;
            dag.add_task(task);
        }

        // Parse dependencies after all tasks are defined
        self.parse_dependencies(&mut dag, content)?;

        // Validate the complete DAG
        dag.validate()?;

        Ok(dag)
    }

    fn parse_task_declaration(&self, line: &str) -> Result<Option<String>, JormError> {
        let task_regex = Regex::new(r"^task\s+(\w+)\s*\{$").unwrap();
        if let Some(captures) = task_regex.captures(line) {
            Ok(Some(captures[1].to_string()))
        } else {
            Ok(None)
        }
    }

    fn parse_task_property(&self, builder: &mut TaskBuilder, line: &str, line_number: usize) -> Result<(), JormError> {
        let property_regex = Regex::new(r"^(\w+):\s*(.+)$").unwrap();
        if let Some(captures) = property_regex.captures(line) {
            let key = captures[1].trim();
            let value = captures[2].trim();
            
            match key {
                "type" => builder.set_type(value, line_number)?,
                "command" => builder.set_property("command".to_string(), self.parse_string_value(value)?),
                "working_dir" => builder.set_property("working_dir".to_string(), self.parse_string_value(value)?),
                "method" => builder.set_property("method".to_string(), self.parse_string_value(value)?),
                "url" => builder.set_property("url".to_string(), self.parse_string_value(value)?),
                "script" => builder.set_property("script".to_string(), self.parse_string_value(value)?),
                "source" => builder.set_property("source".to_string(), self.parse_string_value(value)?),
                "destination" => builder.set_property("destination".to_string(), self.parse_string_value(value)?),
                "path" => builder.set_property("path".to_string(), self.parse_string_value(value)?),
                "headers" => builder.set_headers(self.parse_headers(value)?),
                "body" => builder.set_property("body".to_string(), self.parse_string_value(value)?),
                "args" => builder.set_args(self.parse_args(value)?),
                "depends_on" => {}, // Dependencies handled separately
                _ => {
                    return Err(JormError::ParseError(
                        format!("Line {}: Unknown property '{}'", line_number, key)
                    ));
                }
            }
        } else {
            return Err(JormError::ParseError(
                format!("Line {}: Invalid property format: {}", line_number, line)
            ));
        }
        Ok(())
    }

    fn parse_dependencies(&self, dag: &mut Dag, content: &str) -> Result<(), JormError> {
        let mut current_task: Option<String> = None;
        let mut line_number = 0;

        for line in content.lines() {
            line_number += 1;
            let trimmed = line.trim();
            
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Track current task
            if let Some(task_name) = self.parse_task_declaration(trimmed)? {
                current_task = Some(task_name);
                continue;
            }

            // Parse depends_on property
            if let Some(ref task_name) = current_task {
                if trimmed.starts_with("depends_on:") {
                    let deps_str = trimmed.strip_prefix("depends_on:").unwrap().trim();
                    let dependencies = self.parse_dependency_list(deps_str)?;
                    dag.add_dependency(task_name.clone(), dependencies);
                }
            }

            if trimmed == "}" {
                current_task = None;
            }
        }

        Ok(())
    }

    fn parse_string_value(&self, value: &str) -> Result<String, JormError> {
        if (value.starts_with('"') && value.ends_with('"')) || 
           (value.starts_with('\'') && value.ends_with('\'')) {
            Ok(value[1..value.len()-1].to_string())
        } else {
            Ok(value.to_string())
        }
    }

    fn parse_headers(&self, value: &str) -> Result<HashMap<String, String>, JormError> {
        let mut headers = HashMap::new();
        
        if !value.starts_with('{') || !value.ends_with('}') {
            return Err(JormError::ParseError("Headers must be in {} format".to_string()));
        }

        let content = &value[1..value.len()-1];
        if content.trim().is_empty() {
            return Ok(headers);
        }

        for pair in content.split(',') {
            let parts: Vec<&str> = pair.split(':').collect();
            if parts.len() != 2 {
                return Err(JormError::ParseError("Invalid header format".to_string()));
            }
            let key = self.parse_string_value(parts[0].trim())?;
            let val = self.parse_string_value(parts[1].trim())?;
            headers.insert(key, val);
        }

        Ok(headers)
    }

    fn parse_args(&self, value: &str) -> Result<Vec<String>, JormError> {
        let mut args = Vec::new();
        
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(JormError::ParseError("Args must be in [] format".to_string()));
        }

        let content = &value[1..value.len()-1];
        if content.trim().is_empty() {
            return Ok(args);
        }

        for arg in content.split(',') {
            args.push(self.parse_string_value(arg.trim())?);
        }

        Ok(args)
    }

    fn parse_dependency_list(&self, value: &str) -> Result<Vec<String>, JormError> {
        let mut deps = Vec::new();
        
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(JormError::ParseError("Dependencies must be in [] format".to_string()));
        }

        let content = &value[1..value.len()-1];
        if content.trim().is_empty() {
            return Ok(deps);
        }

        for dep in content.split(',') {
            deps.push(dep.trim().to_string());
        }

        Ok(deps)
    }
}

struct TaskBuilder {
    name: String,
    task_type: Option<String>,
    properties: HashMap<String, String>,
    headers: Option<HashMap<String, String>>,
    args: Option<Vec<String>>,
}

impl TaskBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            task_type: None,
            properties: HashMap::new(),
            headers: None,
            args: None,
        }
    }

    fn set_type(&mut self, task_type: &str, line_number: usize) -> Result<(), JormError> {
        let valid_types = ["shell", "http", "python", "rust", "file_copy", "file_move", "file_delete", "jorm"];
        if !valid_types.contains(&task_type) {
            return Err(JormError::ParseError(
                format!("Line {}: Invalid task type '{}'. Valid types: {:?}", 
                       line_number, task_type, valid_types)
            ));
        }
        self.task_type = Some(task_type.to_string());
        Ok(())
    }

    fn set_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    fn set_headers(&mut self, headers: HashMap<String, String>) {
        self.headers = Some(headers);
    }

    fn set_args(&mut self, args: Vec<String>) {
        self.args = Some(args);
    }

    fn build(self, line_number: usize) -> Result<Task, JormError> {
        let task_type_str = self.task_type.ok_or_else(|| {
            JormError::ParseError(format!("Line {}: Task '{}' missing type", line_number, self.name))
        })?;

        let task_type = match task_type_str.as_str() {
            "shell" => {
                let command = self.properties.get("command")
                    .ok_or_else(|| JormError::ParseError(format!("Shell task '{}' missing command", self.name)))?
                    .clone();
                let working_dir = self.properties.get("working_dir").cloned();
                TaskType::Shell { command, working_dir }
            }
            "http" => {
                let method = self.properties.get("method")
                    .ok_or_else(|| JormError::ParseError(format!("HTTP task '{}' missing method", self.name)))?
                    .clone();
                let url = self.properties.get("url")
                    .ok_or_else(|| JormError::ParseError(format!("HTTP task '{}' missing url", self.name)))?
                    .clone();
                let body = self.properties.get("body").cloned();
                TaskType::Http { method, url, headers: self.headers, body }
            }
            "python" => {
                let script = self.properties.get("script")
                    .ok_or_else(|| JormError::ParseError(format!("Python task '{}' missing script", self.name)))?
                    .clone();
                let working_dir = self.properties.get("working_dir").cloned();
                TaskType::Python { script, args: self.args, working_dir }
            }
            "rust" => {
                let command = self.properties.get("command")
                    .ok_or_else(|| JormError::ParseError(format!("Rust task '{}' missing command", self.name)))?
                    .clone();
                let working_dir = self.properties.get("working_dir").cloned();
                TaskType::Rust { command, working_dir }
            }
            "file_copy" => {
                let source = self.properties.get("source")
                    .ok_or_else(|| JormError::ParseError(format!("File copy task '{}' missing source", self.name)))?
                    .clone();
                let destination = self.properties.get("destination")
                    .ok_or_else(|| JormError::ParseError(format!("File copy task '{}' missing destination", self.name)))?
                    .clone();
                TaskType::FileCopy { source, destination }
            }
            "file_move" => {
                let source = self.properties.get("source")
                    .ok_or_else(|| JormError::ParseError(format!("File move task '{}' missing source", self.name)))?
                    .clone();
                let destination = self.properties.get("destination")
                    .ok_or_else(|| JormError::ParseError(format!("File move task '{}' missing destination", self.name)))?
                    .clone();
                TaskType::FileMove { source, destination }
            }
            "file_delete" => {
                let path = self.properties.get("path")
                    .ok_or_else(|| JormError::ParseError(format!("File delete task '{}' missing path", self.name)))?
                    .clone();
                TaskType::FileDelete { path }
            }
            "jorm" => {
                let command = self.properties.get("command")
                    .ok_or_else(|| JormError::ParseError(format!("Jorm task '{}' missing command", self.name)))?
                    .clone();
                let working_dir = self.properties.get("working_dir").cloned();
                TaskType::Jorm { command, args: self.args, working_dir }
            }
            _ => unreachable!(), // Already validated in set_type
        };

        let task = Task::new(self.name, task_type);
        task.validate()?;
        Ok(task)
    }
}