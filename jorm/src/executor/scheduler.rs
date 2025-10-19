//! Dependency resolution and scheduling engine for DAG execution
//!
//! This module provides sophisticated dependency resolution with topological sorting,
//! cycle detection, and parallel execution planning capabilities.

use crate::executor::error::ExecutorError;
use crate::parser::Dag;
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// Task execution state for dependency tracking
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Execution graph for tracking task states and dependencies
#[derive(Debug)]
pub struct ExecutionGraph {
    /// Task states
    task_states: HashMap<String, TaskState>,
    /// Task dependencies (task -> list of dependencies)
    dependencies: HashMap<String, Vec<String>>,
    /// Reverse dependencies (task -> list of tasks that depend on it)
    dependents: HashMap<String, Vec<String>>,
    /// Queue of ready tasks
    ready_queue: VecDeque<String>,
    /// Currently running tasks
    running_tasks: HashSet<String>,
    /// Completed tasks
    completed_tasks: HashSet<String>,
    /// Failed tasks
    failed_tasks: HashSet<String>,
}

/// Dependency scheduler with topological sorting algorithm
pub struct DependencyScheduler {
    dag: Dag,
    execution_graph: ExecutionGraph,
}

impl ExecutionGraph {
    /// Create a new execution graph from a DAG
    pub fn new(dag: &Dag) -> Result<Self> {
        let mut task_states = HashMap::new();
        let mut dependencies = HashMap::new();
        let mut dependents = HashMap::new();
        let mut ready_queue = VecDeque::new();

        // Initialize all tasks as pending
        for task_name in dag.tasks.keys() {
            task_states.insert(task_name.clone(), TaskState::Pending);
            dependencies.insert(task_name.clone(), Vec::new());
            dependents.insert(task_name.clone(), Vec::new());
        }

        // Build dependency and dependent maps from DAG dependencies
        for dependency in &dag.dependencies {
            let task = &dependency.task;
            let depends_on = &dependency.depends_on;

            // Add dependency
            if let Some(deps) = dependencies.get_mut(task) {
                deps.push(depends_on.clone());
            }

            // Add reverse dependency
            if let Some(deps) = dependents.get_mut(depends_on) {
                deps.push(task.clone());
            }
        }

        // Detect cycles using depth-first search
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        
        for task_name in dag.tasks.keys() {
            if !visited.contains(task_name) {
                Self::detect_cycle_dfs(
                    task_name,
                    &dependencies,
                    &mut visited,
                    &mut visiting,
                )?;
            }
        }

        // Initialize ready queue with tasks that have no dependencies
        for (task_name, deps) in &dependencies {
            if deps.is_empty() {
                ready_queue.push_back(task_name.clone());
                task_states.insert(task_name.clone(), TaskState::Ready);
            }
        }

        Ok(Self {
            task_states,
            dependencies,
            dependents,
            ready_queue,
            running_tasks: HashSet::new(),
            completed_tasks: HashSet::new(),
            failed_tasks: HashSet::new(),
        })
    }

    /// Detect cycles in the dependency graph using depth-first search
    fn detect_cycle_dfs(
        task: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(task) {
            return Err(ExecutorError::DependencyCycle.into());
        }

        if visited.contains(task) {
            return Ok(());
        }

        visiting.insert(task.to_string());

        if let Some(deps) = dependencies.get(task) {
            for dep in deps {
                Self::detect_cycle_dfs(dep, dependencies, visited, visiting)?;
            }
        }

        visiting.remove(task);
        visited.insert(task.to_string());

        Ok(())
    }

    /// Get the next ready task from the queue
    pub fn get_next_ready_task(&mut self) -> Option<String> {
        self.ready_queue.pop_front()
    }

    /// Mark a task as running
    pub fn mark_task_running(&mut self, task_name: &str) {
        self.task_states.insert(task_name.to_string(), TaskState::Running);
        self.running_tasks.insert(task_name.to_string());
    }

    /// Mark a task as completed and update dependent tasks
    pub fn mark_task_completed(&mut self, task_name: &str) {
        self.task_states.insert(task_name.to_string(), TaskState::Completed);
        self.running_tasks.remove(task_name);
        self.completed_tasks.insert(task_name.to_string());
        
        self.update_ready_tasks();
    }

    /// Mark a task as failed and handle failure propagation
    pub fn mark_task_failed(&mut self, task_name: &str) {
        self.task_states.insert(task_name.to_string(), TaskState::Failed);
        self.running_tasks.remove(task_name);
        self.failed_tasks.insert(task_name.to_string());
        
        // Mark dependent tasks as skipped
        self.mark_dependents_as_skipped(task_name);
        
        self.update_ready_tasks();
    }

    /// Mark all dependent tasks as skipped when a dependency fails
    fn mark_dependents_as_skipped(&mut self, failed_task: &str) {
        if let Some(dependents) = self.dependents.get(failed_task).cloned() {
            for dependent in dependents {
                if self.task_states.get(&dependent) == Some(&TaskState::Pending) 
                    || self.task_states.get(&dependent) == Some(&TaskState::Ready) {
                    self.task_states.insert(dependent.clone(), TaskState::Skipped);
                    // Remove from ready queue if present
                    self.ready_queue.retain(|task| task != &dependent);
                    // Recursively skip dependents
                    self.mark_dependents_as_skipped(&dependent);
                }
            }
        }
    }

    /// Update ready tasks based on completed dependencies
    pub fn update_ready_tasks(&mut self) {
        let mut newly_ready = Vec::new();

        for (task_name, task_state) in &self.task_states {
            if *task_state == TaskState::Pending {
                // Check if all dependencies are completed
                if let Some(deps) = self.dependencies.get(task_name) {
                    let all_deps_completed = deps.iter().all(|dep| {
                        self.task_states.get(dep) == Some(&TaskState::Completed)
                    });

                    if all_deps_completed {
                        newly_ready.push(task_name.clone());
                    }
                }
            }
        }

        // Mark newly ready tasks and add to queue
        for task_name in newly_ready {
            self.task_states.insert(task_name.clone(), TaskState::Ready);
            self.ready_queue.push_back(task_name);
        }
    }

    /// Check if there are any pending tasks
    pub fn has_pending_tasks(&self) -> bool {
        self.task_states.values().any(|state| {
            matches!(state, TaskState::Pending | TaskState::Ready | TaskState::Running)
        })
    }

    /// Get the number of failed tasks
    pub fn failed_task_count(&self) -> usize {
        self.failed_tasks.len()
    }

    /// Get the number of skipped tasks
    pub fn skipped_task_count(&self) -> usize {
        self.task_states.values()
            .filter(|state| **state == TaskState::Skipped)
            .count()
    }

    /// Get the current state of a task
    pub fn get_task_state(&self, task_name: &str) -> Option<&TaskState> {
        self.task_states.get(task_name)
    }

    /// Get all tasks in a specific state
    pub fn get_tasks_in_state(&self, state: TaskState) -> Vec<String> {
        self.task_states.iter()
            .filter(|(_, task_state)| **task_state == state)
            .map(|(task_name, _)| task_name.clone())
            .collect()
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        let total_tasks = self.task_states.len();
        let completed_tasks = self.completed_tasks.len();
        let failed_tasks = self.failed_tasks.len();
        let running_tasks = self.running_tasks.len();
        let skipped_tasks = self.task_states.values()
            .filter(|state| **state == TaskState::Skipped)
            .count();
        let pending_tasks = self.task_states.values()
            .filter(|state| matches!(state, TaskState::Pending | TaskState::Ready))
            .count();

        ExecutionStats {
            total_tasks,
            completed_tasks,
            failed_tasks,
            running_tasks,
            skipped_tasks,
            pending_tasks,
        }
    }
}

impl DependencyScheduler {
    /// Create a new dependency scheduler
    pub fn new(dag: Dag) -> Result<Self> {
        let execution_graph = ExecutionGraph::new(&dag)?;
        
        Ok(Self {
            dag,
            execution_graph,
        })
    }

    /// Get ready tasks that can be executed
    pub fn get_ready_tasks(&mut self) -> Vec<String> {
        let mut ready_tasks = Vec::new();
        
        while let Some(task) = self.execution_graph.get_next_ready_task() {
            ready_tasks.push(task);
        }
        
        ready_tasks
    }

    /// Mark a task as completed
    pub fn mark_task_completed(&mut self, task_id: &str) -> Result<()> {
        self.execution_graph.mark_task_completed(task_id);
        Ok(())
    }

    /// Mark a task as failed
    pub fn mark_task_failed(&mut self, task_id: &str) -> Result<()> {
        self.execution_graph.mark_task_failed(task_id);
        Ok(())
    }

    /// Mark a task as running
    pub fn mark_task_running(&mut self, task_id: &str) -> Result<()> {
        self.execution_graph.mark_task_running(task_id);
        Ok(())
    }

    /// Check if there are pending tasks
    pub fn has_pending_tasks(&self) -> bool {
        self.execution_graph.has_pending_tasks()
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        self.execution_graph.get_execution_stats()
    }

    /// Get the DAG reference
    pub fn dag(&self) -> &Dag {
        &self.dag
    }

    /// Get the execution graph reference
    pub fn execution_graph(&self) -> &ExecutionGraph {
        &self.execution_graph
    }

    /// Get mutable execution graph reference
    pub fn execution_graph_mut(&mut self) -> &mut ExecutionGraph {
        &mut self.execution_graph
    }

    /// Resolve task dependencies and return topological order
    pub fn resolve_dependencies(&self) -> Result<Vec<String>> {
        let mut execution_order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        // Topological sort using DFS
        for task_name in self.dag.tasks.keys() {
            if !visited.contains(task_name) {
                self.dfs_resolve_dependencies(
                    task_name,
                    &mut visited,
                    &mut visiting,
                    &mut execution_order,
                )?;
            }
        }

        // Reverse the order to get correct execution sequence
        execution_order.reverse();

        Ok(execution_order)
    }

    /// DFS helper for dependency resolution with cycle detection
    fn dfs_resolve_dependencies(
        &self,
        task_name: &str,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        execution_order: &mut Vec<String>,
    ) -> Result<()> {
        if visiting.contains(task_name) {
            return Err(ExecutorError::DependencyCycle.into());
        }

        if visited.contains(task_name) {
            return Ok(());
        }

        visiting.insert(task_name.to_string());

        // Process dependencies first - check DAG dependencies
        for dependency in &self.dag.dependencies {
            if dependency.task == task_name {
                self.dfs_resolve_dependencies(
                    &dependency.depends_on,
                    visited,
                    visiting,
                    execution_order,
                )?;
            }
        }

        visiting.remove(task_name);
        visited.insert(task_name.to_string());

        // Add current task to the beginning of the order (dependencies first)
        execution_order.insert(0, task_name.to_string());

        Ok(())
    }
}

/// Execution statistics for monitoring progress
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub running_tasks: usize,
    pub skipped_tasks: usize,
    pub pending_tasks: usize,
}

impl ExecutionStats {
    /// Calculate completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_tasks == 0 {
            return 100.0;
        }
        
        (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
    }

    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.pending_tasks == 0 && self.running_tasks == 0
    }

    /// Check if execution was successful
    pub fn is_successful(&self) -> bool {
        self.is_complete() && self.failed_tasks == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Dependency, Task as ParserTask, TaskConfig};

    fn create_test_dag() -> Dag {
        let mut tasks = HashMap::new();
        
        // Create test tasks
        tasks.insert("task1".to_string(), ParserTask {
            name: "Task 1".to_string(),
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo task1".to_string()),
                ..Default::default()
            },
        });
        
        tasks.insert("task2".to_string(), ParserTask {
            name: "Task 2".to_string(),
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo task2".to_string()),
                ..Default::default()
            },
        });
        
        tasks.insert("task3".to_string(), ParserTask {
            name: "Task 3".to_string(),
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo task3".to_string()),
                ..Default::default()
            },
        });

        let dependencies = vec![
            Dependency {
                task: "task2".to_string(),
                depends_on: "task1".to_string(),
            },
            Dependency {
                task: "task3".to_string(),
                depends_on: "task2".to_string(),
            },
        ];

        Dag {
            name: "test_dag".to_string(),
            schedule: None,
            tasks,
            dependencies,
        }
    }

    fn create_cyclic_dag() -> Dag {
        let mut tasks = HashMap::new();
        
        tasks.insert("task1".to_string(), ParserTask {
            name: "Task 1".to_string(),
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo task1".to_string()),
                ..Default::default()
            },
        });
        
        tasks.insert("task2".to_string(), ParserTask {
            name: "Task 2".to_string(),
            description: None,
            config: TaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo task2".to_string()),
                ..Default::default()
            },
        });

        let dependencies = vec![
            Dependency {
                task: "task1".to_string(),
                depends_on: "task2".to_string(),
            },
            Dependency {
                task: "task2".to_string(),
                depends_on: "task1".to_string(),
            },
        ];

        Dag {
            name: "cyclic_dag".to_string(),
            schedule: None,
            tasks,
            dependencies,
        }
    }

    #[test]
    fn test_execution_graph_creation() {
        let dag = create_test_dag();
        let graph = ExecutionGraph::new(&dag).unwrap();
        
        // task1 should be ready (no dependencies)
        assert_eq!(graph.get_task_state("task1"), Some(&TaskState::Ready));
        // task2 and task3 should be pending (have dependencies)
        assert_eq!(graph.get_task_state("task2"), Some(&TaskState::Pending));
        assert_eq!(graph.get_task_state("task3"), Some(&TaskState::Pending));
    }

    #[test]
    fn test_cycle_detection() {
        let dag = create_cyclic_dag();
        let result = ExecutionGraph::new(&dag);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().downcast_ref::<ExecutorError>(), 
                        Some(ExecutorError::DependencyCycle)));
    }

    #[test]
    fn test_dependency_resolution() {
        let dag = create_test_dag();
        let scheduler = DependencyScheduler::new(dag).unwrap();
        
        let execution_order = scheduler.resolve_dependencies().unwrap();
        
        // task1 should come before task2, task2 before task3
        let task1_pos = execution_order.iter().position(|t| t == "task1").unwrap();
        let task2_pos = execution_order.iter().position(|t| t == "task2").unwrap();
        let task3_pos = execution_order.iter().position(|t| t == "task3").unwrap();
        
        assert!(task1_pos < task2_pos);
        assert!(task2_pos < task3_pos);
    }

    #[test]
    fn test_task_completion_flow() {
        let dag = create_test_dag();
        let mut scheduler = DependencyScheduler::new(dag).unwrap();
        
        // Initially, only task1 should be ready
        let ready_tasks = scheduler.get_ready_tasks();
        assert_eq!(ready_tasks, vec!["task1"]);
        
        // Mark task1 as running
        scheduler.mark_task_running("task1").unwrap();
        assert_eq!(scheduler.execution_graph.get_task_state("task1"), Some(&TaskState::Running));
        
        // Complete task1
        scheduler.mark_task_completed("task1").unwrap();
        assert_eq!(scheduler.execution_graph.get_task_state("task1"), Some(&TaskState::Completed));
        
        // Now task2 should be ready
        let ready_tasks = scheduler.get_ready_tasks();
        assert_eq!(ready_tasks, vec!["task2"]);
    }

    #[test]
    fn test_failure_propagation() {
        let dag = create_test_dag();
        let mut scheduler = DependencyScheduler::new(dag).unwrap();
        
        // Get task1 and mark it as failed
        let ready_tasks = scheduler.get_ready_tasks();
        assert_eq!(ready_tasks, vec!["task1"]);
        
        scheduler.mark_task_running("task1").unwrap();
        scheduler.mark_task_failed("task1").unwrap();
        
        // task2 and task3 should be skipped
        assert_eq!(scheduler.execution_graph.get_task_state("task2"), Some(&TaskState::Skipped));
        assert_eq!(scheduler.execution_graph.get_task_state("task3"), Some(&TaskState::Skipped));
        
        // No more ready tasks
        let ready_tasks = scheduler.get_ready_tasks();
        assert!(ready_tasks.is_empty());
    }

    #[test]
    fn test_execution_stats() {
        let dag = create_test_dag();
        let mut scheduler = DependencyScheduler::new(dag).unwrap();
        
        let stats = scheduler.get_execution_stats();
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.pending_tasks, 3); // task1 is Ready, task2 and task3 are Pending (Ready + Pending = 3)
        assert_eq!(stats.completed_tasks, 0);
        assert_eq!(stats.running_tasks, 0);
        
        // Complete task1
        scheduler.get_ready_tasks(); // Get task1
        scheduler.mark_task_running("task1").unwrap();
        scheduler.mark_task_completed("task1").unwrap();
        
        let stats = scheduler.get_execution_stats();
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.pending_tasks, 2); // task2 is now Ready, task3 still Pending
    }
}