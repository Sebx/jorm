pub mod dag;
pub mod parsers;

pub use dag::*;

use anyhow::Result;

/// Parse a DAG file and return the DAG structure
pub async fn parse_dag_file(file_path: &str) -> Result<Dag> {
    let content = tokio::fs::read_to_string(file_path).await?;

    // Detect format based on content
    if content.trim().starts_with("dag:") || content.contains("script: |") {
        // YAML format
        parsers::parse_yaml(&content)
    } else if content.contains("tasks:") && content.contains("- ") {
        // Markdown format
        parsers::parse_md(&content)
    } else {
        // Text format (default)
        parsers::parse_txt(&content)
    }
}

/// Validate a DAG structure
pub fn validate_dag(dag: &Dag) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    // Check DAG name
    if dag.name.is_empty() {
        errors.push("DAG name is missing".to_string());
    }

    // Check tasks exist
    if dag.tasks.is_empty() {
        errors.push("No tasks defined".to_string());
    }

    // Check for duplicate tasks
    let mut task_names: Vec<_> = dag.tasks.keys().collect();
    task_names.sort();
    for i in 1..task_names.len() {
        if task_names[i] == task_names[i - 1] {
            errors.push(format!("Duplicate task found: {}", task_names[i]));
        }
    }

    // Validate dependencies
    for dep in &dag.dependencies {
        if !dag.tasks.contains_key(&dep.task) {
            errors.push(format!("Dependent task not defined: {}", dep.task));
        }
        if !dag.tasks.contains_key(&dep.depends_on) {
            errors.push(format!("Base task not defined: {}", dep.depends_on));
        }
    }

    // Check for cycles
    if has_cycle(dag) {
        errors.push("Cyclic dependencies detected".to_string());
    }

    // Validate schedule expression
    if let Some(schedule) = &dag.schedule {
        if !is_valid_schedule(schedule) {
            errors.push(format!("Invalid schedule expression: {}", schedule));
        }
    }

    Ok(errors)
}

fn has_cycle(dag: &Dag) -> bool {
    use std::collections::{HashMap, HashSet};

    // Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for task_name in dag.tasks.keys() {
        graph.insert(task_name.clone(), Vec::new());
    }

    for dep in &dag.dependencies {
        graph
            .get_mut(&dep.depends_on)
            .unwrap()
            .push(dep.task.clone());
    }

    // DFS cycle detection
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if dfs(neighbor, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    for task_name in dag.tasks.keys() {
        if !visited.contains(task_name) {
            if dfs(task_name, &graph, &mut visited, &mut rec_stack) {
                return true;
            }
        }
    }

    false
}

fn is_valid_schedule(schedule: &str) -> bool {
    use regex::Regex;

    let schedule_regex = Regex::new(r"^every \d+ (minute|minutes|hour|hours|day|days)$").unwrap();
    schedule_regex.is_match(&schedule.to_lowercase())
}
