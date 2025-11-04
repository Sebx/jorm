use crate::core::error::JormError;
use crate::parser::dag_parser::DagParser;
use regex::Regex;
use std::collections::HashMap;

pub struct NlpProcessor {
    // Pattern matchers for different task types
    shell_patterns: Vec<Regex>,
    file_patterns: Vec<Regex>,
    http_patterns: Vec<Regex>,
    python_patterns: Vec<Regex>,
    rust_patterns: Vec<Regex>,
    jorm_patterns: Vec<Regex>,
    dependency_patterns: Vec<Regex>,
}

#[derive(Debug, Clone)]
struct ParsedTask {
    name: String,
    task_type: String,
    parameters: HashMap<String, String>,
    dependencies: Vec<String>,
}

impl NlpProcessor {
    pub async fn new() -> Result<Self, JormError> {
        let shell_patterns = vec![
            Regex::new(r#"(?i)run\s+(?:command\s+)?['"]([^'"]+)['"]"#).unwrap(),
            Regex::new(r#"(?i)execute\s+['"]([^'"]+)['"]"#).unwrap(),
            Regex::new(r#"(?i)(?:shell|command|cmd):\s*['"]([^'"]+)['"]"#).unwrap(),
            Regex::new(r"(?i)(?:bash|shell)\s+task").unwrap(),
            Regex::new(r"(?i)print\s+(?:the\s+)?(?:number|task)").unwrap(),
            Regex::new(r"(?i)echo\s+").unwrap(),
            Regex::new(r"(?i)(\d+)\s+(?:bash|shell)\s+task").unwrap(),
        ];

        let file_patterns = vec![
            Regex::new(r"(?i)copy\s+([^\s]+)\s+to\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)move\s+([^\s]+)\s+to\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)delete\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)remove\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)copy\s+files").unwrap(),
            Regex::new(r"(?i)backup\s+files").unwrap(),
            Regex::new(r"(?i)backup\s+results").unwrap(),
            Regex::new(r"(?i)copy\s+config\s+files").unwrap(),
        ];

        let http_patterns = vec![
            Regex::new(r"(?i)(?:send|make)\s+(?:a\s+)?(?:get|post|put|delete)\s+(?:request\s+)?to\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)(?:call|request)\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)webhook\s+to\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)send\s+notification").unwrap(),
            Regex::new(r"(?i)notify").unwrap(),
        ];

        let python_patterns = vec![
            Regex::new(r"(?i)run\s+python\s+(?:script\s+)?([^\s]+)").unwrap(),
            Regex::new(r"(?i)execute\s+([^\s]+\.py)").unwrap(),
            Regex::new(r"(?i)python\s+([^\s]+)").unwrap(),
        ];

        let rust_patterns = vec![
            Regex::new(r"(?i)(?:cargo\s+)?build\s+(?:the\s+)?(?:rust\s+)?project").unwrap(),
            Regex::new(r"(?i)(?:cargo\s+)?test\s+(?:the\s+)?(?:rust\s+)?project").unwrap(),
            Regex::new(r"(?i)(?:cargo\s+)?run\s+(?:the\s+)?(?:rust\s+)?project").unwrap(),
            Regex::new(r"(?i)compile\s+(?:the\s+)?(?:rust\s+)?project").unwrap(),
            Regex::new(r"(?i)run\s+tests").unwrap(),
            Regex::new(r"(?i)execute\s+tests").unwrap(),
        ];

        let jorm_patterns = vec![
            Regex::new(r"(?i)jorm\s+execute\s+([^\s]+)").unwrap(),
            Regex::new(r"(?i)jorm\s+generate\s+").unwrap(),
            Regex::new(r"(?i)jorm\s+server").unwrap(),
            Regex::new(r"(?i)jorm\s+schedule").unwrap(),
            Regex::new(r"(?i)run\s+(?:existing\s+)?(?:jorm\s+)?(?:work)?flow").unwrap(),
            Regex::new(r"(?i)execute\s+(?:existing\s+)?(?:jorm\s+)?(?:work)?flow").unwrap(),
            Regex::new(r"(?i)create\s+(?:new\s+)?dag").unwrap(),
            Regex::new(r"(?i)generate\s+(?:new\s+)?dag").unwrap(),
            Regex::new(r"(?i)start\s+(?:jorm\s+)?server").unwrap(),
        ];

        let dependency_patterns = vec![
            Regex::new(r"(?i)(?:then|after|afterwards)\s+").unwrap(),
            Regex::new(r"(?i)(?:first|initially)\s+").unwrap(),
            Regex::new(r"(?i)(?:finally|lastly)\s+").unwrap(),
        ];

        Ok(Self {
            shell_patterns,
            file_patterns,
            http_patterns,
            python_patterns,
            rust_patterns,
            jorm_patterns,
            dependency_patterns,
        })
    }

    pub async fn generate_dag(&self, description: &str) -> Result<String, JormError> {
        let tasks = self.parse_natural_language(description)?;
        let dag_content = self.generate_dag_syntax(tasks)?;

        // Validate the generated DAG
        self.validate_generated_dag(&dag_content)?;

        Ok(dag_content)
    }

    fn parse_natural_language(&self, description: &str) -> Result<Vec<ParsedTask>, JormError> {
        let mut tasks = Vec::new();

        // Check for patterns that indicate multiple similar tasks (like "10 bash tasks")
        if let Some(multiple_tasks) = self.parse_multiple_tasks(description)? {
            tasks.extend(multiple_tasks);
        } else {
            // Parse as individual sentences
            let sentences = self.split_into_sentences(description);

            for (index, sentence) in sentences.iter().enumerate() {
                if let Some(task) = self.parse_sentence(sentence, index)? {
                    tasks.push(task);
                }
            }

            // Add dependencies based on order and keywords for sentence-based parsing
            if !tasks.is_empty() {
                self.infer_dependencies(&mut tasks, &sentences);
            }
        }

        if tasks.is_empty() {
            return Err(JormError::NlpError(
                "Could not parse any tasks from the description".to_string(),
            ));
        }

        Ok(tasks)
    }

    fn parse_multiple_tasks(
        &self,
        description: &str,
    ) -> Result<Option<Vec<ParsedTask>>, JormError> {
        // Pattern for "N bash/shell tasks that do X"
        let multiple_pattern =
            Regex::new(r"(?i)(\d+)\s+(?:bash|shell)\s+task(?:s)?\s+that\s+(.+)").unwrap();

        if let Some(captures) = multiple_pattern.captures(description) {
            let count: usize = captures.get(1).unwrap().as_str().parse().unwrap_or(1);
            let action = captures.get(2).unwrap().as_str();

            let mut tasks = Vec::new();

            for i in 1..=count {
                let task_name = format!("task_{}", i);
                let mut parameters = HashMap::new();

                // Generate command based on the action
                let command = if action.contains("print") && action.contains("number") {
                    format!("echo 'Task number {}'", i)
                } else if action.contains("print") && action.contains("task") {
                    format!("echo 'This is task {}'", i)
                } else {
                    format!("echo '{} - Task {}'", action, i)
                };

                parameters.insert("command".to_string(), command);

                tasks.push(ParsedTask {
                    name: task_name,
                    task_type: "shell".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                });
            }

            return Ok(Some(tasks));
        }

        // Pattern for "simple workflow with N tasks"
        let workflow_pattern =
            Regex::new(r"(?i)(?:simple\s+)?workflow\s+with\s+(\d+)\s+(?:bash|shell)\s+task")
                .unwrap();

        if let Some(captures) = workflow_pattern.captures(description) {
            let count: usize = captures.get(1).unwrap().as_str().parse().unwrap_or(1);

            let mut tasks = Vec::new();

            for i in 1..=count {
                let task_name = format!("task_{}", i);
                let mut parameters = HashMap::new();

                let command = if description.contains("print") && description.contains("number") {
                    format!("echo 'Task number {}'", i)
                } else {
                    format!("echo 'Executing task {}'", i)
                };

                parameters.insert("command".to_string(), command);

                tasks.push(ParsedTask {
                    name: task_name,
                    task_type: "shell".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                });
            }

            return Ok(Some(tasks));
        }

        Ok(None)
    }

    fn split_into_sentences(&self, description: &str) -> Vec<String> {
        // Split on sentence-ending punctuation and conjunctions, but not on dots in filenames
        let sentence_regex =
            Regex::new(r"[,;]\s*|\s+(?:then|and|after|afterwards)\s+|\n+").unwrap();
        let sentences: Vec<String> = sentence_regex
            .split(description)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // If no sentence separators found, treat the whole description as one sentence
        if sentences.len() <= 1 {
            vec![description.trim().to_string()]
        } else {
            sentences
        }
    }

    fn parse_sentence(
        &self,
        sentence: &str,
        index: usize,
    ) -> Result<Option<ParsedTask>, JormError> {
        let task_name = format!("task_{}", index + 1);

        // Try to match shell commands
        for pattern in &self.shell_patterns {
            if let Some(captures) = pattern.captures(sentence) {
                let command = captures.get(1).unwrap().as_str();
                let mut parameters = HashMap::new();
                parameters.insert("command".to_string(), command.to_string());

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "shell".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            }
        }

        // Try to match file operations
        for pattern in &self.file_patterns {
            if pattern.is_match(sentence) {
                let mut parameters = HashMap::new();

                if sentence.to_lowercase().contains("copy config files") {
                    // Copy config files task
                    parameters.insert("source".to_string(), "config.template".to_string());
                    parameters.insert("destination".to_string(), "config.prod".to_string());
                    return Ok(Some(ParsedTask {
                        name: task_name,
                        task_type: "file_copy".to_string(),
                        parameters,
                        dependencies: Vec::new(),
                    }));
                } else if sentence.to_lowercase().contains("backup results") {
                    // Backup results task
                    parameters.insert("source".to_string(), "./results/".to_string());
                    parameters.insert("destination".to_string(), "./backup/results/".to_string());
                    return Ok(Some(ParsedTask {
                        name: task_name,
                        task_type: "file_copy".to_string(),
                        parameters,
                        dependencies: Vec::new(),
                    }));
                } else if sentence.to_lowercase().contains("backup files") {
                    // Generic backup files task
                    parameters.insert("source".to_string(), "./data/".to_string());
                    parameters.insert("destination".to_string(), "./backup/".to_string());
                    return Ok(Some(ParsedTask {
                        name: task_name,
                        task_type: "file_copy".to_string(),
                        parameters,
                        dependencies: Vec::new(),
                    }));
                } else if sentence.to_lowercase().contains("copy files") {
                    // Generic copy files task
                    parameters.insert("source".to_string(), "*.txt".to_string());
                    parameters.insert("destination".to_string(), "./backup/".to_string());
                    return Ok(Some(ParsedTask {
                        name: task_name,
                        task_type: "file_copy".to_string(),
                        parameters,
                        dependencies: Vec::new(),
                    }));
                } else if let Some(captures) = pattern.captures(sentence) {
                    // Handle patterns with capture groups
                    if sentence.to_lowercase().contains("copy") && captures.len() >= 3 {
                        parameters.insert(
                            "source".to_string(),
                            captures.get(1).unwrap().as_str().to_string(),
                        );
                        parameters.insert(
                            "destination".to_string(),
                            captures.get(2).unwrap().as_str().to_string(),
                        );
                        return Ok(Some(ParsedTask {
                            name: task_name,
                            task_type: "file_copy".to_string(),
                            parameters,
                            dependencies: Vec::new(),
                        }));
                    } else if sentence.to_lowercase().contains("move") && captures.len() >= 3 {
                        parameters.insert(
                            "source".to_string(),
                            captures.get(1).unwrap().as_str().to_string(),
                        );
                        parameters.insert(
                            "destination".to_string(),
                            captures.get(2).unwrap().as_str().to_string(),
                        );
                        return Ok(Some(ParsedTask {
                            name: task_name,
                            task_type: "file_move".to_string(),
                            parameters,
                            dependencies: Vec::new(),
                        }));
                    } else if (sentence.to_lowercase().contains("delete")
                        || sentence.to_lowercase().contains("remove"))
                        && captures.len() >= 2
                    {
                        parameters.insert(
                            "path".to_string(),
                            captures.get(1).unwrap().as_str().to_string(),
                        );
                        return Ok(Some(ParsedTask {
                            name: task_name,
                            task_type: "file_delete".to_string(),
                            parameters,
                            dependencies: Vec::new(),
                        }));
                    }
                }
            }
        }

        // Try to match HTTP requests
        for pattern in &self.http_patterns {
            if let Some(captures) = pattern.captures(sentence) {
                let mut parameters = HashMap::new();

                if captures.len() >= 2 {
                    let url = captures.get(1).unwrap().as_str();
                    parameters.insert("url".to_string(), url.to_string());
                } else if sentence.to_lowercase().contains("notification")
                    || sentence.to_lowercase().contains("notify")
                {
                    // Generic notification
                    parameters.insert(
                        "url".to_string(),
                        "https://hooks.slack.com/webhook".to_string(),
                    );
                    parameters.insert(
                        "body".to_string(),
                        r#"{"text": "Task completed successfully"}"#.to_string(),
                    );
                }

                // Determine HTTP method from context
                let method = if sentence.to_lowercase().contains("post") {
                    "POST"
                } else if sentence.to_lowercase().contains("put") {
                    "PUT"
                } else if sentence.to_lowercase().contains("delete") {
                    "DELETE"
                } else if sentence.to_lowercase().contains("notification")
                    || sentence.to_lowercase().contains("notify")
                {
                    "POST"
                } else {
                    "GET"
                };
                parameters.insert("method".to_string(), method.to_string());

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "http".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            }
        }

        // Try to match Python scripts
        for pattern in &self.python_patterns {
            if let Some(captures) = pattern.captures(sentence) {
                let script = captures.get(1).unwrap().as_str();
                let mut parameters = HashMap::new();
                parameters.insert("script".to_string(), script.to_string());

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "python".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            }
        }

        // Try to match Rust/Cargo commands
        for pattern in &self.rust_patterns {
            if pattern.is_match(sentence) {
                let mut parameters = HashMap::new();

                let command = if sentence.to_lowercase().contains("build") {
                    "cargo build"
                } else if sentence.to_lowercase().contains("test")
                    || sentence.to_lowercase().contains("run tests")
                {
                    "cargo test"
                } else if sentence.to_lowercase().contains("run")
                    && !sentence.to_lowercase().contains("tests")
                {
                    "cargo run"
                } else {
                    "cargo test"
                };

                parameters.insert("command".to_string(), command.to_string());

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "rust".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            }
        }

        // Try to match Jorm commands
        for pattern in &self.jorm_patterns {
            if let Some(captures) = pattern.captures(sentence) {
                let mut parameters = HashMap::new();

                if sentence.to_lowercase().contains("execute") && captures.len() >= 2 {
                    // jorm execute <dag_file>
                    let dag_file = captures.get(1).unwrap().as_str();
                    parameters.insert("command".to_string(), "execute".to_string());
                    parameters.insert("dag_file".to_string(), dag_file.to_string());
                } else if sentence.to_lowercase().contains("create")
                    || sentence.to_lowercase().contains("generate")
                {
                    // jorm generate
                    parameters.insert("command".to_string(), "generate".to_string());
                    parameters.insert(
                        "description".to_string(),
                        "Build and test workflow".to_string(),
                    );
                } else if sentence.to_lowercase().contains("server") {
                    // jorm server
                    parameters.insert("command".to_string(), "server".to_string());
                } else if sentence.to_lowercase().contains("schedule") {
                    // jorm schedule
                    parameters.insert("command".to_string(), "schedule".to_string());
                } else if sentence.to_lowercase().contains("run")
                    || sentence.to_lowercase().contains("execute")
                {
                    // Run existing workflow
                    parameters.insert("command".to_string(), "execute".to_string());
                    parameters.insert("dag_file".to_string(), "existing_workflow.txt".to_string());
                } else {
                    // Default to execute
                    parameters.insert("command".to_string(), "execute".to_string());
                    parameters.insert("dag_file".to_string(), "workflow.txt".to_string());
                }

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "jorm".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            } else if pattern.is_match(sentence) {
                // Handle patterns without captures
                let mut parameters = HashMap::new();

                if sentence.to_lowercase().contains("create")
                    || sentence.to_lowercase().contains("generate")
                {
                    parameters.insert("command".to_string(), "generate".to_string());
                    parameters.insert("description".to_string(), "Generated workflow".to_string());
                } else if sentence.to_lowercase().contains("server") {
                    parameters.insert("command".to_string(), "server".to_string());
                } else if sentence.to_lowercase().contains("run")
                    || sentence.to_lowercase().contains("execute")
                {
                    parameters.insert("command".to_string(), "execute".to_string());
                    parameters.insert("dag_file".to_string(), "existing_workflow.txt".to_string());
                } else {
                    parameters.insert("command".to_string(), "execute".to_string());
                    parameters.insert("dag_file".to_string(), "workflow.txt".to_string());
                }

                return Ok(Some(ParsedTask {
                    name: task_name,
                    task_type: "jorm".to_string(),
                    parameters,
                    dependencies: Vec::new(),
                }));
            }
        }

        Ok(None)
    }

    fn infer_dependencies(&self, tasks: &mut [ParsedTask], sentences: &[String]) {
        // If we have multiple tasks from a single description, check if they should be sequential
        if tasks.len() > 1 && sentences.len() == 1 {
            let description = &sentences[0].to_lowercase();

            // For simple workflows, make tasks sequential by default
            if description.contains("workflow") || description.contains("sequential") {
                for i in 1..tasks.len() {
                    let prev_name = tasks[i - 1].name.clone();
                    tasks[i].dependencies.push(prev_name);
                }
                return;
            }

            // For parallel tasks (like "10 bash tasks"), don't add dependencies
            if description.contains("parallel") || description.contains("independent") {
                return;
            }

            // Default for multiple similar tasks: make them sequential
            for i in 1..tasks.len() {
                let prev_name = tasks[i - 1].name.clone();
                tasks[i].dependencies.push(prev_name);
            }
            return;
        }

        // Original logic for multiple sentences
        for (i, sentence) in sentences.iter().enumerate() {
            if i > 0 && i < tasks.len() {
                // Check for dependency keywords
                let mut has_explicit_dependency = false;
                for pattern in &self.dependency_patterns {
                    if pattern.is_match(sentence) {
                        if sentence.to_lowercase().contains("then")
                            || sentence.to_lowercase().contains("after")
                        {
                            // Current task depends on previous task
                            if i > 0 {
                                let prev_name = tasks[i - 1].name.clone();
                                tasks[i].dependencies.push(prev_name);
                                has_explicit_dependency = true;
                            }
                        }
                        break;
                    }
                }

                // Default: if no explicit dependency keywords, assume sequential execution
                if !has_explicit_dependency && tasks[i].dependencies.is_empty() && i > 0 {
                    let prev_name = tasks[i - 1].name.clone();
                    tasks[i].dependencies.push(prev_name);
                }
            }
        }
    }

    fn generate_dag_syntax(&self, tasks: Vec<ParsedTask>) -> Result<String, JormError> {
        let mut dag_content = String::new();
        dag_content.push_str("# Generated DAG from natural language\n\n");

        for task in tasks {
            dag_content.push_str(&format!("task {} {{\n", task.name));
            dag_content.push_str(&format!("    type: {}\n", task.task_type));

            // Handle special parameters for jorm tasks
            if task.task_type == "jorm" {
                if let Some(command) = task.parameters.get("command") {
                    dag_content.push_str(&format!("    command: \"{}\"\n", command));
                }
                if let Some(dag_file) = task.parameters.get("dag_file") {
                    dag_content.push_str(&format!("    args: [\"{}\"]\n", dag_file));
                } else if let Some(description) = task.parameters.get("description") {
                    dag_content.push_str(&format!(
                        "    args: [\"--description\", \"{}\"]\n",
                        description
                    ));
                }
            } else {
                // Handle regular parameters
                for (key, value) in &task.parameters {
                    dag_content.push_str(&format!("    {}: \"{}\"\n", key, value));
                }
            }

            if !task.dependencies.is_empty() {
                dag_content.push_str("    depends_on: [");
                dag_content.push_str(&task.dependencies.join(", "));
                dag_content.push_str("]\n");
            }

            dag_content.push_str("}\n\n");
        }

        Ok(dag_content)
    }

    pub fn validate_generated_dag(&self, dag_content: &str) -> Result<(), JormError> {
        // Use the existing DAG parser to validate the generated content
        let parser = DagParser::new();
        match parser.parse_content(dag_content) {
            Ok(_) => Ok(()),
            Err(e) => Err(JormError::NlpError(format!(
                "Generated DAG is invalid: {}",
                e
            ))),
        }
    }

    pub async fn generate_dag_with_preview(
        &self,
        description: &str,
    ) -> Result<DagPreview, JormError> {
        let dag_content = self.generate_dag(description).await?;
        let parser = DagParser::new();
        let dag = parser.parse_content(&dag_content)?;

        Ok(DagPreview {
            original_description: description.to_string(),
            generated_dag_content: dag_content,
            task_count: dag.tasks.len(),
            task_names: dag.tasks.keys().cloned().collect(),
            has_dependencies: dag.dependencies.values().any(|deps| !deps.is_empty()),
        })
    }

    pub fn edit_generated_dag(
        &self,
        dag_content: &str,
        edits: Vec<DagEdit>,
    ) -> Result<String, JormError> {
        let mut modified_content = dag_content.to_string();

        for edit in edits {
            match edit {
                DagEdit::RenameTask { old_name, new_name } => {
                    modified_content = modified_content
                        .replace(&format!("task {}", old_name), &format!("task {}", new_name));
                    modified_content = modified_content.replace(&old_name, &new_name);
                }
                DagEdit::ModifyParameter {
                    task_name,
                    parameter,
                    new_value,
                } => {
                    // Simple regex-based parameter modification
                    let pattern =
                        format!(r#"(task {}\s*\{{[^}}]*){}: "[^"]*""#, task_name, parameter);
                    let replacement = format!(r#"$1{}: "{}""#, parameter, new_value);
                    if let Ok(re) = Regex::new(&pattern) {
                        modified_content = re
                            .replace(&modified_content, replacement.as_str())
                            .to_string();
                    }
                }
                DagEdit::AddDependency {
                    task_name,
                    dependency,
                } => {
                    // Add dependency to existing depends_on or create new one
                    let task_pattern = format!(r"(task {}\s*\{{[^}}]*)", task_name);
                    if let Ok(re) = Regex::new(&task_pattern) {
                        if modified_content.contains(&format!("task {}", task_name))
                            && modified_content.contains("depends_on:")
                        {
                            // Add to existing depends_on
                            let depends_pattern =
                                format!(r"(task {}\s*\{{[^}}]*depends_on: \[[^\]]*)", task_name);
                            if let Ok(depends_re) = Regex::new(&depends_pattern) {
                                let replacement = format!("$1, {}", dependency);
                                modified_content = depends_re
                                    .replace(&modified_content, replacement.as_str())
                                    .to_string();
                            }
                        } else {
                            // Add new depends_on line
                            let replacement = format!("$1\n    depends_on: [{}]", dependency);
                            modified_content = re
                                .replace(&modified_content, replacement.as_str())
                                .to_string();
                        }
                    }
                }
            }
        }

        // Validate the modified DAG
        self.validate_generated_dag(&modified_content)?;

        Ok(modified_content)
    }
}

#[derive(Debug, Clone)]
pub struct DagPreview {
    pub original_description: String,
    pub generated_dag_content: String,
    pub task_count: usize,
    pub task_names: Vec<String>,
    pub has_dependencies: bool,
}

#[derive(Debug, Clone)]
pub enum DagEdit {
    RenameTask {
        old_name: String,
        new_name: String,
    },
    ModifyParameter {
        task_name: String,
        parameter: String,
        new_value: String,
    },
    AddDependency {
        task_name: String,
        dependency: String,
    },
}

impl DagPreview {
    pub fn summary(&self) -> String {
        format!(
            "Generated DAG from: \"{}\"\n\
             Tasks: {} ({})\n\
             Dependencies: {}\n\n\
             DAG Content:\n{}",
            self.original_description,
            self.task_count,
            self.task_names.join(", "),
            if self.has_dependencies { "Yes" } else { "No" },
            self.generated_dag_content
        )
    }
}
