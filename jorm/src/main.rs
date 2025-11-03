use clap::{Parser, Subcommand};
use jorm::{JormEngine, Result, DagParser, TaskType};
use jorm::server::http::HttpServer;
use jorm::scheduler::{Daemon, Schedule};
use std::path::PathBuf;
use std::process;
use std::sync::Arc;


#[derive(Parser)]
#[command(name = "jorm")]
#[command(about = "A simplified DAG execution engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a DAG from a file
    Execute {
        /// Path to the DAG file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Generate and execute a DAG from natural language
    Generate {
        /// Natural language description of the workflow
        #[arg(short, long)]
        description: String,
        /// Preview the generated DAG without executing
        #[arg(short, long)]
        preview: bool,
    },
    /// Validate a DAG file syntax
    Validate {
        /// Path to the DAG file to validate
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Start the HTTP server
    Server {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Authentication token (can also be set via JORM_AUTH_TOKEN env var)
        #[arg(short, long)]
        auth_token: Option<String>,
    },
    /// Daemon operations
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start {
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
        /// Schedules configuration file
        #[arg(short, long)]
        schedules: Option<PathBuf>,
        /// Log file for daemon output
        #[arg(short, long)]
        log_file: Option<PathBuf>,
    },
    /// Stop the daemon
    Stop {
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
    },
    /// Check daemon status
    Status {
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
    },
    /// Add a schedule to the daemon
    AddSchedule {
        /// Schedule ID
        #[arg(short, long)]
        id: String,
        /// Cron expression
        #[arg(short, long)]
        cron: String,
        /// DAG file to execute
        #[arg(short, long)]
        dag_file: PathBuf,
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
    },
    /// Remove a schedule from the daemon
    RemoveSchedule {
        /// Schedule ID to remove
        #[arg(short, long)]
        id: String,
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
    },
    /// List all schedules
    ListSchedules {
        /// State file for daemon persistence
        #[arg(long, default_value = ".jorm/daemon.state")]
        state_file: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    let exit_code = match run_command(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    };
    
    process::exit(exit_code);
}

async fn run_command(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Execute { file } => execute_dag_file(file).await,
        Commands::Generate { description, preview } => generate_dag(description, preview).await,
        Commands::Validate { file } => validate_dag_file(file).await,
        Commands::Server { port, auth_token } => start_http_server(port, auth_token).await,
        Commands::Daemon { action } => handle_daemon_action(action).await,
    }
}

async fn execute_dag_file(file: PathBuf) -> Result<i32> {
    println!("🚀 Executing DAG file: {}", file.display());
    
    let engine = JormEngine::new().await?;
    let file_path = file.to_str().ok_or_else(|| {
        jorm::JormError::FileError("Invalid file path".to_string())
    })?;
    
    match engine.execute_from_file(file_path).await {
        Ok(result) => {
            if result.success {
                println!("✅ Execution completed successfully!");
                println!("📊 Results:");
                for task_result in &result.task_results {
                    if task_result.success {
                        println!("  ✅ {}: {}", task_result.task_name, task_result.output);
                    } else {
                        println!("  ❌ {}: {}", task_result.task_name, 
                                task_result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                    }
                }
                Ok(0)
            } else {
                println!("❌ Execution failed: {}", result.message);
                println!("📊 Results:");
                for task_result in &result.task_results {
                    if task_result.success {
                        println!("  ✅ {}: {}", task_result.task_name, task_result.output);
                    } else {
                        println!("  ❌ {}: {}", task_result.task_name, 
                                task_result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                    }
                }
                Ok(1)
            }
        }
        Err(e) => {
            println!("❌ Failed to execute DAG: {}", e);
            Ok(1)
        }
    }
}

async fn generate_dag(description: String, preview: bool) -> Result<i32> {
    println!("🤖 Generating DAG from natural language...");
    println!("📝 Description: {}", description);
    
    let engine = JormEngine::new().await?;
    
    if preview {
        match engine.generate_dag_from_nl(&description).await {
            Ok(dag_content) => {
                println!("✅ Generated DAG preview:");
                println!("─────────────────────────────────────");
                println!("{}", dag_content);
                println!("─────────────────────────────────────");
                println!("💡 Use without --preview flag to execute directly");
                Ok(0)
            }
            Err(e) => {
                println!("❌ Failed to generate DAG: {}", e);
                Ok(1)
            }
        }
    } else {
        match engine.execute_from_natural_language(&description).await {
            Ok(result) => {
                if result.success {
                    println!("✅ Generation and execution completed successfully!");
                    println!("📊 Results:");
                    for task_result in &result.task_results {
                        if task_result.success {
                            println!("  ✅ {}: {}", task_result.task_name, task_result.output);
                        } else {
                            println!("  ❌ {}: {}", task_result.task_name, 
                                    task_result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                        }
                    }
                    Ok(0)
                } else {
                    println!("❌ Execution failed: {}", result.message);
                    println!("📊 Results:");
                    for task_result in &result.task_results {
                        if task_result.success {
                            println!("  ✅ {}: {}", task_result.task_name, task_result.output);
                        } else {
                            println!("  ❌ {}: {}", task_result.task_name, 
                                    task_result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                        }
                    }
                    Ok(1)
                }
            }
            Err(e) => {
                println!("❌ Failed to generate and execute DAG: {}", e);
                Ok(1)
            }
        }
    }
}

async fn validate_dag_file(file: PathBuf) -> Result<i32> {
    println!("🔍 Validating DAG file: {}", file.display());
    
    let parser = DagParser::new();
    let file_path = file.to_str().ok_or_else(|| {
        jorm::JormError::FileError("Invalid file path".to_string())
    })?;
    
    match parser.parse_file(file_path).await {
        Ok(dag) => {
            match dag.validate() {
                Ok(_) => {
                    println!("✅ DAG file is valid!");
                    println!("📊 Summary:");
                    println!("  📝 DAG name: {}", dag.name);
                    println!("  🔢 Total tasks: {}", dag.tasks.len());
                    println!("  🔗 Dependencies: {}", dag.dependencies.len());
                    
                    // Show task types summary
                    let mut task_types = std::collections::HashMap::new();
                    for task in dag.tasks.values() {
                        let task_type = match &task.task_type {
                            TaskType::Shell { .. } => "Shell",
                            TaskType::Http { .. } => "HTTP",
                            TaskType::Python { .. } => "Python",
                            TaskType::Rust { .. } => "Rust",
                            TaskType::FileCopy { .. } => "File Copy",
                            TaskType::FileMove { .. } => "File Move",
                            TaskType::FileDelete { .. } => "File Delete",
                            TaskType::Jorm { .. } => "Jorm",
                        };
                        *task_types.entry(task_type).or_insert(0) += 1;
                    }
                    
                    println!("  📋 Task types:");
                    for (task_type, count) in task_types {
                        println!("    - {}: {}", task_type, count);
                    }
                    
                    Ok(0)
                }
                Err(e) => {
                    println!("❌ DAG validation failed: {}", e);
                    Ok(1)
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to parse DAG file: {}", e);
            Ok(1)
        }
    }
}

async fn start_http_server(port: u16, auth_token: Option<String>) -> Result<i32> {
    println!("🌐 Starting HTTP server on port {}", port);
    
    let engine = Arc::new(JormEngine::new().await?);
    let server = if let Some(token) = auth_token {
        HttpServer::with_auth_token(engine, port, token)
    } else {
        HttpServer::new(engine, port)
    };
    
    match server.start().await {
        Ok(_) => {
            println!("✅ HTTP server started successfully");
            Ok(0)
        }
        Err(e) => {
            println!("❌ Failed to start HTTP server: {}", e);
            Ok(1)
        }
    }
}

async fn handle_daemon_action(action: DaemonAction) -> Result<i32> {
    match action {
        DaemonAction::Start { state_file, schedules, log_file } => {
            println!("🚀 Starting Jorm daemon...");
            
            let engine = Arc::new(JormEngine::new().await?);
            let mut daemon = Daemon::new(engine, state_file, schedules, log_file);
            
            match daemon.start().await {
                Ok(_) => {
                    println!("✅ Daemon stopped gracefully");
                    Ok(0)
                }
                Err(e) => {
                    println!("❌ Daemon error: {}", e);
                    Ok(1)
                }
            }
        }
        DaemonAction::Stop { state_file } => {
            println!("🛑 Stopping Jorm daemon...");
            
            let engine = Arc::new(JormEngine::new().await?);
            let mut daemon = Daemon::new(engine, state_file, None, None);
            
            match daemon.stop().await {
                Ok(_) => {
                    println!("✅ Daemon stopped successfully");
                    Ok(0)
                }
                Err(e) => {
                    println!("❌ Failed to stop daemon: {}", e);
                    Ok(1)
                }
            }
        }
        DaemonAction::Status { state_file } => {
            let engine = Arc::new(JormEngine::new().await?);
            let daemon = Daemon::new(engine, state_file, None, None);
            
            match daemon.status().await {
                Ok(state) => {
                    if let Some(pid) = state.pid {
                        println!("✅ Daemon is running (PID: {})", pid);
                        if let Some(started_at) = state.started_at {
                            println!("   Started at: {}", started_at.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        if let Some(schedules_file) = state.schedules_file {
                            println!("   Schedules file: {}", schedules_file);
                        }
                        if let Some(log_file) = state.log_file {
                            println!("   Log file: {}", log_file);
                        }
                    } else {
                        println!("❌ Daemon is not running");
                    }
                    Ok(0)
                }
                Err(e) => {
                    println!("❌ Failed to get daemon status: {}", e);
                    Ok(1)
                }
            }
        }
        DaemonAction::AddSchedule { id, cron, dag_file, state_file } => {
            println!("📅 Adding schedule: {} -> {}", id, dag_file.display());
            
            let dag_file_str = dag_file.to_str().ok_or_else(|| {
                jorm::JormError::FileError("Invalid DAG file path".to_string())
            })?;
            
            match Schedule::new(id.clone(), cron, dag_file_str.to_string()) {
                Ok(schedule) => {
                    let engine = Arc::new(JormEngine::new().await?);
                    let mut daemon = Daemon::new(engine, state_file, None, None);
                    
                    match daemon.add_schedule(schedule).await {
                        Ok(_) => {
                            println!("✅ Schedule '{}' added successfully", id);
                            Ok(0)
                        }
                        Err(e) => {
                            println!("❌ Failed to add schedule: {}", e);
                            Ok(1)
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Invalid schedule: {}", e);
                    Ok(1)
                }
            }
        }
        DaemonAction::RemoveSchedule { id, state_file } => {
            println!("🗑️ Removing schedule: {}", id);
            
            let engine = Arc::new(JormEngine::new().await?);
            let mut daemon = Daemon::new(engine, state_file, None, None);
            
            match daemon.remove_schedule(&id).await {
                Ok(_) => {
                    println!("✅ Schedule '{}' removed successfully", id);
                    Ok(0)
                }
                Err(e) => {
                    println!("❌ Failed to remove schedule: {}", e);
                    Ok(1)
                }
            }
        }
        DaemonAction::ListSchedules { state_file } => {
            let engine = Arc::new(JormEngine::new().await?);
            let daemon = Daemon::new(engine, state_file, None, None);
            
            let schedules = daemon.list_schedules();
            
            if schedules.is_empty() {
                println!("📅 No schedules configured");
            } else {
                println!("📅 Configured schedules:");
                for schedule in schedules {
                    let status = if schedule.enabled { "✅ Enabled" } else { "❌ Disabled" };
                    println!("  {} [{}]", schedule.id, status);
                    println!("    Cron: {}", schedule.cron_expression);
                    println!("    DAG: {}", schedule.dag_file);
                    if let Some(last) = schedule.last_execution {
                        println!("    Last execution: {}", last.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                    if let Some(next) = schedule.next_execution {
                        println!("    Next execution: {}", next.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                    println!();
                }
            }
            
            Ok(0)
        }
    }
}