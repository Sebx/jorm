use super::{Dag, Task, TaskConfig};
use anyhow::{Context, Result};
use regex::Regex;
use serde_yaml;
use std::collections::HashMap;

/// Parse a text format DAG
pub fn parse_txt(content: &str) -> Result<Dag> {
    let mut dag = Dag::new(String::new());
    let mut current_section = None;
    let mut current_task: Option<Task> = None;
    let mut task_script = String::new();
    let mut in_script = false;

    for line in content.lines() {
        let raw = line; // preserve original line for script blocks
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            // Preserve blank lines inside script blocks
            if in_script {
                task_script.push_str("\n");
            }
            continue;
        }

        if line.starts_with("dag:") {
            dag.name = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("schedule:") {
            dag.schedule = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
        } else if line == "tasks:" {
            current_section = Some("tasks");
        } else if line == "dependencies:" {
            current_section = Some("dependencies");
        } else if line.starts_with("- ") {
            if let Some(stripped) = line.strip_prefix("- ") {
                let item = stripped.trim();

                match current_section {
                    Some("tasks") => {
                        // Save previous task if exists
                        if let Some(mut task) = current_task.take() {
                            // Assign script to previous task before saving
                            if !task_script.is_empty() {
                                // Trim trailing whitespace and dedent so Python
                                // scripts don't have unexpected top-level indentation.
                                let script = dedent(task_script.trim_end());
                                task.config.script = Some(script);
                            }
                            dag.add_task(task);
                        }

                        // Start new task
                        let task = Task::new(item.to_string());
                        current_task = Some(task);
                        in_script = false;
                        task_script.clear();
                    }
                    Some("dependencies") => {
                        if let Some((task, depends_on)) = parse_dependency_txt(item) {
                            dag.add_dependency(task, depends_on);
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(ref mut task) = current_task {
            // Parse task configuration
            if line.starts_with("type:") {
                let task_type = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.task_type = Some(task_type);
            } else if line.starts_with("description:") {
                let description = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.description = Some(description);
            } else if line.starts_with("module:") {
                let module = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.module = Some(module);
            } else if line.starts_with("function:") {
                let function = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.function = Some(function);
            } else if line.starts_with("command:") {
                let command = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.command = Some(command);
            } else if line.starts_with("destination:") {
                let destination = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.destination = Some(destination);
            } else if line.starts_with("source:") {
                let source = line.split(':').nth(1).unwrap_or("").trim().to_string();
                task.config.source = Some(source);
            } else if line.starts_with("script:") {
                in_script = true;
                task_script.clear();
            } else if in_script {
                // Continue collecting script content
                // Preserve the original line including leading whitespace so
                // Python indentation is kept intact.
                task_script.push_str(raw);
                task_script.push('\n');
            } else if !line.starts_with("  ") && !line.starts_with("\t") && !line.trim().is_empty()
            {
                // If we're not in a script and the line doesn't start with whitespace,
                // and it's not empty, we're probably starting a new task or section
                in_script = false;
            }
        }
    }

    // Save the last task if exists
    if let Some(mut task) = current_task.take() {
        if !task_script.is_empty() {
            // Trim trailing whitespace and dedent script block so Python
            // scripts don't have unexpected top-level indentation.
            let script = dedent(task_script.trim_end());
            task.config.script = Some(script);
        }
        dag.add_task(task);
    }

    Ok(dag)
}

/// Parse a markdown format DAG
pub fn parse_md(content: &str) -> Result<Dag> {
    let mut dag = Dag::new(String::new());
    let mut current_section = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.to_lowercase().starts_with("# dag:") {
            dag.name = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.to_lowercase().starts_with("**schedule:**") {
            let schedule_part = line.split("**schedule:**").nth(1).unwrap_or("").trim();
            dag.schedule = Some(schedule_part.trim_matches('*').trim().to_string());
        } else if line.to_lowercase().starts_with("## tasks") {
            current_section = Some("tasks");
        } else if line.to_lowercase().starts_with("## dependencies") {
            current_section = Some("dependencies");
        } else if line.starts_with("- ") {
            if let Some(stripped) = line.strip_prefix("- ") {
                let item = stripped.trim();
                // Remove markdown formatting (backticks, asterisks)
                let clean_item = item.replace(['`', '*'], "");

                match current_section {
                    Some("tasks") => {
                        let task = Task::new(clean_item);
                        dag.add_task(task);
                    }
                    Some("dependencies") => {
                        if let Some((task, depends_on)) = parse_dependency_md(&clean_item) {
                            dag.add_dependency(task, depends_on);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(dag)
}

/// Parse a YAML format DAG
pub fn parse_yaml(content: &str) -> Result<Dag> {
    let yaml_data: serde_yaml::Value =
        serde_yaml::from_str(content).context("Failed to parse YAML content")?;

    let mut dag = Dag::new(String::new());

    // Extract DAG name
    if let Some(name) = yaml_data.get("name").and_then(|v| v.as_str()) {
        dag.name = name.to_string();
    } else if let Some(name) = yaml_data.get("dag").and_then(|v| v.as_str()) {
        dag.name = name.to_string();
    }

    // Extract schedule
    if let Some(schedule) = yaml_data.get("schedule").and_then(|v| v.as_str()) {
        dag.schedule = Some(schedule.to_string());
    }

    // Extract tasks
    if let Some(tasks_data) = yaml_data.get("tasks").and_then(|v| v.as_mapping()) {
        for (task_name, task_config) in tasks_data {
            if let Some(name) = task_name.as_str() {
                let mut task = Task::new(name.to_string());

                // Parse task configuration
                if let Some(config_map) = task_config.as_mapping() {
                    task.config = parse_task_config_yaml(config_map)?;

                    // Extract dependencies from depends_on field
                    if let Some(depends_on) =
                        config_map.get(serde_yaml::Value::String("depends_on".to_string()))
                    {
                        match depends_on {
                            serde_yaml::Value::String(dep) => {
                                dag.add_dependency(name.to_string(), dep.clone());
                            }
                            serde_yaml::Value::Sequence(deps) => {
                                for dep in deps {
                                    if let Some(dep_str) = dep.as_str() {
                                        dag.add_dependency(name.to_string(), dep_str.to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                dag.add_task(task);
            }
        }
    }

    // Extract dependencies from root-level dependencies section
    if let Some(dependencies_data) = yaml_data.get("dependencies").and_then(|v| v.as_sequence()) {
        for dep_item in dependencies_data {
            if let Some(dep_str) = dep_item.as_str() {
                if let Some((task, depends_on)) = parse_dependency_txt(dep_str) {
                    dag.add_dependency(task, depends_on);
                }
            }
        }
    }

    Ok(dag)
}

fn parse_dependency_txt(item: &str) -> Option<(String, String)> {
    let re = Regex::new(r"(.+) after (.+)").ok()?;
    let caps = re.captures(item)?;
    Some((
        caps.get(1)?.as_str().trim().to_string(),
        caps.get(2)?.as_str().trim().to_string(),
    ))
}

fn dedent(s: &str) -> String {
    // Find minimal leading indent (spaces) across non-empty lines
    let mut min_indent: Option<usize> = None;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let leading = line.chars().take_while(|c| *c == ' ').count();
        min_indent = match min_indent {
            Some(m) => Some(std::cmp::min(m, leading)),
            None => Some(leading),
        };
    }

    let indent = min_indent.unwrap_or(0);
    if indent == 0 {
        return s.to_string();
    }

    let mut out = String::new();
    for line in s.lines() {
        if line.len() >= indent {
            out.push_str(&line[indent..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // Remove final added newline if original didn't end with one
    if !s.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn parse_dependency_md(item: &str) -> Option<(String, String)> {
    let re = Regex::new(r"(.+) runs after (.+)").ok()?;
    let caps = re.captures(item)?;
    Some((
        caps.get(1)?.as_str().trim().to_string(),
        caps.get(2)?.as_str().trim().to_string(),
    ))
}

fn parse_task_config_yaml(config_map: &serde_yaml::Mapping) -> Result<TaskConfig> {
    let mut config = TaskConfig::default();

    for (key, value) in config_map {
        if let Some(key_str) = key.as_str() {
            match key_str {
                "type" => config.task_type = value.as_str().map(|s| s.to_string()),
                "command" => config.command = value.as_str().map(|s| s.to_string()),
                "module" => config.module = value.as_str().map(|s| s.to_string()),
                "function" => config.function = value.as_str().map(|s| s.to_string()),
                "args" => {
                    if let Ok(json_value) = serde_yaml::to_value(value) {
                        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&json_value).unwrap_or_default(),
                        ) {
                            config.args = Some(arr);
                        }
                    }
                }
                "kwargs" => {
                    if let Ok(json_value) = serde_yaml::to_value(value) {
                        if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&json_value).unwrap_or_default(),
                        ) {
                            config.kwargs = Some(obj.into_iter().collect());
                        }
                    }
                }
                "method" => config.method = value.as_str().map(|s| s.to_string()),
                "url" => config.url = value.as_str().map(|s| s.to_string()),
                "headers" => {
                    if let Some(headers_map) = value.as_mapping() {
                        let mut headers = HashMap::new();
                        for (k, v) in headers_map {
                            if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                                headers.insert(key.to_string(), val.to_string());
                            }
                        }
                        config.headers = Some(headers);
                    }
                }
                "data" => {
                    if let Ok(json_value) = serde_yaml::to_value(value) {
                        if let Ok(serde_json_value) = serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&json_value).unwrap_or_default(),
                        ) {
                            config.data = Some(serde_json_value);
                        }
                    }
                }
                "script" => config.script = value.as_str().map(|s| s.to_string()),
                "operation" => config.operation = value.as_str().map(|s| s.to_string()),
                "source" => config.source = value.as_str().map(|s| s.to_string()),
                "dest" => config.dest = value.as_str().map(|s| s.to_string()),
                "destination" => config.destination = value.as_str().map(|s| s.to_string()),
                "target" => config.target = value.as_str().map(|s| s.to_string()),
                "timeout" => config.timeout = value.as_u64(),
                "retry_count" => config.retry_count = value.as_u64().map(|v| v as u32),
                "backoff_strategy" => {
                    config.backoff_strategy = value.as_str().map(|s| s.to_string())
                }
                "depends_on" => {
                    // This is handled in the main parsing function
                }
                _ => {
                    // Ignore unknown fields
                }
            }
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_txt() {
        let content = r#"
dag: test_dag
schedule: every 10 minutes

tasks:
  - task1
  - task2
  - task3

dependencies:
  - task2 after task1
  - task3 after task2
"#;

        let dag = parse_txt(content).unwrap();
        assert_eq!(dag.name, "test_dag");
        assert_eq!(dag.schedule, Some("every 10 minutes".to_string()));
        assert_eq!(dag.tasks.len(), 3);
        assert_eq!(dag.dependencies.len(), 2);
    }

    #[test]
    fn test_parse_yaml() {
        let content = r#"
dag: test_dag
schedule: "every 1 hour"

tasks:
  task1:
    type: shell
    command: "echo hello"
  task2:
    type: python
    module: "math"
    function: "sqrt"
    args: [16]
    depends_on: [task1]
"#;

        let dag = parse_yaml(content).unwrap();
        assert_eq!(dag.name, "test_dag");
        assert_eq!(dag.tasks.len(), 2);
        assert_eq!(dag.dependencies.len(), 1);

        let task1 = dag.tasks.get("task1").unwrap();
        assert_eq!(task1.config.task_type, Some("shell".to_string()));
        assert_eq!(task1.config.command, Some("echo hello".to_string()));
    }
}
