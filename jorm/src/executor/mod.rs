pub mod file;
pub mod http;
pub mod jorm;
pub mod python;
pub mod rust;
pub mod shell;

#[cfg(test)]
mod tests;

use crate::core::engine::{ExecutionResult, TaskResult};
use crate::core::{dag::Dag, error::JormError, task::TaskType};
use file::FileExecutor;
use http::HttpExecutor;
use jorm::JormExecutor;
use python::PythonExecutor;
use rust::RustExecutor;
use shell::ShellExecutor;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct TaskExecutor {
    shell_executor: ShellExecutor,
    http_executor: HttpExecutor,
    python_executor: PythonExecutor,
    rust_executor: RustExecutor,
    file_executor: FileExecutor,
    jorm_executor: JormExecutor,
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self {
            shell_executor: ShellExecutor::new(),
            http_executor: HttpExecutor::new(),
            python_executor: PythonExecutor::new(),
            rust_executor: RustExecutor::new(),
            file_executor: FileExecutor::new(),
            jorm_executor: JormExecutor::new(),
        }
    }

    pub async fn execute_dag(&self, dag: &Dag) -> Result<ExecutionResult, JormError> {
        // Resolve execution order using topological sort
        let execution_order = self.resolve_execution_order(dag)?;

        let mut task_results = Vec::new();
        let mut overall_success = true;

        // Execute tasks in dependency order
        for task_name in execution_order {
            if let Some(task) = dag.tasks.get(&task_name) {
                println!("Executing task: {}", task_name);

                match self.execute_task(&task_name, &task.task_type).await {
                    Ok(result) => {
                        if !result.success {
                            overall_success = false;
                            println!(
                                "Task '{}' failed: {}",
                                task_name,
                                result.error.as_deref().unwrap_or("Unknown error")
                            );
                        } else {
                            println!("Task '{}' completed successfully", task_name);
                        }
                        task_results.push(result);
                    }
                    Err(e) => {
                        overall_success = false;
                        let error_msg = format!("Task execution error: {}", e);
                        println!("Task '{}' failed with error: {}", task_name, error_msg);
                        task_results.push(TaskResult {
                            task_name: task_name.clone(),
                            success: false,
                            output: String::new(),
                            error: Some(error_msg),
                        });
                        // Continue with remaining tasks where possible
                    }
                }
            }
        }

        let message = if overall_success {
            format!("DAG '{}' executed successfully", dag.name)
        } else {
            format!("DAG '{}' completed with some failures", dag.name)
        };

        Ok(ExecutionResult {
            success: overall_success,
            message,
            task_results,
        })
    }

    /// Resolve task execution order using topological sort
    /// Returns tasks in dependency order (dependencies first)
    fn resolve_execution_order(&self, dag: &Dag) -> Result<Vec<String>, JormError> {
        let mut in_degree = HashMap::new();
        let mut graph = HashMap::new();

        // Initialize in-degree count for all tasks
        for task_name in dag.tasks.keys() {
            in_degree.insert(task_name.clone(), 0);
            graph.insert(task_name.clone(), Vec::new());
        }

        // Build the dependency graph and calculate in-degrees
        for (task_name, dependencies) in &dag.dependencies {
            for dep in dependencies {
                // dep -> task_name (dep must execute before task_name)
                graph.get_mut(dep).unwrap().push(task_name.clone());
                *in_degree.get_mut(task_name).unwrap() += 1;
            }
        }

        // Find tasks with no dependencies (in-degree = 0)
        let mut queue = VecDeque::new();
        for (task_name, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(task_name.clone());
            }
        }

        let mut execution_order = Vec::new();
        let mut processed = HashSet::new();

        // Process tasks in topological order
        while let Some(current_task) = queue.pop_front() {
            execution_order.push(current_task.clone());
            processed.insert(current_task.clone());

            // Reduce in-degree for dependent tasks
            if let Some(dependents) = graph.get(&current_task) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        // Check if all tasks were processed (no circular dependencies)
        if execution_order.len() != dag.tasks.len() {
            return Err(JormError::ExecutionError(
                "Circular dependency detected - cannot resolve execution order".to_string(),
            ));
        }

        Ok(execution_order)
    }

    pub async fn execute_task(
        &self,
        task_name: &str,
        task_type: &TaskType,
    ) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Shell { .. } => self.shell_executor.execute(task_name, task_type).await,
            TaskType::Http { .. } => self.http_executor.execute(task_name, task_type).await,
            TaskType::Python { .. } => self.python_executor.execute(task_name, task_type).await,
            TaskType::Rust { .. } => self.rust_executor.execute(task_name, task_type).await,
            TaskType::FileCopy { .. } | TaskType::FileMove { .. } | TaskType::FileDelete { .. } => {
                self.file_executor.execute(task_name, task_type).await
            }
            TaskType::Jorm { .. } => self.jorm_executor.execute(task_name, task_type).await,
        }
    }
}
