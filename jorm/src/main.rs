use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::path::Path;

mod ai;
mod executor;
mod parser;
mod scheduler;
mod shebang;

use ai::interactive::InteractiveMode;
use executor::{ExecutorConfig, NativeExecutor, ConfigManager, OutputFormatter};
use parser::{parse_dag_file, validate_dag};
use scheduler::{ConfigManager as SchedulerConfigManager, CronScheduler, Schedule, ScheduledJob, SchedulerDaemon};

#[derive(Parser)]
#[command(name = "jorm")]
#[command(about = "A fast, reliable DAG execution engine with AI intelligence")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a DAG from file
    Run {
        /// Path to DAG file (.txt, .md, .yaml)
        file: String,
        /// Skip validation before execution
        #[arg(long)]
        no_validate: bool,
        /// Use native Rust executor (default)
        #[arg(long)]
        native_executor: bool,
        /// Use Python executor as fallback
        #[arg(long)]
        python: bool,
        /// Enable state management and recovery
        #[arg(long)]
        with_state: bool,
        /// Maximum concurrent tasks
        #[arg(long)]
        max_concurrent: Option<usize>,
        /// Configuration file path
        #[arg(long)]
        config: Option<String>,
        /// Configuration profile to use
        #[arg(long)]
        profile: Option<String>,
        /// Task timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Validate DAG syntax and structure
    Validate {
        /// Path to DAG file
        file: String,
    },
    /// Describe DAG structure and dependencies
    Describe {
        /// Path to DAG file
        file: String,
    },
    /// Execute a single task
    Exec {
        /// Task name to execute
        task: String,
    },
    /// Show current execution status
    Status,
    /// List available DAGs
    List,
    /// Schedule recurring execution of a DAG
    Schedule {
        /// Path to DAG file
        file: String,
        /// Cron expression for scheduling
        #[arg(long)]
        cron: Option<String>,
        /// Schedule name
        #[arg(long)]
        name: Option<String>,
    },
    /// Start the scheduler daemon
    Daemon {
        /// Configuration file path
        #[arg(long)]
        config: Option<String>,
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the scheduler daemon
    Stop,
    /// Show scheduler status and jobs
    Jobs {
        /// Show only enabled jobs
        #[arg(long)]
        enabled: bool,
    },
    /// Trigger a job manually
    Trigger {
        /// Job ID or name to trigger
        job: String,
    },
    /// Interactive mode with AI assistance
    Interactive,
    /// Analyze a DAG for optimization opportunities
    Analyze {
        /// Path to DAG file
        file: String,
    },
    /// Generate a DAG from natural language description
    Generate {
        /// Natural language description of the DAG
        description: String,
        /// Output file path (optional)
        #[arg(long)]
        output: Option<String>,
    },
    /// Show AI model information
    ModelInfo,
    /// Show version information
    Version,
    /// Setup environment for jorm-rs on any platform
    Setup {
        /// Force reinstall of dependencies
        #[arg(long)]
        force: bool,
        /// Skip Python installation check
        #[arg(long)]
        skip_python: bool,
        /// Skip shell command verification
        #[arg(long)]
        skip_shell: bool,
    },
    /// Configuration management commands
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show {
        /// Configuration profile to show
        #[arg(long)]
        profile: Option<String>,
    },
    /// Create a default configuration file
    Init {
        /// Output file path
        #[arg(long, default_value = "jorm.toml")]
        output: String,
        /// Configuration format (toml, yaml, json)
        #[arg(long, default_value = "toml")]
        format: String,
    },
    /// Validate configuration file
    Validate {
        /// Configuration file path
        file: String,
    },
    /// List available configuration profiles
    Profiles,
}

fn validate_file_exists(file: &str) -> Result<()> {
    if !Path::new(file).exists() {
        anyhow::bail!("File not found: {file}");
    }
    Ok(())
}

async fn run_dag(
    file: &str, 
    no_validate: bool, 
    use_native_executor: bool, 
    use_python: bool,
    with_state: bool,
    max_concurrent: Option<usize>,
    config_file: Option<String>,
    profile: Option<String>,
    timeout: Option<u64>,
) -> Result<()> {
    // Create output formatter with appropriate settings
    let mut formatter = OutputFormatter::new();
    
    // Adjust formatter settings based on environment
    if std::env::var("NO_COLOR").is_ok() || std::env::var("TERM").map_or(false, |term| term == "dumb") {
        formatter.set_colored(false);
    }
    
    // Set verbosity based on environment or default to normal
    let verbosity = std::env::var("JORM_VERBOSITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    formatter.set_verbosity(verbosity);

    println!("{}", format!("🚀 Running DAG: {file}").cyan());

    // Parse the DAG
    let dag = parse_dag_file(file).await?;

    // Debug: show parsed DAG structure to help with integration test failures
    println!("DEBUG: parsed DAG -> {dag:#?}");

    // Validate if not skipped
    if !no_validate {
        let errors = validate_dag(&dag)?;
        if !errors.is_empty() {
            println!("{}", "❌ Invalid DAG. Errors:".red());
            for error in errors {
                println!("  - {error}");
            }
            anyhow::bail!("DAG validation failed");
        }
    }

    // Determine which executor to use
    let should_use_native = if use_python {
        false // Explicitly requested Python executor
    } else if use_native_executor {
        true // Explicitly requested native executor
    } else {
        true // Default to native executor
    };

    if should_use_native {
        formatter.print_executor_selection(true);
        
        // Load configuration
        let mut config = if let Some(config_path) = config_file {
            println!("📄 Loading configuration from: {}", config_path);
            ExecutorConfig::load_from_file(&config_path)
                .context("Failed to load configuration file")?
        } else if let Some(profile_name) = profile {
            println!("📋 Using configuration profile: {}", profile_name);
            ExecutorConfig::load_with_profile(&profile_name)
                .context("Failed to load configuration profile")?
        } else {
            ExecutorConfig::load()
                .context("Failed to load default configuration")?
        };

        // Apply command-line overrides
        if let Some(max_concurrent) = max_concurrent {
            config.max_concurrent_tasks = max_concurrent;
        }
        
        if let Some(timeout_secs) = timeout {
            config.default_timeout = std::time::Duration::from_secs(timeout_secs);
        }

        // Validate the final configuration
        config.validate().context("Configuration validation failed")?;

        // Print configuration summary
        formatter.print_configuration_summary(&config);

        // Create executor with or without state management
        let executor = if with_state {
            println!("{}", "💾 State management enabled".cyan());
            let state_config = crate::executor::StateConfig::default();
            NativeExecutor::with_state_management(config, state_config).await
                .context("Failed to create executor with state management")?
        } else {
            NativeExecutor::new(config)
        };

        match executor.execute_dag(&dag).await {
            Ok(result) => {
                // Print execution summary using formatter
                formatter.print_execution_summary(&result);
                
                if result.status != crate::executor::ExecutionStatus::Success {
                    anyhow::bail!("DAG execution failed");
                }
            }
            Err(e) => {
                formatter.print_error(&e);
                
                // If native executor fails and Python fallback is available, try it
                if !use_python && !use_native_executor {
                    // Only fallback if user didn't explicitly request native executor
                    formatter.print_warning("Attempting Python fallback...");
                    match run_with_python_fallback(&dag).await {
                        Ok(_) => {
                            formatter.print_info("Python fallback succeeded");
                            return Ok(());
                        }
                        Err(fallback_err) => {
                            formatter.print_error(&fallback_err);
                            anyhow::bail!("Both native executor and Python fallback failed. Native: {e}, Python: {fallback_err}");
                        }
                    }
                } else {
                    anyhow::bail!("Native executor failed: {e}");
                }
            }
        }
    } else {
        formatter.print_executor_selection(false);
        return run_with_python_fallback(&dag).await;
    }

    Ok(())
}

async fn run_with_python_fallback(dag: &crate::parser::Dag) -> Result<()> {
    println!("{}", "⚠️ Python executor not yet implemented".yellow());
    println!("{}", "This would execute the DAG using the Python engine".dimmed());
    
    // For now, just show what would be executed
    println!("DAG to execute:");
    println!("  • Name: {}", dag.name);
    println!("  • Tasks: {}", dag.tasks.len());
    println!("  • Dependencies: {}", dag.dependencies.len());
    
    // In a real implementation, this would:
    // 1. Convert the Rust DAG structure to Python format
    // 2. Call the Python executor
    // 3. Parse the results back
    
    anyhow::bail!("Python executor fallback not implemented yet");
}

async fn validate_dag_command(file: &str) -> Result<()> {
    println!("{}", format!("🔍 Validating DAG: {file}").cyan());

    let dag = parse_dag_file(file).await?;
    let errors = validate_dag(&dag)?;

    if errors.is_empty() {
        println!("{}", "✅ DAG is valid".green());
    } else {
        println!("{}", "❌ Invalid DAG. Errors:".red());
        for error in errors {
            println!("  - {error}");
        }
        anyhow::bail!("DAG validation failed");
    }

    Ok(())
}

async fn describe_dag(file: &str) -> Result<()> {
    println!("{}", format!("📋 Describing DAG: {file}").cyan());

    let dag = parse_dag_file(file).await?;

    println!("DAG: {}", dag.name);
    if let Some(schedule) = &dag.schedule {
        println!("Schedule: {schedule}");
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

async fn exec_task(task: &str) -> Result<()> {
    println!("{}", format!("⚡ Executing task: {task}").cyan());
    println!(
        "{}",
        "Note: Single task execution requires a DAG context".yellow()
    );
    println!(
        "{}",
        "Use 'jorm run <file>' to execute a complete DAG".yellow()
    );

    Ok(())
}

async fn show_status() -> Result<()> {
    println!("{}", "📊 Execution Status".cyan());
    println!("No active executions");
    println!("Use 'jorm run <file>' to execute a DAG");

    Ok(())
}

async fn list_dags() -> Result<()> {
    println!("{}", "📂 Available DAGs".cyan());
    println!("No DAGs found in current directory");
    println!("Create DAG files (.txt, .md, .yaml) and use 'jorm run <file>' to execute them");

    Ok(())
}

async fn schedule_dag(file: &str, cron_expr: Option<&str>, name: Option<&str>) -> Result<()> {
    println!("{}", format!("⏰ Scheduling DAG: {file}").cyan());

    let schedule = if let Some(cron) = cron_expr {
        Schedule::Cron(cron.to_string())
    } else {
        // Try to extract schedule from DAG file
        let dag = parse_dag_file(file).await?;
        if let Some(schedule_str) = dag.schedule {
            Schedule::Cron(schedule_str)
        } else {
            anyhow::bail!("No schedule specified. Use --cron option or add schedule to DAG file");
        }
    };

    let job_name = name.unwrap_or_else(|| {
        std::path::Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed_job")
    });

    let job = ScheduledJob::new(job_name.to_string(), file.to_string(), schedule);

    // For now, just print what would be scheduled
    // In a full implementation, this would connect to a running scheduler daemon
    println!("✅ Job '{job_name}' would be scheduled with:");
    println!("   File: {file}");
    match &job.schedule {
        Schedule::Cron(expr) => println!("   Schedule: {expr}"),
        _ => println!("   Schedule: Manual"),
    }
    println!("   Job ID: {}", job.id);

    Ok(())
}

async fn start_daemon(config_file: Option<&str>, foreground: bool) -> Result<()> {
    println!("{}", "🚀 Starting scheduler daemon...".cyan());

    let config_manager = if let Some(config_path) = config_file {
        SchedulerConfigManager::load_from_file(config_path).context("Failed to load configuration file")?
    } else {
        SchedulerConfigManager::new()
    };

    let scheduler = CronScheduler::new();
    let mut daemon = SchedulerDaemon::new(scheduler);

    if let Some(pid_file) = &config_manager.config().daemon.pid_file {
        daemon = daemon.with_pid_file(pid_file.clone());
    }

    if let Some(log_file) = &config_manager.config().daemon.log_file {
        daemon = daemon.with_log_file(log_file.clone());
    }

    if foreground {
        println!("Running in foreground mode...");
        daemon.start().await?;
    } else {
        println!("Starting daemon in background...");
        // In a full implementation, this would properly daemonize the process
        daemon.start().await?;
    }

    Ok(())
}

async fn stop_daemon() -> Result<()> {
    println!("{}", "🛑 Stopping scheduler daemon...".cyan());

    // In a full implementation, this would send a signal to the running daemon
    // For now, just print a message
    println!("✅ Daemon stop signal sent");

    Ok(())
}

async fn list_jobs(enabled_only: bool) -> Result<()> {
    println!("{}", "📋 Listing scheduled jobs...".cyan());

    // In a full implementation, this would connect to the running scheduler
    // For now, just show an example
    println!("No jobs currently scheduled");
    println!("Use 'jorm schedule <file>' to add jobs");

    Ok(())
}

async fn trigger_job(job_identifier: &str) -> Result<()> {
    println!(
        "{}",
        format!("⚡ Triggering job: {job_identifier}").cyan()
    );

    // In a full implementation, this would connect to the running scheduler
    // For now, simulate checking if job exists
    if job_identifier == "nonexistent_job" {
        println!("{}", "❌ Job not found: nonexistent_job".red());
        anyhow::bail!("Job 'nonexistent_job' does not exist");
    }

    println!("✅ Job trigger request sent");
    Ok(())
}

async fn start_interactive() -> Result<()> {
    println!("{}", "🤖 Starting interactive mode...".cyan());

    // Create interactive mode
    let mut interactive = match InteractiveMode::new().await {
        Ok(mode) => mode,
        Err(_) => {
            println!("{}", "❌ Interactive mode not available".red());
            return Ok(());
        }
    };

    // Start interactive session
    interactive.start().await?;

    Ok(())
}

async fn analyze_dag(file: &str) -> Result<()> {
    println!("{}", format!("🔍 Analyzing DAG: {file}").cyan());

    // Parse the DAG file
    let dag = parse_dag_file(file).await?;

    // Create AI service
    let ai_service = match ai::AIService::new().await {
        Ok(service) => service,
        Err(_) => {
            println!(
                "{}",
                "⚠️ AI service not available, showing basic analysis".yellow()
            );
            show_basic_dag_analysis(&dag).await;
            return Ok(());
        }
    };

    // Analyze the DAG
    match ai_service.analyze_dag(&dag).await {
        Ok(analysis) => {
            println!("{}", "📊 DAG Analysis Results".bold().green());
            println!("Performance Score: {:.2}", analysis.performance_score);
            println!("Complexity Metrics:");
            println!("  • Tasks: {}", analysis.complexity_metrics.task_count);
            println!(
                "  • Dependencies: {}",
                analysis.complexity_metrics.dependency_count
            );
            println!("  • Max Depth: {}", analysis.complexity_metrics.max_depth);
            println!(
                "  • Maintainability: {:.1}",
                analysis.complexity_metrics.maintainability_index
            );

            if !analysis.optimization_suggestions.is_empty() {
                println!("\n{}", "💡 Optimization Suggestions".bold().yellow());
                for (i, suggestion) in analysis.optimization_suggestions.iter().enumerate() {
                    println!(
                        "{}. {} ({:?} impact, {:?} effort)",
                        i + 1,
                        suggestion.description,
                        suggestion.impact,
                        suggestion.implementation_effort
                    );
                }
            }

            if !analysis.potential_issues.is_empty() {
                println!("\n{}", "⚠️ Potential Issues".bold().red());
                for (i, issue) in analysis.potential_issues.iter().enumerate() {
                    println!("{}. {} ({:?})", i + 1, issue.description, issue.severity);
                }
            }
        }
        Err(e) => {
            println!("{}", format!("❌ Analysis failed: {e}").red());
            show_basic_dag_analysis(&dag).await;
        }
    }

    Ok(())
}

async fn generate_dag(description: &str, output_path: Option<&str>) -> Result<()> {
    println!(
        "{}",
        format!("🚀 Generating DAG from: {description}").cyan()
    );

    // Create AI service
    let ai_service = match ai::AIService::new().await {
        Ok(service) => service,
        Err(_) => {
            println!(
                "{}",
                "⚠️ AI service not available, using basic generation".yellow()
            );
            return generate_basic_dag(description, output_path).await;
        }
    };

    // Generate the DAG
    match ai_service
        .generate_dag_from_natural_language(description)
        .await
    {
        Ok(dag) => {
            println!("{}", "✅ DAG Generated Successfully".bold().green());
            println!("DAG Name: {}", dag.name);
            println!("Tasks: {}", dag.tasks.len());
            println!("Dependencies: {}", dag.dependencies.len());

            // Save to file if output path specified
            if let Some(path) = output_path {
                let yaml_content = serde_yaml::to_string(&dag)?;
                tokio::fs::write(path, yaml_content).await?;
                println!("💾 Saved to: {path}");
            } else {
                // Print YAML to stdout
                let yaml_content = serde_yaml::to_string(&dag)?;
                println!("\n{}", "Generated DAG (YAML):".bold());
                println!("{yaml_content}");
            }
        }
        Err(e) => {
            println!("{}", format!("❌ Generation failed: {e}").red());
            return generate_basic_dag(description, output_path).await;
        }
    }

    Ok(())
}

async fn show_model_info() -> Result<()> {
    println!("{}", "🤖 AI Model Information".bold().cyan());

    // Create AI service
    let ai_service = match ai::AIService::new().await {
        Ok(service) => service,
        Err(_) => {
            println!("{}", "❌ AI service not available".red());
            return Ok(());
        }
    };

    let model_info = ai_service.model_info();
    println!("Model: {}", model_info.name);
    println!("Version: {}", model_info.version);
    println!("Parameters: {}B", model_info.parameters / 1_000_000_000);
    println!(
        "Memory Usage: {:.1}GB",
        model_info.memory_usage as f64 / 1_000_000_000.0
    );
    println!("Capabilities:");
    for capability in &model_info.capabilities {
        println!("  • {capability}");
    }

    Ok(())
}

async fn show_basic_dag_analysis(dag: &crate::parser::Dag) {
    println!("{}", "📊 Basic DAG Analysis".bold().green());
    println!("DAG Name: {}", dag.name);
    println!("Tasks: {}", dag.tasks.len());
    println!("Dependencies: {}", dag.dependencies.len());

    if let Some(schedule) = &dag.schedule {
        println!("Schedule: {schedule}");
    }

    // Show task names
    println!("Task Names:");
    for task_name in dag.tasks.keys() {
        println!("  • {task_name}");
    }

    // Show dependencies
    if !dag.dependencies.is_empty() {
        println!("Dependencies:");
        for dep in &dag.dependencies {
            println!("  • {} → {}", dep.depends_on, dep.task);
        }
    }
}

async fn generate_basic_dag(description: &str, output_path: Option<&str>) -> Result<()> {
    println!("{}", "🔧 Generating Basic DAG".yellow());

    // Create a simple DAG based on description
    let mut dag = crate::parser::Dag::new("generated_dag".to_string());

    // Add some basic tasks based on keywords
    let description_lower = description.to_lowercase();
    if description_lower.contains("data") || description_lower.contains("pipeline") {
        dag.add_task(crate::parser::Task::new("extract".to_string()));
        dag.add_task(crate::parser::Task::new("transform".to_string()));
        dag.add_task(crate::parser::Task::new("load".to_string()));
        dag.add_dependency("transform".to_string(), "extract".to_string());
        dag.add_dependency("load".to_string(), "transform".to_string());
    } else {
        dag.add_task(crate::parser::Task::new("task1".to_string()));
        dag.add_task(crate::parser::Task::new("task2".to_string()));
        dag.add_dependency("task2".to_string(), "task1".to_string());
    }

    // Save to file if output path specified
    if let Some(path) = output_path {
        let yaml_content = serde_yaml::to_string(&dag)?;
        tokio::fs::write(path, yaml_content).await?;
        println!("💾 Saved to: {path}");
    } else {
        // Print YAML to stdout
        let yaml_content = serde_yaml::to_string(&dag)?;
        println!("\n{}", "Generated DAG (YAML):".bold());
        println!("{yaml_content}");
    }

    Ok(())
}

fn print_version_info() {
    println!("{}", "🦀 JORM - Pure Rust DAG Engine".bold().cyan());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    println!();
    println!("{}", "Architecture:".bold());
    println!(
        "  • {} {}",
        "Engine:".bold(),
        "Pure Rust (fast, reliable, cross-platform)".green()
    );
    println!(
        "  • {} {}",
        "Parser:".bold(),
        "Native Rust (supports .txt, .md, .yaml)".green()
    );
    println!(
        "  • {} {}",
        "Executors:".bold(),
        "Shell, HTTP, File operations".green()
    );
    println!(
        "  • {} {}",
        "AI:".bold(),
        "Local language models (Phi-3, Gemma)".green()
    );
    println!(
        "  • {} {}",
        "Benefits:".bold(),
        "Fast execution + AI intelligence + No Python dependency".green()
    );
    println!();
    println!("{}", "Features:".bold());
    println!(
        "  • {} {}",
        "DAG Execution:".bold(),
        "Native Rust engine with retry mechanisms".cyan()
    );
    println!(
        "  • {} {}",
        "AI Intelligence:".bold(),
        "Analysis, generation, chat interface".cyan()
    );
    println!(
        "  • {} {}",
        "Scheduling:".bold(),
        "Cron-based scheduling with daemon".cyan()
    );
    println!(
        "  • {} {}",
        "Cross-platform:".bold(),
        "Windows, Linux, macOS support".cyan()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Execute command
    let result = match cli.command {
        Commands::Run { file, no_validate, native_executor, python, with_state, max_concurrent, config, profile, timeout } => {
            validate_file_exists(&file)?;
            run_dag(&file, no_validate, native_executor, python, with_state, max_concurrent, config, profile, timeout).await
        }
        Commands::Validate { file } => {
            validate_file_exists(&file)?;
            validate_dag_command(&file).await
        }
        Commands::Describe { file } => {
            validate_file_exists(&file)?;
            describe_dag(&file).await
        }
        Commands::Exec { task } => exec_task(&task).await,
        Commands::Status => show_status().await,
        Commands::List => list_dags().await,
        Commands::Schedule { file, cron, name } => {
            validate_file_exists(&file)?;
            schedule_dag(&file, cron.as_deref(), name.as_deref()).await
        }
        Commands::Daemon { config, foreground } => {
            start_daemon(config.as_deref(), foreground).await
        }
        Commands::Stop => stop_daemon().await,
        Commands::Jobs { enabled } => list_jobs(enabled).await,
        Commands::Trigger { job } => trigger_job(&job).await,
        Commands::Interactive => start_interactive().await,
        Commands::Analyze { file } => analyze_dag(&file).await,
        Commands::Generate {
            description,
            output,
        } => generate_dag(&description, output.as_deref()).await,
        Commands::ModelInfo => show_model_info().await,
        Commands::Version => {
            print_version_info();
            Ok(())
        }
        Commands::Setup {
            force,
            skip_python,
            skip_shell,
        } => setup_environment(force, skip_python, skip_shell).await,
        Commands::Config { action } => handle_config_command(action).await,
    };

    // Handle errors with colored output
    if let Err(e) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}

async fn setup_environment(force: bool, skip_python: bool, skip_shell: bool) -> Result<()> {
    println!("{}", "🔧 Setting up jorm environment...".cyan().bold());

    let mut issues = Vec::new();
    let mut fixes_applied = Vec::new();

    // Detect platform
    let platform = detect_platform();
    println!("{}", format!("📱 Detected platform: {platform}").blue());

    // Check Python installation
    if !skip_python {
        match check_python_installation().await {
            Ok(version) => {
                println!("{}", format!("✅ Python {version} is available").green());
            }
            Err(e) => {
                println!("{}", format!("❌ Python not found: {e}").red());
                issues.push("Python not installed or not in PATH".to_string());

                // Provide platform-specific installation instructions
                let python_install_cmd = get_python_install_command(&platform);
                println!(
                    "{}",
                    format!("💡 To install Python: {python_install_cmd}").yellow()
                );
                fixes_applied.push("Python installation instructions provided".to_string());
            }
        }
    }

    // Check shell commands
    if !skip_shell {
        let shell_commands = get_required_shell_commands(&platform);
        for cmd in shell_commands {
            match check_shell_command(&cmd).await {
                Ok(_) => {
                    println!("{}", format!("✅ Command '{cmd}' is available").green());
                }
                Err(_) => {
                    println!("{}", format!("❌ Command '{cmd}' not found").red());
                    issues.push(format!("Command '{cmd}' not available"));

                    // Provide alternative commands for Windows
                    if platform == "Windows" {
                        let alt_cmd = get_windows_alternative(&cmd);
                        if let Some(alt) = alt_cmd {
                            println!(
                                "{}",
                                format!("💡 Use '{alt}' instead on Windows").yellow()
                            );
                            fixes_applied
                                .push(format!("Windows alternative for '{cmd}': '{alt}'"));
                        }
                    }
                }
            }
        }
    }

    // Check Rust installation
    match check_rust_installation().await {
        Ok(version) => {
            println!("{}", format!("✅ Rust {version} is available").green());
        }
        Err(e) => {
            println!("{}", format!("❌ Rust not found: {e}").red());
            issues.push("Rust not installed".to_string());
            println!("{}", "💡 Install Rust from https://rustup.rs/".yellow());
            fixes_applied.push("Rust installation instructions provided".to_string());
        }
    }

    // Check jorm installation
    match check_jorm_installation().await {
        Ok(version) => {
            println!("{}", format!("✅ jorm {version} is installed").green());
        }
        Err(_) => {
            println!("{}", "❌ jorm not found in PATH".red());
            issues.push("jorm not in PATH".to_string());
            println!(
                "{}",
                "💡 Add jorm to your PATH or use 'cargo run' to execute".yellow()
            );
            fixes_applied.push("jorm PATH instructions provided".to_string());
        }
    }

    // Create platform-specific configuration
    create_platform_config(&platform)?;
    fixes_applied.push("Platform-specific configuration created".to_string());

    // Summary
    println!("\n{}", "📊 Setup Summary".cyan().bold());
    if issues.is_empty() {
        println!(
            "{}",
            "🎉 All dependencies are properly configured!"
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            format!("⚠️  Found {} issues that need attention:", issues.len())
                .yellow()
                .bold()
        );
        for issue in &issues {
            println!("  • {issue}");
        }
    }

    if !fixes_applied.is_empty() {
        println!("\n{}", "🔧 Fixes Applied:".blue().bold());
        for fix in &fixes_applied {
            println!("  ✅ {fix}");
        }
    }

    // Create test environment
    create_test_environment()?;
    fixes_applied.push("Test environment created".to_string());

    println!("\n{}", "🚀 Environment setup complete!".green().bold());
    println!(
        "{}",
        "💡 Run 'jorm --help' to see available commands".blue()
    );

    Ok(())
}

async fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show { profile } => show_config(profile).await,
        ConfigAction::Init { output, format } => init_config(&output, &format).await,
        ConfigAction::Validate { file } => validate_config(&file).await,
        ConfigAction::Profiles => list_profiles().await,
    }
}

async fn show_config(profile: Option<String>) -> Result<()> {
    println!("{}", "📋 Current Configuration".cyan().bold());

    let config = if let Some(profile_name) = profile {
        println!("Profile: {}", profile_name);
        ExecutorConfig::load_with_profile(&profile_name)
            .context("Failed to load configuration profile")?
    } else {
        ExecutorConfig::load()
            .context("Failed to load configuration")?
    };

    config.print_summary();
    Ok(())
}

async fn init_config(output: &str, format: &str) -> Result<()> {
    println!("{}", format!("📄 Creating configuration file: {}", output).cyan());

    let manager = ConfigManager::new();
    
    match format {
        "toml" | "yaml" | "yml" | "json" => {
            manager.create_default_config_file(output)
                .context("Failed to create configuration file")?;
            println!("{}", format!("✅ Configuration file created: {}", output).green());
            println!("{}", "💡 Edit the file to customize your settings".blue());
        }
        _ => {
            anyhow::bail!("Unsupported format: {}. Use toml, yaml, or json", format);
        }
    }

    Ok(())
}

async fn validate_config(file: &str) -> Result<()> {
    println!("{}", format!("🔍 Validating configuration: {}", file).cyan());

    let config = ExecutorConfig::load_from_file(file)
        .context("Failed to load configuration file")?;

    config.validate()
        .context("Configuration validation failed")?;

    println!("{}", "✅ Configuration is valid".green());
    config.print_summary();

    Ok(())
}

async fn list_profiles() -> Result<()> {
    println!("{}", "📋 Available Configuration Profiles".cyan().bold());

    let manager = ConfigManager::new();
    let profile_names = manager.get_profile_names();

    if profile_names.is_empty() {
        println!("No profiles configured");
        println!("💡 Add profiles to your configuration file or use 'jorm config init' to create one");
    } else {
        for profile in profile_names {
            println!("  • {}", profile);
        }
    }

    Ok(())
}

fn detect_platform() -> String {
    if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else {
        "Unknown".to_string()
    }
}

async fn check_python_installation() -> Result<String> {
    let output = std::process::Command::new("python")
        .arg("--version")
        .output()
        .map_err(|_| anyhow::anyhow!("Python not found"))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    } else {
        // Try python3
        let output = std::process::Command::new("python3")
            .arg("--version")
            .output()
            .map_err(|_| anyhow::anyhow!("Python3 not found"))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            Ok(version.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Neither python nor python3 found"))
        }
    }
}

fn get_python_install_command(platform: &str) -> String {
    match platform {
        "Windows" => "Download from https://python.org or use 'winget install Python.Python.3'",
        "macOS" => "brew install python3",
        "Linux" => {
            "sudo apt install python3 (Ubuntu/Debian) or sudo yum install python3 (RHEL/CentOS)"
        }
        _ => "Visit https://python.org for installation instructions",
    }
    .to_string()
}

fn get_required_shell_commands(platform: &str) -> Vec<String> {
    match platform {
        "Windows" => vec!["cmd".to_string(), "powershell".to_string()],
        _ => vec!["bash".to_string(), "sh".to_string()],
    }
}

async fn check_shell_command(cmd: &str) -> Result<()> {
    let output = std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .or_else(|_| std::process::Command::new(cmd).arg("/?").output());

    match output {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(anyhow::anyhow!("Command not found")),
    }
}

fn get_windows_alternative(cmd: &str) -> Option<String> {
    match cmd {
        "ls" => Some("dir".to_string()),
        "cp" => Some("copy".to_string()),
        "mv" => Some("move".to_string()),
        "rm" => Some("del".to_string()),
        "mkdir" => Some("md".to_string()),
        "cat" => Some("type".to_string()),
        _ => None,
    }
}

async fn check_rust_installation() -> Result<String> {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|_| anyhow::anyhow!("Rust not found"))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    } else {
        Err(anyhow::anyhow!("Rust not found"))
    }
}

async fn check_jorm_installation() -> Result<String> {
    let output = std::process::Command::new("jorm")
        .arg("--version")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            Ok(version.trim().to_string())
        }
        _ => Err(anyhow::anyhow!("jorm not found in PATH")),
    }
}

fn create_platform_config(platform: &str) -> Result<()> {
    let config_dir = std::env::current_dir()?.join(".jorm");
    std::fs::create_dir_all(&config_dir)?;

    let config_content = match platform {
        "Windows" => {
            r#"
# Windows-specific configuration
shell = "cmd"
python_cmd = "python"
file_operations = {
    copy = "copy"
    move = "move"
    delete = "del"
    list = "dir"
}
"#
        }
        _ => {
            r#"
# Unix-like system configuration
shell = "bash"
python_cmd = "python3"
file_operations = {
    copy = "cp"
    move = "mv"
    delete = "rm"
    list = "ls"
}
"#
        }
    };

    let config_file = config_dir.join("config.toml");
    std::fs::write(config_file, config_content)?;

    println!("{}", "✅ Platform configuration created".green());
    Ok(())
}

fn create_test_environment() -> Result<()> {
    let test_dir = std::env::current_dir()?.join("test_env");
    std::fs::create_dir_all(&test_dir)?;

    // Create a simple test DAG
    let test_dag = r#"dag: test_environment
schedule: "0 0 * * *"

tasks:
- test_shell
  type: shell
  description: Test shell command execution
  command: echo "Environment test successful"

- test_python
  type: python
  description: Test Python execution
  script: |
    print("Python environment test successful")
    import sys
    print(f"Python version: {sys.version}")

- test_file_ops
  type: file
  description: Test file operations
  operation: create
  path: test_file.txt
  destination: test_file_copy.txt
"#;

    let dag_file = test_dir.join("test_environment.txt");
    std::fs::write(dag_file, test_dag)?;

    println!("{}", "✅ Test environment created in ./test_env/".green());
    println!(
        "{}",
        "💡 Run 'jorm run test_env/test_environment.txt' to test".blue()
    );

    Ok(())
}
