pub mod dag;
pub mod parsers;

pub use dag::*;

use anyhow::Result;

/// Parse a DAG file and return the DAG structure
pub async fn parse_dag_file(file_path: &str) -> Result<Dag> {
    let content = tokio::fs::read_to_string(file_path).await?;
    // Detect format based on content and choose an initial parser.
    // Prioritize Markdown-ish inputs containing headings, but prefer the
    // forgiving text parser when there's an explicit `tasks:` block because
    // AI-generated DAGs often mix prose + indented task configs.
    let mut dag = if content.contains("##") {
        if content.contains("tasks:") {
            parsers::parse_txt(&content)?
        } else {
            parsers::parse_md(&content)?
        }
    } else if content.contains("tasks:") && content.contains("- ") {
        // Prefer the permissive text parser when there's an explicit tasks list
        // with dash-prefixed items. Some DAGs (and AI-generated content)
        // start with `dag:` but use a plain text/list style rather than
        // strict YAML mapping, so prefer parse_txt in that case.
        parsers::parse_txt(&content)?
    } else if content.trim().starts_with("dag:") {
        parsers::parse_yaml(&content)?
    } else if content.contains("script: |") {
        parsers::parse_yaml(&content)?
    } else {
        parsers::parse_txt(&content)?
    };

    // If name is missing, try to extract a human-friendly title from the
    // first non-empty non-comment line (AI-generated DAGs often start with
    // a plain English title rather than a `dag:` field).
    if dag.name.is_empty() {
        if let Some(title) = extract_title_from_content(&content) {
            dag.name = title;
        }
    }

    // If the initially chosen parser produced no tasks, try alternative
    // parsers as a fallback and pick the first one that yields tasks.
    if dag.tasks.is_empty() {
        let parsers_to_try: [&dyn Fn(&str) -> Result<Dag>; 3] = [&parsers::parse_txt, &parsers::parse_md, &parsers::parse_yaml];
        for p in parsers_to_try.iter() {
            if let Ok(candidate) = p(&content) {
                if !candidate.tasks.is_empty() {
                    // preserve any already-extracted name if the candidate has none
                    let mut merged = candidate;
                    if merged.name.is_empty() {
                        merged.name = dag.name.clone();
                    }
                    dag = merged;
                    break;
                }
            }
        }
    }

    Ok(dag)
}

fn extract_title_from_content(content: &str) -> Option<String> {
    for line in content.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        // Skip obvious section headers that are not a title
        let lower = s.to_lowercase();
        if lower.starts_with("##") || lower.starts_with("tasks:") || lower.starts_with("this dag") || lower.starts_with("dag:") {
            continue;
        }
        // Use the first reasonably sized line as the title
        if s.len() > 3 {
            return Some(s.to_string());
        }
    }
    None
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
            errors.push(format!("Invalid schedule expression: {schedule}"));
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
        if !visited.contains(task_name)
            && dfs(task_name, &graph, &mut visited, &mut rec_stack) {
                return true;
            }
    }

    false
}

fn is_valid_schedule(schedule: &str) -> bool {
    use regex::Regex;

    let schedule_regex = Regex::new(r"^every \d+ (minute|minutes|hour|hours|day|days)$").unwrap();
    schedule_regex.is_match(&schedule.to_lowercase())
}
