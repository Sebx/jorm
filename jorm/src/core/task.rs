use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub task_type: TaskType,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Shell { 
        command: String, 
        working_dir: Option<String> 
    },
    Http { 
        method: String, 
        url: String, 
        headers: Option<HashMap<String, String>>, 
        body: Option<String> 
    },
    Python { 
        script: String, 
        args: Option<Vec<String>>, 
        working_dir: Option<String> 
    },
    Rust { 
        command: String, 
        working_dir: Option<String> 
    },
    FileCopy { 
        source: String, 
        destination: String 
    },
    FileMove { 
        source: String, 
        destination: String 
    },
    FileDelete { 
        path: String 
    },
    Jorm {
        command: String,
        args: Option<Vec<String>>,
        working_dir: Option<String>
    },
}

impl Task {
    pub fn new(name: String, task_type: TaskType) -> Self {
        Self {
            name,
            task_type,
            parameters: HashMap::new(),
        }
    }

    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }

    pub fn validate(&self) -> Result<(), crate::core::error::JormError> {
        match &self.task_type {
            TaskType::Shell { command, .. } => {
                if command.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Shell task '{}' has empty command", self.name)
                    ));
                }
            }
            TaskType::Http { method, url, .. } => {
                if method.is_empty() || url.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("HTTP task '{}' missing method or URL", self.name)
                    ));
                }
            }
            TaskType::Python { script, .. } => {
                if script.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Python task '{}' has empty script", self.name)
                    ));
                }
            }
            TaskType::Rust { command, .. } => {
                if command.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Rust task '{}' has empty command", self.name)
                    ));
                }
            }
            TaskType::FileCopy { source, destination } => {
                if source.is_empty() || destination.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("File copy task '{}' missing source or destination", self.name)
                    ));
                }
            }
            TaskType::FileMove { source, destination } => {
                if source.is_empty() || destination.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("File move task '{}' missing source or destination", self.name)
                    ));
                }
            }
            TaskType::FileDelete { path } => {
                if path.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("File delete task '{}' has empty path", self.name)
                    ));
                }
            }
            TaskType::Jorm { command, .. } => {
                if command.is_empty() {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Jorm task '{}' has empty command", self.name)
                    ));
                }
            }
        }
        Ok(())
    }
}