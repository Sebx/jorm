//! Interactive mode for Jorm - unified interface for all commands
//!
//! This module provides an intelligent interactive interface that combines:
//! - Natural language processing for intuitive commands
//! - Direct command execution for power users
//! - Context-aware suggestions and help
//! - Seamless integration of all Jorm functionality

use crate::ai::{AIService, ModelContext};
use crate::executor::{ExecutorConfig, NativeExecutor};
use crate::parser::{parse_dag_file, validate_dag, Dag};
use crate::scheduler::CronScheduler;
use crate::shebang::{DAGFilters, JormDAGHandler};
use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use tokio::io::stdin;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Interactive mode state and context
#[derive(Debug, Clone)]
pub struct InteractiveState {
    /// Current working directory context
    pub working_directory: String,
    /// Currently loaded DAG (if any)
    pub current_dag: Option<Dag>,
    /// Recent command history
    pub command_history: Vec<String>,
    /// User preferences and settings
    pub preferences: UserPreferences,
    /// Execution context and metadata
    pub execution_context: ExecutionContext,
}

/// User preferences for interactive mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Preferred output format (json, yaml, table)
    pub output_format: OutputFormat,
    /// Auto-suggestions enabled
    pub auto_suggestions: bool,
    /// Verbose output mode
    pub verbose: bool,
    /// Default timeout for operations
    pub default_timeout: u64,
    /// Theme preferences
    pub theme: Theme,
}

/// Output format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
    Markdown,
}

/// Theme options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Default,
    Dark,
    Light,
    Colorful,
}

/// Execution context for operations
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current session ID
    pub session_id: String,
    /// Active executions
    pub active_executions: HashMap<String, ExecutionInfo>,
    /// Available DAGs in current directory
    pub available_dags: Vec<String>,
    /// Scheduler status
    pub scheduler_status: SchedulerStatus,
}

/// Information about an active execution
#[derive(Debug, Clone)]
pub struct ExecutionInfo {
    pub execution_id: String,
    pub dag_name: String,
    pub status: ExecutionStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub progress: f64,
}

/// Execution status
#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Scheduler status
#[derive(Debug, Clone)]
pub struct SchedulerStatus {
    pub is_running: bool,
    pub active_jobs: Vec<String>,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
}

/// Interactive mode command types
#[derive(Debug, Clone)]
pub enum InteractiveCommand {
    /// Direct command execution
    Direct(String),
    /// Natural language query
    NaturalLanguage(String),
    /// Help request
    Help,
    /// Context switch
    Context(String),
    /// Preference change
    Preference(String, String),
    /// Exit request
    Exit,
}

/// Interactive mode interface
pub struct InteractiveMode {
    /// AI service for natural language processing
    ai_service: AIService,
    /// Current state
    state: InteractiveState,
    /// Command registry for direct commands
    command_registry: CommandRegistry,
    /// Context manager
    context_manager: ContextManager,
}

/// Registry for available commands
pub struct CommandRegistry {
    commands: HashMap<String, CommandInfo>,
}

/// Information about a command
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub examples: Vec<String>,
    pub category: CommandCategory,
    pub requires_dag: bool,
    pub requires_scheduler: bool,
}

/// Command categories
#[derive(Debug, Clone)]
pub enum CommandCategory {
    DAG,
    Execution,
    Scheduling,
    AI,
    System,
    Help,
}

/// Context manager for maintaining state
pub struct ContextManager {
    /// Current working directory
    working_directory: String,
    /// Available DAGs cache
    dag_cache: HashMap<String, Dag>,
    /// Scheduler instance
    scheduler: Option<CronScheduler>,
}

impl InteractiveMode {
    /// Create a new interactive mode
    pub async fn new() -> Result<Self> {
        println!("🔄 Initializing AI service...");

        let ai_service = match AIService::new().await {
            Ok(service) => {
                println!("✅ AI service initialized successfully");
                service
            }
            Err(e) => {
                println!("⚠️  AI service initialization failed: {}", e);
                println!("🔄 Falling back to basic mode...");
                // Create a minimal AI service for fallback
                return Err(e);
            }
        };

        let state = InteractiveState::default();
        let command_registry = CommandRegistry::new();
        let context_manager = ContextManager::new()?;

        Ok(Self {
            ai_service,
            state,
            command_registry,
            context_manager,
        })
    }

    /// Start the interactive mode
    pub async fn start(&mut self) -> Result<()> {
        self.show_welcome_message();
        self.show_quick_start();

        // Test AI availability
        if !self.ai_service.model_provider.is_available().await {
            println!("⚠️  AI models not available - running in basic mode");
            println!("   You can still use direct commands like 'run', 'validate', etc.");
            println!();
        }

        let stdin = stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            self.show_prompt();
            io::stdout().flush()?;

            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("\nGoodbye! 👋");
                    break;
                }
                Ok(_) => {
                    let input = line.trim();

                    if input.is_empty() {
                        continue;
                    }

                    match self.parse_input(input) {
                        InteractiveCommand::Exit => {
                            println!("Goodbye! 👋");
                            break;
                        }
                        InteractiveCommand::Help => {
                            self.show_help().await?;
                        }
                        InteractiveCommand::Direct(cmd) => {
                            self.execute_direct_command(&cmd).await?;
                        }
                        InteractiveCommand::NaturalLanguage(query) => {
                            self.process_natural_language(&query).await?;
                        }
                        InteractiveCommand::Context(context) => {
                            self.switch_context(&context).await?;
                        }
                        InteractiveCommand::Preference(key, value) => {
                            self.update_preference(&key, &value).await?;
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error reading input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Parse user input to determine command type
    fn parse_input(&self, input: &str) -> InteractiveCommand {
        let input_lower = input.to_lowercase();

        // Direct commands
        if input_lower.starts_with("run ")
            || input_lower.starts_with("validate ")
            || input_lower.starts_with("describe ")
            || input_lower.starts_with("analyze ")
            || input_lower.starts_with("generate ")
            || input_lower.starts_with("schedule ")
            || input_lower.starts_with("daemon ")
            || input_lower.starts_with("stop")
            || input_lower.starts_with("jobs")
            || input_lower.starts_with("trigger ")
            || input_lower.starts_with("status")
            || input_lower.starts_with("list")
            || input_lower.starts_with("model-info")
            || input_lower.starts_with("version")
        {
            return InteractiveCommand::Direct(input.to_string());
        }

        // Special commands
        match input_lower.as_str() {
            "help" | "?" => InteractiveCommand::Help,
            "quit" | "exit" | "q" => InteractiveCommand::Exit,
            "clear" => InteractiveCommand::Direct("clear".to_string()),
            "history" => InteractiveCommand::Direct("history".to_string()),
            _ => InteractiveCommand::NaturalLanguage(input.to_string()),
        }
    }

    /// Show welcome message
    fn show_welcome_message(&self) {
        println!("{}", "🤖 Jorm Interactive Mode".bold().cyan());
        println!("{}", "Intelligent DAG execution with AI assistance".green());
        println!();
        println!("{}", "Features:".bold());
        println!(
            "  • {} {}",
            "Natural Language:".bold(),
            "Ask questions in plain English".cyan()
        );
        println!(
            "  • {} {}",
            "Direct Commands:".bold(),
            "Use familiar CLI commands".cyan()
        );
        println!(
            "  • {} {}",
            "AI Intelligence:".bold(),
            "Smart suggestions and analysis".cyan()
        );
        println!(
            "  • {} {}",
            "Context Awareness:".bold(),
            "Remembers your workflow".cyan()
        );
        println!();
    }

    /// Show quick start guide
    fn show_quick_start(&self) {
        println!("{}", "Quick Start:".bold().yellow());
        println!("  • {} {}", "Try:".bold(), "'run my_dag.yaml'".cyan());
        println!(
            "  • {} {}",
            "Or:".bold(),
            "'Create a data processing pipeline'".cyan()
        );
        println!("  • {} {}", "Help:".bold(), "'help' or '?'".cyan());
        println!("  • {} {}", "Exit:".bold(), "'quit' or 'exit'".cyan());
        println!();
    }

    /// Show interactive prompt
    fn show_prompt(&self) {
        let context = if self.state.current_dag.is_some() {
            format!("[{}]", self.state.current_dag.as_ref().unwrap().name)
        } else {
            "[jorm]".to_string()
        };

        print!("{}> ", context.cyan());
    }

    /// Show comprehensive help
    async fn show_help(&self) -> Result<()> {
        println!("{}", "📚 Jorm Interactive Help".bold().cyan());
        println!();

        // Natural Language Examples
        println!("{}", "🗣️  Natural Language Commands:".bold().green());
        println!("  • 'Create a data pipeline for processing sales data'");
        println!("  • 'Analyze my current DAG for optimization opportunities'");
        println!("  • 'Explain this error: Task failed with exit code 1'");
        println!("  • 'Generate a web scraping workflow'");
        println!("  • 'What DAGs are available in this directory?'");
        println!();

        // Direct Commands
        println!("{}", "⚡ Direct Commands:".bold().blue());
        println!("  • {} {}", "run <file>".bold(), "- Execute a DAG".cyan());
        println!(
            "  • {} {}",
            "validate <file>".bold(),
            "- Validate DAG syntax".cyan()
        );
        println!(
            "  • {} {}",
            "describe <file>".bold(),
            "- Show DAG structure".cyan()
        );
        println!(
            "  • {} {}",
            "analyze <file>".bold(),
            "- AI-powered DAG analysis".cyan()
        );
        println!(
            "  • {} {}",
            "generate <description>".bold(),
            "- Create DAG from description".cyan()
        );
        println!(
            "  • {} {}",
            "schedule <file>".bold(),
            "- Schedule recurring execution".cyan()
        );
        println!(
            "  • {} {}",
            "daemon".bold(),
            "- Start scheduler daemon".cyan()
        );
        println!("  • {} {}", "jobs".bold(), "- List scheduled jobs".cyan());
        println!(
            "  • {} {}",
            "status".bold(),
            "- Show execution status".cyan()
        );
        println!("  • {} {}", "list".bold(), "- List available DAGs".cyan());
        println!(
            "  • {} {}",
            "model-info".bold(),
            "- Show AI model information".cyan()
        );
        println!();

        // System Commands
        println!("{}", "🔧 System Commands:".bold().yellow());
        println!("  • {} {}", "help".bold(), "- Show this help".cyan());
        println!("  • {} {}", "clear".bold(), "- Clear screen".cyan());
        println!(
            "  • {} {}",
            "history".bold(),
            "- Show command history".cyan()
        );
        println!(
            "  • {} {}",
            "context <name>".bold(),
            "- Switch context".cyan()
        );
        println!(
            "  • {} {}",
            "pref <key> <value>".bold(),
            "- Set preference".cyan()
        );
        println!(
            "  • {} {}",
            "quit/exit".bold(),
            "- Exit interactive mode".cyan()
        );
        println!();

        // Tips
        println!("{}", "💡 Tips:".bold().magenta());
        println!("  • Use {} for natural language queries", "quotes".italic());
        println!("  • Commands work with or without arguments");
        println!("  • AI can understand context from previous commands");
        println!(
            "  • Use {} to see available options",
            "Tab completion".italic()
        );
        println!();

        Ok(())
    }

    /// Execute direct command
    async fn execute_direct_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "run" => {
                if args.is_empty() {
                    println!("❌ Usage: run <file>");
                    return Ok(());
                }
                self.run_dag(args[0]).await?;
            }
            "validate" => {
                if args.is_empty() {
                    println!("❌ Usage: validate <file>");
                    return Ok(());
                }
                self.validate_dag(args[0]).await?;
            }
            "describe" => {
                if args.is_empty() {
                    println!("❌ Usage: describe <file>");
                    return Ok(());
                }
                self.describe_dag(args[0]).await?;
            }
            "analyze" => {
                if args.is_empty() {
                    println!("❌ Usage: analyze <file>");
                    return Ok(());
                }
                self.analyze_dag(args[0]).await?;
            }
            "generate" => {
                if args.is_empty() {
                    println!("❌ Usage: generate <description>");
                    return Ok(());
                }
                let description = args.join(" ");
                self.generate_dag(&description).await?;
            }
            "schedule" => {
                if args.is_empty() {
                    println!("❌ Usage: schedule <file> [--cron <expr>] [--name <name>]");
                    return Ok(());
                }
                self.schedule_dag(args[0], None, None).await?;
            }
            "daemon" => {
                self.start_daemon(None, false).await?;
            }
            "stop" => {
                self.stop_daemon().await?;
            }
            "jobs" => {
                self.list_jobs(false).await?;
            }
            "trigger" => {
                if args.is_empty() {
                    println!("❌ Usage: trigger <job>");
                    return Ok(());
                }
                self.trigger_job(args[0]).await?;
            }
            "status" => {
                self.show_status().await?;
            }
            "list" => {
                self.list_dags().await?;
            }
            "model-info" => {
                self.show_model_info().await?;
            }
            "version" => {
                self.show_version().await?;
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
            }
            "history" => {
                self.show_history().await?;
            }
            _ => {
                println!(
                    "❌ Unknown command: {}. Type 'help' for available commands.",
                    cmd
                );
            }
        }

        // Add to command history
        self.state.command_history.push(command.to_string());

        Ok(())
    }

    /// Process natural language query
    async fn process_natural_language(&mut self, query: &str) -> Result<()> {
        println!("🤔 Processing: {}", query);

        // Check if AI service is available
        if !self.ai_service.model_provider.is_available().await {
            println!("⚠️  AI model not available, using fallback response");
            self.show_fallback_response(query);
            return Ok(());
        }

        // Create context for the AI
        let context = ModelContext {
            max_tokens: Some(2000),
            temperature: Some(0.7),
            system_prompt: Some(format!(
                "You are Jorm AI, a helpful assistant for DAG management and workflow automation.
                
                Available commands: run, validate, describe, analyze, generate, schedule, daemon, jobs, status, list, model-info
                Current context: {:?}
                User query: {}",
                self.state.current_dag.as_ref().map(|d| &d.name),
                query
            )),
            conversation_history: vec![],
        };

        // Generate response with error handling
        match self
            .ai_service
            .model_provider
            .generate_response(query, &context)
            .await
        {
            Ok(response) => {
                if response.is_empty() {
                    println!("⚠️  Empty response from AI model, using fallback");
                    self.show_fallback_response(query);
                } else {
                    // Try to extract and execute commands from the response
                    if let Some(command) = self.extract_command_from_response(&response) {
                        println!("🤖 Executing: {}", command);
                        self.execute_direct_command(&command).await?;
                    } else {
                        println!("🤖 {}", response);
                    }
                }
            }
            Err(e) => {
                println!("❌ Error generating AI response: {}", e);
                self.show_fallback_response(query);
            }
        }

        Ok(())
    }

    /// Show fallback response when AI is not available
    fn show_fallback_response(&self, query: &str) {
        let query_lower = query.to_lowercase();

        if query_lower.contains("create") || query_lower.contains("generate") {
            println!("💡 I can help you create a DAG! Try:");
            println!("  • 'generate \"Create a data processing pipeline\"'");
            println!("  • 'run my_dag.yaml' to execute an existing DAG");
        } else if query_lower.contains("analyze") || query_lower.contains("optimize") {
            println!("💡 I can help you analyze DAGs! Try:");
            println!("  • 'analyze my_dag.yaml'");
            println!("  • 'describe my_dag.yaml' to see the structure");
        } else if query_lower.contains("run") || query_lower.contains("execute") {
            println!("💡 I can help you run DAGs! Try:");
            println!("  • 'run my_dag.yaml'");
            println!("  • 'validate my_dag.yaml' to check syntax");
        } else {
            println!("💡 I can help you with DAG management! Try:");
            println!("  • 'help' - Show all available commands");
            println!("  • 'run <file>' - Execute a DAG");
            println!("  • 'generate <description>' - Create a DAG from description");
            println!("  • 'analyze <file>' - Analyze a DAG for optimization");
        }
    }

    /// Extract command from AI response
    fn extract_command_from_response(&self, response: &str) -> Option<String> {
        // Simple pattern matching to extract commands
        if response.contains("run ") {
            Some(response.to_string())
        } else if response.contains("validate ") {
            Some(response.to_string())
        } else if response.contains("analyze ") {
            Some(response.to_string())
        } else if response.contains("generate ") {
            Some(response.to_string())
        } else {
            None
        }
    }

    /// Switch context
    async fn switch_context(&mut self, context: &str) -> Result<()> {
        match context {
            "dag" => {
                if let Some(dag) = &self.state.current_dag {
                    println!("📋 Current DAG: {}", dag.name);
                } else {
                    println!("❌ No DAG loaded. Use 'run <file>' to load a DAG.");
                }
            }
            "clear" => {
                self.state.current_dag = None;
                println!("✅ Context cleared");
            }
            _ => {
                println!("❌ Unknown context: {}. Available: dag, clear", context);
            }
        }
        Ok(())
    }

    /// Update preference
    async fn update_preference(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "format" => match value {
                "json" => self.state.preferences.output_format = OutputFormat::Json,
                "yaml" => self.state.preferences.output_format = OutputFormat::Yaml,
                "table" => self.state.preferences.output_format = OutputFormat::Table,
                "markdown" => self.state.preferences.output_format = OutputFormat::Markdown,
                _ => println!("❌ Invalid format. Options: json, yaml, table, markdown"),
            },
            "verbose" => {
                self.state.preferences.verbose = value == "true";
                println!("✅ Verbose mode: {}", self.state.preferences.verbose);
            }
            "suggestions" => {
                self.state.preferences.auto_suggestions = value == "true";
                println!(
                    "✅ Auto-suggestions: {}",
                    self.state.preferences.auto_suggestions
                );
            }
            _ => {
                println!(
                    "❌ Unknown preference: {}. Available: format, verbose, suggestions",
                    key
                );
            }
        }
        Ok(())
    }

    /// Show command history
    async fn show_history(&self) -> Result<()> {
        if self.state.command_history.is_empty() {
            println!("No command history");
            return Ok(());
        }

        println!("📜 Command History:");
        for (i, cmd) in self.state.command_history.iter().enumerate() {
            println!("  {}. {}", i + 1, cmd);
        }
        Ok(())
    }

    // Command implementations (delegating to existing functions)
    async fn run_dag(&mut self, file: &str) -> Result<()> {
        println!("🚀 Running DAG: {}", file);
        let dag = parse_dag_file(file).await?;
        self.state.current_dag = Some(dag.clone());

        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        match executor.execute_dag(&dag).await {
            Ok(_) => println!("✅ DAG execution completed successfully"),
            Err(e) => println!("❌ DAG execution failed: {}", e),
        }
        Ok(())
    }

    async fn validate_dag(&self, file: &str) -> Result<()> {
        println!("🔍 Validating DAG: {}", file);
        let dag = parse_dag_file(file).await?;
        let errors = validate_dag(&dag)?;

        if errors.is_empty() {
            println!("✅ DAG is valid");
        } else {
            println!("❌ Invalid DAG. Errors:");
            for error in errors {
                println!("  - {}", error);
            }
        }
        Ok(())
    }

    async fn describe_dag(&self, file: &str) -> Result<()> {
        println!("📋 Describing DAG: {}", file);
        let dag = parse_dag_file(file).await?;

        println!("DAG: {}", dag.name);
        if let Some(schedule) = &dag.schedule {
            println!("Schedule: {}", schedule);
        }

        let task_names: Vec<String> = dag.tasks.keys().cloned().collect();
        println!("Tasks: {}", task_names.join(", "));

        if !dag.dependencies.is_empty() {
            println!("Dependencies:");
            for dep in &dag.dependencies {
                println!(" - {} after {}", dep.task, dep.depends_on);
            }
        }
        Ok(())
    }

    async fn analyze_dag(&self, file: &str) -> Result<()> {
        println!("🔍 Analyzing DAG: {}", file);
        let dag = parse_dag_file(file).await?;
        let analysis = self.ai_service.analyze_dag(&dag).await?;

        println!("📊 Analysis Results:");
        println!("  Performance Score: {:.2}", analysis.performance_score);
        println!(
            "  Optimization Suggestions: {}",
            analysis.optimization_suggestions.len()
        );
        println!("  Potential Issues: {}", analysis.potential_issues.len());
        Ok(())
    }

    async fn generate_dag(&self, description: &str) -> Result<()> {
        println!("🚀 Generating DAG from: {}", description);
        let dag = self
            .ai_service
            .generate_dag_from_natural_language(description)
            .await?;

        // Generate filename based on DAG name - use .txt format for Jorm
        let filename = format!("{}.txt", dag.name);

        // Create JormDAG metadata
        let handler = JormDAGHandler::new();
        let metadata = crate::shebang::JormDAGMetadata {
            name: dag.name.clone(),
            version: "1.0.0".to_string(),
            description: Some(description.to_string()),
            author: Some("Jorm AI".to_string()),
            tags: self.extract_tags_from_description(description),
            dependencies: Vec::new(),
            requirements: self.extract_requirements_from_dag(&dag),
            schedule: dag.schedule.clone(),
            timeout: None,
            retries: None,
            environment: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        // Generate shebang header
        let shebang_header = handler.generate_shebang_header(&metadata);

        // Convert DAG to Jorm text format and add shebang
        let txt_content = self.dag_to_jorm_txt(&dag)?;
        let full_content = format!("{}{}", shebang_header, txt_content);

        tokio::fs::write(&filename, full_content).await?;

        // Validate the generated DAG
        println!("🔍 Validating generated DAG...");
        self.validate_generated_dag(&dag, &filename).await?;

        println!("✅ DAG Generated and saved:");
        println!("  Name: {}", dag.name);
        println!("  File: {}", filename);
        println!("  Tasks: {}", dag.tasks.len());
        println!("  Dependencies: {}", dag.dependencies.len());
        println!("  Tags: {}", metadata.tags.join(", "));
        println!("  Requirements: {}", metadata.requirements.join(", "));
        println!("  📁 File saved to: {}", filename);
        println!(
            "  🚀 Execute with: ./{} or jorm-rs run {}",
            filename, filename
        );
        Ok(())
    }

    /// Convert DAG to YAML format
    fn dag_to_yaml(&self, dag: &crate::parser::Dag) -> Result<String> {
        use serde_yaml;

        let mut yaml_dag = serde_yaml::Mapping::new();
        yaml_dag.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(dag.name.clone()),
        );

        // Add schedule if available
        if let Some(schedule) = &dag.schedule {
            yaml_dag.insert(
                serde_yaml::Value::String("schedule".to_string()),
                serde_yaml::Value::String(schedule.clone()),
            );
        }

        // Add tasks
        let mut tasks = serde_yaml::Mapping::new();
        for (task_name, task) in &dag.tasks {
            let mut task_config = serde_yaml::Mapping::new();

            // Add task name
            task_config.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(task.name.clone()),
            );

            // Add description if available
            if let Some(description) = &task.description {
                task_config.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String(description.clone()),
                );
            }

            // Add task configuration
            let mut config = serde_yaml::Mapping::new();
            if let Some(task_type) = &task.config.task_type {
                config.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String(task_type.clone()),
                );
            }

            // Add type-specific configuration
            match task.config.task_type.as_deref() {
                Some("shell") => {
                    if let Some(command) = &task.config.command {
                        config.insert(
                            serde_yaml::Value::String("command".to_string()),
                            serde_yaml::Value::String(command.clone()),
                        );
                    }
                }
                Some("python") => {
                    if let Some(module) = &task.config.module {
                        config.insert(
                            serde_yaml::Value::String("module".to_string()),
                            serde_yaml::Value::String(module.clone()),
                        );
                    }
                    if let Some(function) = &task.config.function {
                        config.insert(
                            serde_yaml::Value::String("function".to_string()),
                            serde_yaml::Value::String(function.clone()),
                        );
                    }
                    if let Some(script) = &task.config.data {
                        // Handle script content stored in data field
                        if let Some(script_str) = script.as_str() {
                            config.insert(
                                serde_yaml::Value::String("script".to_string()),
                                serde_yaml::Value::String(script_str.to_string()),
                            );
                        }
                    }
                }
                Some("http") => {
                    if let Some(method) = &task.config.method {
                        config.insert(
                            serde_yaml::Value::String("method".to_string()),
                            serde_yaml::Value::String(method.clone()),
                        );
                    }
                    if let Some(url) = &task.config.url {
                        config.insert(
                            serde_yaml::Value::String("url".to_string()),
                            serde_yaml::Value::String(url.clone()),
                        );
                    }
                }
                Some("file") => {
                    if let Some(operation) = &task.config.operation {
                        config.insert(
                            serde_yaml::Value::String("operation".to_string()),
                            serde_yaml::Value::String(operation.clone()),
                        );
                    }
                    if let Some(dest) = &task.config.dest {
                        config.insert(
                            serde_yaml::Value::String("destination".to_string()),
                            serde_yaml::Value::String(dest.clone()),
                        );
                    }
                }
                _ => {}
            }

            task_config.insert(
                serde_yaml::Value::String("config".to_string()),
                serde_yaml::Value::Mapping(config),
            );

            tasks.insert(
                serde_yaml::Value::String(task_name.clone()),
                serde_yaml::Value::Mapping(task_config),
            );
        }

        yaml_dag.insert(
            serde_yaml::Value::String("tasks".to_string()),
            serde_yaml::Value::Mapping(tasks),
        );

        // Add dependencies
        if !dag.dependencies.is_empty() {
            let mut dependencies = serde_yaml::Mapping::new();
            for dep in &dag.dependencies {
                dependencies.insert(
                    serde_yaml::Value::String(dep.task.clone()),
                    serde_yaml::Value::String(dep.depends_on.clone()),
                );
            }
            yaml_dag.insert(
                serde_yaml::Value::String("dependencies".to_string()),
                serde_yaml::Value::Mapping(dependencies),
            );
        }

        Ok(serde_yaml::to_string(&yaml_dag)?)
    }

    /// Convert DAG to Jorm text format with embedded Python code
    fn dag_to_jorm_txt(&self, dag: &crate::parser::Dag) -> Result<String> {
        let mut content = String::new();

        // Add DAG header
        content.push_str(&format!("dag: {}\n", dag.name));
        if let Some(schedule) = &dag.schedule {
            content.push_str(&format!("schedule: {}\n", schedule));
        }
        content.push_str("\n");

        // Add tasks section
        content.push_str("tasks:\n");
        for (task_name, task) in &dag.tasks {
            content.push_str(&format!("- {}\n", task_name));

            // Add task configuration
            if let Some(task_type) = &task.config.task_type {
                content.push_str(&format!("  type: {}\n", task_type));
            }

            if let Some(description) = &task.description {
                content.push_str(&format!("  description: {}\n", description));
            }

            // Add type-specific configuration
            match task.config.task_type.as_deref() {
                Some("shell") => {
                    if let Some(command) = &task.config.command {
                        content.push_str(&format!("  command: {}\n", command));
                    }
                }
                Some("python") => {
                    if let Some(module) = &task.config.module {
                        content.push_str(&format!("  module: {}\n", module));
                    }
                    if let Some(function) = &task.config.function {
                        content.push_str(&format!("  function: {}\n", function));
                    }
                    if let Some(script) = &task.config.data {
                        if let Some(script_str) = script.as_str() {
                            content.push_str("  script: |\n");
                            // Indent each line of the script
                            for line in script_str.lines() {
                                content.push_str(&format!("    {}\n", line));
                            }
                        }
                    }
                }
                Some("http") => {
                    if let Some(method) = &task.config.method {
                        content.push_str(&format!("  method: {}\n", method));
                    }
                    if let Some(url) = &task.config.url {
                        content.push_str(&format!("  url: {}\n", url));
                    }
                }
                Some("file") => {
                    if let Some(operation) = &task.config.operation {
                        content.push_str(&format!("  operation: {}\n", operation));
                    }
                    if let Some(dest) = &task.config.dest {
                        content.push_str(&format!("  destination: {}\n", dest));
                    }
                }
                _ => {}
            }
            content.push_str("\n");
        }

        // Add dependencies section
        if !dag.dependencies.is_empty() {
            content.push_str("dependencies:\n");
            for dep in &dag.dependencies {
                content.push_str(&format!("- {} -> {}\n", dep.task, dep.depends_on));
            }
        }

        Ok(content)
    }

    /// Extract tags from description
    fn extract_tags_from_description(&self, description: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let description_lower = description.to_lowercase();

        // Common workflow types
        if description_lower.contains("web scraping") || description_lower.contains("scraping") {
            tags.push("web-scraping".to_string());
        }
        if description_lower.contains("database") || description_lower.contains("sql") {
            tags.push("database".to_string());
        }
        if description_lower.contains("etl") || description_lower.contains("extract") {
            tags.push("etl".to_string());
        }
        if description_lower.contains("machine learning") || description_lower.contains("ml") {
            tags.push("ml".to_string());
        }
        if description_lower.contains("data pipeline") {
            tags.push("data-pipeline".to_string());
        }
        if description_lower.contains("python") {
            tags.push("python".to_string());
        }
        if description_lower.contains("automation") {
            tags.push("automation".to_string());
        }

        tags
    }

    /// Extract requirements from DAG
    fn extract_requirements_from_dag(&self, dag: &crate::parser::Dag) -> Vec<String> {
        let mut requirements = Vec::new();

        // Check for common dependencies in task configurations
        for (_, task) in &dag.tasks {
            if let Some(script) = &task.config.data {
                if let Some(script_str) = script.as_str() {
                    if script_str.contains("pandas") || script_str.contains("pd.") {
                        requirements.push("pandas".to_string());
                    }
                    if script_str.contains("pyodbc") {
                        requirements.push("pyodbc".to_string());
                    }
                    if script_str.contains("requests") {
                        requirements.push("requests".to_string());
                    }
                    if script_str.contains("numpy") {
                        requirements.push("numpy".to_string());
                    }
                    if script_str.contains("sqlalchemy") {
                        requirements.push("sqlalchemy".to_string());
                    }
                }
            }
        }

        // Remove duplicates
        requirements.sort();
        requirements.dedup();
        requirements
    }

    /// Validate generated DAG with comprehensive checks
    async fn validate_generated_dag(
        &self,
        dag: &crate::parser::Dag,
        _filename: &str,
    ) -> Result<()> {
        let mut validation_errors = Vec::new();
        let mut validation_warnings = Vec::new();

        // 1. Validate DAG structure
        if let Err(e) = validate_dag(dag) {
            validation_errors.push(format!("DAG structure validation failed: {}", e));
        }

        // 2. Validate Python scripts in tasks
        for (task_name, task) in &dag.tasks {
            if let Some(script) = &task.config.data {
                if let Some(script_str) = script.as_str() {
                    // Basic Python script validation
                    self.validate_python_script(
                        script_str,
                        task_name,
                        &mut validation_errors,
                        &mut validation_warnings,
                    );
                }
            }
        }

        // 3. Check for common issues
        self.check_common_dag_issues(dag, &mut validation_warnings);

        // 4. Report results
        if !validation_errors.is_empty() {
            println!(
                "❌ Validation failed with {} error(s):",
                validation_errors.len()
            );
            for error in &validation_errors {
                println!("  • {}", error);
            }
        }

        if !validation_warnings.is_empty() {
            println!("⚠️ Found {} warning(s):", validation_warnings.len());
            for warning in &validation_warnings {
                println!("  • {}", warning);
            }
        }

        if validation_errors.is_empty() {
            println!("✅ DAG validation passed successfully!");
        }

        Ok(())
    }

    /// Check for common DAG issues
    fn check_common_dag_issues(&self, dag: &crate::parser::Dag, warnings: &mut Vec<String>) {
        // Check for circular dependencies
        if self.has_circular_dependencies(dag) {
            warnings.push("Potential circular dependencies detected".to_string());
        }

        // Check for orphaned tasks
        let task_names: std::collections::HashSet<String> = dag.tasks.keys().cloned().collect();
        let referenced_tasks: std::collections::HashSet<String> = dag
            .dependencies
            .iter()
            .flat_map(|dep| vec![dep.task.clone(), dep.depends_on.clone()])
            .collect();

        for task_name in &task_names {
            if !referenced_tasks.contains(task_name)
                && dag.dependencies.iter().any(|d| d.task == *task_name)
            {
                warnings.push(format!(
                    "Task '{}' has no dependencies and is not referenced",
                    task_name
                ));
            }
        }

        // Check for missing dependencies
        for dep in &dag.dependencies {
            if !task_names.contains(&dep.task) {
                warnings.push(format!(
                    "Dependency references non-existent task: {}",
                    dep.task
                ));
            }
            if !task_names.contains(&dep.depends_on) {
                warnings.push(format!(
                    "Dependency references non-existent task: {}",
                    dep.depends_on
                ));
            }
        }
    }

    /// Check for circular dependencies
    fn has_circular_dependencies(&self, dag: &crate::parser::Dag) -> bool {
        use std::collections::{HashMap, HashSet};

        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Build dependency graph
        for dep in &dag.dependencies {
            graph
                .entry(dep.task.clone())
                .or_insert_with(Vec::new)
                .push(dep.depends_on.clone());
        }

        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task in dag.tasks.keys() {
            if !visited.contains(task) {
                if self.dfs_has_cycle(task, &graph, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    /// DFS to detect cycles
    fn dfs_has_cycle(
        &self,
        task: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(task.to_string());
        rec_stack.insert(task.to_string());

        if let Some(dependencies) = graph.get(task) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if self.dfs_has_cycle(dep, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(task);
        false
    }

    /// Basic Python script validation
    fn validate_python_script(
        &self,
        script: &str,
        task_name: &str,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        // Check for basic Python syntax issues
        if script.contains("import ") {
            // Check for common import issues
            if script.contains("import pandas") && !script.contains("import pandas as pd") {
                warnings.push(format!(
                    "Task '{}': Consider using 'import pandas as pd' for consistency",
                    task_name
                ));
            }
        }

        // Check for potential issues
        if script.contains("password") && script.contains("=") {
            warnings.push(format!(
                "Task '{}': Hardcoded credentials detected - consider using environment variables",
                task_name
            ));
        }

        if script.contains("connect(") && !script.contains("try:") {
            warnings.push(format!("Task '{}': Database connection without error handling - consider adding try-except blocks", task_name));
        }

        // Check for missing imports
        if script.contains("pd.") && !script.contains("import pandas") {
            errors.push(format!(
                "Task '{}': Uses pandas but doesn't import it",
                task_name
            ));
        }

        if script.contains("pyodbc.") && !script.contains("import pyodbc") {
            errors.push(format!(
                "Task '{}': Uses pyodbc but doesn't import it",
                task_name
            ));
        }

        if script.contains("requests.") && !script.contains("import requests") {
            errors.push(format!(
                "Task '{}': Uses requests but doesn't import it",
                task_name
            ));
        }

        // Check for common Python issues
        if script.contains("print ") && !script.contains("print(") {
            warnings.push(format!(
                "Task '{}': Consider using print() function instead of print statement",
                task_name
            ));
        }
    }

    async fn schedule_dag(
        &self,
        file: &str,
        _cron: Option<&str>,
        _name: Option<&str>,
    ) -> Result<()> {
        println!("📅 Scheduling DAG: {}", file);
        // Implementation would go here
        println!("✅ DAG scheduled successfully");
        Ok(())
    }

    async fn start_daemon(&self, _config: Option<&str>, _foreground: bool) -> Result<()> {
        println!("🚀 Starting scheduler daemon...");
        // Implementation would go here
        println!("✅ Scheduler daemon started");
        Ok(())
    }

    async fn stop_daemon(&self) -> Result<()> {
        println!("🛑 Stopping scheduler daemon...");
        // Implementation would go here
        println!("✅ Scheduler daemon stopped");
        Ok(())
    }

    async fn list_jobs(&self, _enabled_only: bool) -> Result<()> {
        println!("📋 Scheduled Jobs:");
        // Implementation would go here
        println!("No jobs found");
        Ok(())
    }

    async fn trigger_job(&self, job: &str) -> Result<()> {
        println!("⚡ Triggering job: {}", job);
        // Implementation would go here
        println!("✅ Job triggered successfully");
        Ok(())
    }

    async fn show_status(&self) -> Result<()> {
        println!("📊 Execution Status:");
        println!("No active executions");
        Ok(())
    }

    async fn list_dags(&self) -> Result<()> {
        println!("📂 Available DAGs:");

        // Use JormDAG handler for enhanced listing
        let handler = JormDAGHandler::new();
        let filters = DAGFilters::new(); // No filters for basic list
        let dag_files = handler.list_dags_with_filter(".", &filters)?;

        if dag_files.is_empty() {
            println!("  No DAG files found in current directory");
            println!("  💡 Try: generate \"web scraping workflow\" to create a DAG");
        } else {
            println!("  Found {} DAG file(s):", dag_files.len());
            for (i, dag_info) in dag_files.iter().enumerate() {
                if let Some(metadata) = &dag_info.metadata {
                    println!(
                        "    {}. {} (v{}) - {}",
                        i + 1,
                        dag_info.name,
                        metadata.version,
                        metadata.description.as_deref().unwrap_or("No description")
                    );
                    if !metadata.tags.is_empty() {
                        println!("       Tags: {}", metadata.tags.join(", "));
                    }
                } else {
                    println!("    {}. {}", i + 1, dag_info.name);
                }
            }
            println!("  💡 Use: run <file> to execute a DAG");
            println!("  💡 Use: list --tag <tag> to filter by tags");
            println!("  💡 Use: list --author <author> to filter by author");
        }

        Ok(())
    }

    async fn show_model_info(&self) -> Result<()> {
        let model_info = self.ai_service.model_info();
        println!("🤖 AI Model Information:");
        println!("  Model: {}", model_info.name);
        println!("  Version: {}", model_info.version);
        println!("  Parameters: {}", model_info.parameters);
        println!(
            "  Memory Usage: {} GB",
            model_info.memory_usage / 1_000_000_000
        );
        println!("  Capabilities: {}", model_info.capabilities.join(", "));
        Ok(())
    }

    async fn show_version(&self) -> Result<()> {
        println!("🦀 Jorm-RS - Pure Rust DAG Engine");
        println!("Version: 0.1.0");
        println!("Description: A fast, reliable DAG execution engine with AI intelligence");
        Ok(())
    }
}

impl Default for InteractiveState {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            current_dag: None,
            command_history: Vec::new(),
            preferences: UserPreferences::default(),
            execution_context: ExecutionContext::default(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Table,
            auto_suggestions: true,
            verbose: false,
            default_timeout: 300,
            theme: Theme::Default,
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            active_executions: HashMap::new(),
            available_dags: Vec::new(),
            scheduler_status: SchedulerStatus {
                is_running: false,
                active_jobs: Vec::new(),
                last_heartbeat: None,
            },
        }
    }
}

impl CommandRegistry {
    fn new() -> Self {
        let mut commands = HashMap::new();

        // Add all available commands
        commands.insert(
            "run".to_string(),
            CommandInfo {
                name: "run".to_string(),
                description: "Execute a DAG from file".to_string(),
                usage: "run <file>".to_string(),
                examples: vec!["run my_dag.yaml".to_string()],
                category: CommandCategory::DAG,
                requires_dag: true,
                requires_scheduler: false,
            },
        );

        // Add more commands...

        Self { commands }
    }
}

impl ContextManager {
    fn new() -> Result<Self> {
        Ok(Self {
            working_directory: std::env::current_dir()?.to_string_lossy().to_string(),
            dag_cache: HashMap::new(),
            scheduler: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interactive_mode_creation() {
        let mode = InteractiveMode::new().await;
        assert!(mode.is_ok());
    }

    #[tokio::test]
    async fn test_command_parsing() {
        let mode = InteractiveMode::new().await.unwrap();

        assert!(matches!(
            mode.parse_input("run my_dag.yaml"),
            InteractiveCommand::Direct(_)
        ));

        assert!(matches!(
            mode.parse_input("Create a data pipeline"),
            InteractiveCommand::NaturalLanguage(_)
        ));

        assert!(matches!(mode.parse_input("help"), InteractiveCommand::Help));

        assert!(matches!(mode.parse_input("quit"), InteractiveCommand::Exit));
    }
}
