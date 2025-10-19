/// Domain services - Business logic that doesn't belong to a single entity
/// 
/// Services contain domain logic that operates on multiple entities or
/// coordinates complex business operations.

use super::entities::{Dag, Dependency, Task};
use super::errors::{DomainError, DomainResult};
use super::value_objects::TaskId;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, info, instrument, warn};

/// Service for validating DAG structures
/// 
/// Provides comprehensive validation of DAG integrity including
/// dependency validation, cycle detection, and structural checks.
pub struct DagValidator;

impl DagValidator {
    /// Validates a DAG and returns all validation errors
    /// 
    /// # Arguments
    /// * `dag` - The DAG to validate
    /// 
    /// # Returns
    /// * `DomainResult<Vec<String>>` - List of validation errors (empty if valid)
    #[instrument(skip(dag), fields(dag_id = %dag.id(), task_count = dag.task_count()))]
    pub fn validate(dag: &Dag) -> DomainResult<Vec<String>> {
        debug!("Starting DAG validation");
        let mut errors = Vec::new();

        // Check for empty DAG
        if dag.is_empty() {
            errors.push("DAG contains no tasks".to_string());
        }

        // Validate all dependencies reference existing tasks
        for dep in dag.dependencies() {
            if !dag.tasks().contains_key(dep.task_id()) {
                errors.push(format!(
                    "Dependency references non-existent task: {}",
                    dep.task_id()
                ));
            }
            if !dag.tasks().contains_key(dep.depends_on()) {
                errors.push(format!(
                    "Dependency references non-existent prerequisite: {}",
                    dep.depends_on()
                ));
            }
        }

        // Check for cycles
        if let Err(e) = Self::check_for_cycles(dag) {
            errors.push(e.to_string());
        }

        if errors.is_empty() {
            info!("DAG validation passed");
        } else {
            warn!(error_count = errors.len(), "DAG validation failed");
        }

        Ok(errors)
    }

    /// Checks if the DAG contains any circular dependencies
    /// 
    /// # Arguments
    /// * `dag` - The DAG to check
    /// 
    /// # Returns
    /// * `DomainResult<()>` - Ok if no cycles, error if cycle detected
    fn check_for_cycles(dag: &Dag) -> DomainResult<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in dag.tasks().keys() {
            if !visited.contains(task_id) {
                Self::dfs_cycle_check(
                    task_id,
                    dag.dependencies(),
                    &mut visited,
                    &mut rec_stack,
                )?;
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_check(
        task_id: &TaskId,
        dependencies: &HashSet<Dependency>,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
    ) -> DomainResult<()> {
        visited.insert(task_id.clone());
        rec_stack.insert(task_id.clone());

        // Find all tasks that this task depends on
        for dep in dependencies {
            if dep.task_id() == task_id {
                let prerequisite = dep.depends_on();
                if !visited.contains(prerequisite) {
                    Self::dfs_cycle_check(prerequisite, dependencies, visited, rec_stack)?;
                } else if rec_stack.contains(prerequisite) {
                    return Err(DomainError::CircularDependency(format!(
                        "{} -> {}",
                        task_id, prerequisite
                    )));
                }
            }
        }

        rec_stack.remove(task_id);
        Ok(())
    }
}

/// Service for resolving task execution order based on dependencies
/// 
/// Uses topological sorting to determine the correct order for task execution
/// while respecting all dependencies.
pub struct DependencyResolver;

impl DependencyResolver {
    /// Resolves the execution order for tasks in a DAG
    /// 
    /// # Arguments
    /// * `dag` - The DAG to resolve
    /// 
    /// # Returns
    /// * `DomainResult<Vec<TaskId>>` - Ordered list of task IDs for execution
    /// 
    /// # Errors
    /// * `DomainError::CircularDependency` - If DAG contains cycles
    #[instrument(skip(dag), fields(dag_id = %dag.id(), task_count = dag.task_count()))]
    pub fn resolve_execution_order(dag: &Dag) -> DomainResult<Vec<TaskId>> {
        debug!("Resolving task execution order");
        
        // First validate there are no cycles
        DagValidator::check_for_cycles(dag)?;

        // Build adjacency list and in-degree map
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        let mut adjacency: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        // Initialize all tasks with in-degree 0
        for task_id in dag.tasks().keys() {
            in_degree.insert(task_id.clone(), 0);
            adjacency.insert(task_id.clone(), Vec::new());
        }

        // Build the graph
        for dep in dag.dependencies() {
            // dep.task_id() depends on dep.depends_on()
            // So depends_on() -> task_id() edge
            adjacency
                .get_mut(dep.depends_on())
                .unwrap()
                .push(dep.task_id().clone());
            *in_degree.get_mut(dep.task_id()).unwrap() += 1;
        }

        // Kahn's algorithm for topological sort
        let mut queue: VecDeque<TaskId> = VecDeque::new();
        let mut result = Vec::new();

        // Start with tasks that have no dependencies
        for (task_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(task_id.clone());
            }
        }

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id.clone());

            // Reduce in-degree for dependent tasks
            if let Some(dependents) = adjacency.get(&task_id) {
                for dependent in dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        // If we didn't process all tasks, there's a cycle
        if result.len() != dag.tasks().len() {
            warn!("Circular dependency detected during resolution");
            return Err(DomainError::CircularDependency(
                "Circular dependency detected during resolution".to_string(),
            ));
        }

        info!(task_count = result.len(), "Execution order resolved successfully");
        Ok(result)
    }

    /// Gets all direct dependencies for a task
    /// 
    /// # Arguments
    /// * `task_id` - The task to get dependencies for
    /// * `dag` - The DAG containing the task
    /// 
    /// # Returns
    /// * `Vec<TaskId>` - List of task IDs that this task depends on
    pub fn get_dependencies(task_id: &TaskId, dag: &Dag) -> Vec<TaskId> {
        dag.dependencies()
            .iter()
            .filter(|dep| dep.task_id() == task_id)
            .map(|dep| dep.depends_on().clone())
            .collect()
    }

    /// Gets all tasks that depend on a given task
    /// 
    /// # Arguments
    /// * `task_id` - The task to get dependents for
    /// * `dag` - The DAG containing the task
    /// 
    /// # Returns
    /// * `Vec<TaskId>` - List of task IDs that depend on this task
    pub fn get_dependents(task_id: &TaskId, dag: &Dag) -> Vec<TaskId> {
        dag.dependencies()
            .iter()
            .filter(|dep| dep.depends_on() == task_id)
            .map(|dep| dep.task_id().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ExecutionStatus, TaskConfig};
    use crate::domain::value_objects::DagId;

    /// Helper to create a test task
    fn create_test_task(id: &str) -> Task {
        let task_id = TaskId::new(id).unwrap();
        let config = TaskConfig::Shell {
            command: "echo test".to_string(),
            working_dir: None,
        };
        Task::new(task_id, id.to_string(), "shell".to_string(), config)
    }

    // DagValidator Tests

    /// Test validation of empty DAG
    #[test]
    fn test_validator_empty_dag() {
        let dag_id = DagId::new("test_dag").unwrap();
        let dag = Dag::new(dag_id, "Test DAG".to_string());

        let errors = DagValidator::validate(&dag).unwrap();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("no tasks"));
    }

    /// Test validation of valid DAG
    #[test]
    fn test_validator_valid_dag() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();

        let errors = DagValidator::validate(&dag).unwrap();
        assert!(errors.is_empty());
    }

    /// Test validation detects circular dependencies
    #[test]
    fn test_validator_circular_dependency() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();

        // Create circular dependency
        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();
        // This should fail due to cycle detection
        let result = dag.add_dependency(task1_id, task2_id);
        assert!(result.is_err());
    }

    // DependencyResolver Tests

    /// Test resolving execution order for simple linear DAG
    #[test]
    fn test_resolver_linear_dag() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        let task3_id = TaskId::new("task3").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();
        dag.add_task(create_test_task("task3")).unwrap();

        // task2 depends on task1, task3 depends on task2
        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task3_id.clone(), task2_id.clone()).unwrap();

        let order = DependencyResolver::resolve_execution_order(&dag).unwrap();
        
        assert_eq!(order.len(), 3);
        // task1 must come before task2
        let pos1 = order.iter().position(|id| id == &task1_id).unwrap();
        let pos2 = order.iter().position(|id| id == &task2_id).unwrap();
        let pos3 = order.iter().position(|id| id == &task3_id).unwrap();
        
        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    /// Test resolving execution order for parallel tasks
    #[test]
    fn test_resolver_parallel_tasks() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        let task3_id = TaskId::new("task3").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();
        dag.add_task(create_test_task("task3")).unwrap();

        // task2 and task3 both depend on task1 (can run in parallel)
        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task3_id.clone(), task1_id.clone()).unwrap();

        let order = DependencyResolver::resolve_execution_order(&dag).unwrap();
        
        assert_eq!(order.len(), 3);
        // task1 must come first
        assert_eq!(order[0], task1_id);
        // task2 and task3 can be in any order after task1
        assert!(order[1..].contains(&task2_id));
        assert!(order[1..].contains(&task3_id));
    }

    /// Test resolving execution order for diamond DAG
    #[test]
    fn test_resolver_diamond_dag() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        let task3_id = TaskId::new("task3").unwrap();
        let task4_id = TaskId::new("task4").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();
        dag.add_task(create_test_task("task3")).unwrap();
        dag.add_task(create_test_task("task4")).unwrap();

        // Diamond: task1 -> task2, task3 -> task4
        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task3_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task4_id.clone(), task2_id.clone()).unwrap();
        dag.add_dependency(task4_id.clone(), task3_id.clone()).unwrap();

        let order = DependencyResolver::resolve_execution_order(&dag).unwrap();
        
        assert_eq!(order.len(), 4);
        // task1 must be first
        assert_eq!(order[0], task1_id);
        // task4 must be last
        assert_eq!(order[3], task4_id);
        // task2 and task3 must be in the middle
        assert!(order[1..3].contains(&task2_id));
        assert!(order[1..3].contains(&task3_id));
    }

    /// Test getting direct dependencies
    #[test]
    fn test_get_dependencies() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        let task3_id = TaskId::new("task3").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();
        dag.add_task(create_test_task("task3")).unwrap();

        dag.add_dependency(task3_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task3_id.clone(), task2_id.clone()).unwrap();

        let deps = DependencyResolver::get_dependencies(&task3_id, &dag);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&task1_id));
        assert!(deps.contains(&task2_id));
    }

    /// Test getting dependents
    #[test]
    fn test_get_dependents() {
        let dag_id = DagId::new("test_dag").unwrap();
        let mut dag = Dag::new(dag_id, "Test DAG".to_string());

        let task1_id = TaskId::new("task1").unwrap();
        let task2_id = TaskId::new("task2").unwrap();
        let task3_id = TaskId::new("task3").unwrap();

        dag.add_task(create_test_task("task1")).unwrap();
        dag.add_task(create_test_task("task2")).unwrap();
        dag.add_task(create_test_task("task3")).unwrap();

        dag.add_dependency(task2_id.clone(), task1_id.clone()).unwrap();
        dag.add_dependency(task3_id.clone(), task1_id.clone()).unwrap();

        let dependents = DependencyResolver::get_dependents(&task1_id, &dag);
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&task2_id));
        assert!(dependents.contains(&task3_id));
    }
}
