use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::task::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub name: String,
    pub tasks: HashMap<String, Task>,
    pub dependencies: HashMap<String, Vec<String>>,
}

impl Dag {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tasks: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.name.clone(), task);
    }

    pub fn add_dependency(&mut self, task_name: String, depends_on: Vec<String>) {
        self.dependencies.insert(task_name, depends_on);
    }

    pub fn validate(&self) -> Result<(), crate::core::error::JormError> {
        // Check for circular dependencies
        self.check_circular_dependencies()?;
        
        // Validate task references in dependencies
        self.validate_task_references()?;
        
        // Validate all tasks have required parameters
        self.validate_task_parameters()?;
        
        // Validate DAG has at least one task
        if self.tasks.is_empty() {
            return Err(crate::core::error::JormError::ParseError(
                "DAG must contain at least one task".to_string()
            ));
        }
        
        Ok(())
    }

    fn check_circular_dependencies(&self) -> Result<(), crate::core::error::JormError> {
        // Simple cycle detection using DFS with path tracking
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();
        let mut path = Vec::new();

        for task_name in self.tasks.keys() {
            if !visited.get(task_name).unwrap_or(&false) {
                if let Some(cycle_path) = self.find_cycle(task_name, &mut visited, &mut rec_stack, &mut path)? {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Circular dependency detected: {}", cycle_path.join(" -> "))
                    ));
                }
            }
        }
        Ok(())
    }

    fn find_cycle(
        &self,
        task_name: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
        path: &mut Vec<String>,
    ) -> Result<Option<Vec<String>>, crate::core::error::JormError> {
        visited.insert(task_name.to_string(), true);
        rec_stack.insert(task_name.to_string(), true);
        path.push(task_name.to_string());

        if let Some(deps) = self.dependencies.get(task_name) {
            for dep in deps {
                if !visited.get(dep).unwrap_or(&false) {
                    if let Some(cycle) = self.find_cycle(dep, visited, rec_stack, path)? {
                        return Ok(Some(cycle));
                    }
                } else if *rec_stack.get(dep).unwrap_or(&false) {
                    // Found cycle - construct the cycle path
                    let cycle_start = path.iter().position(|x| x == dep).unwrap();
                    let mut cycle_path = path[cycle_start..].to_vec();
                    cycle_path.push(dep.to_string());
                    return Ok(Some(cycle_path));
                }
            }
        }

        rec_stack.insert(task_name.to_string(), false);
        path.pop();
        Ok(None)
    }

    fn validate_task_references(&self) -> Result<(), crate::core::error::JormError> {
        for (task_name, deps) in &self.dependencies {
            if !self.tasks.contains_key(task_name) {
                return Err(crate::core::error::JormError::ParseError(
                    format!("Task '{}' referenced in dependencies but not defined", task_name)
                ));
            }
            
            for dep in deps {
                if !self.tasks.contains_key(dep) {
                    return Err(crate::core::error::JormError::ParseError(
                        format!("Dependency '{}' for task '{}' not found", dep, task_name)
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_task_parameters(&self) -> Result<(), crate::core::error::JormError> {
        for (task_name, task) in &self.tasks {
            // Validate each task's parameters
            task.validate().map_err(|e| {
                crate::core::error::JormError::ParseError(
                    format!("Task '{}' validation failed: {}", task_name, e)
                )
            })?;
        }
        Ok(())
    }
}