//! Output formatting and progress indicators for native executor
//!
//! This module provides enhanced output formatting, colored output, and progress
//! indicators for the native executor to ensure compatibility with existing
//! Python engine output while adding improved user experience.

use crate::executor::{ExecutionResult, ExecutionStatus, TaskResult, TaskStatus, ExecutionMetrics};
use colored::*;
use std::io::{self, Write};
use std::time::Duration;

/// Output formatter for execution results
pub struct OutputFormatter {
    /// Whether to use colored output
    colored: bool,
    /// Verbosity level (0 = minimal, 1 = normal, 2 = verbose, 3 = debug)
    verbosity: u8,
    /// Whether to show progress indicators
    show_progress: bool,
}

impl OutputFormatter {
    /// Create a new output formatter with default settings
    pub fn new() -> Self {
        Self {
            colored: true,
            verbosity: 1,
            show_progress: true,
        }
    }

    /// Create a formatter with custom settings
    pub fn with_settings(colored: bool, verbosity: u8, show_progress: bool) -> Self {
        Self {
            colored,
            verbosity,
            show_progress,
        }
    }

    /// Set colored output
    pub fn set_colored(&mut self, colored: bool) {
        self.colored = colored;
    }

    /// Set verbosity level
    pub fn set_verbosity(&mut self, verbosity: u8) {
        self.verbosity = verbosity;
    }

    /// Set progress indicators
    pub fn set_show_progress(&mut self, show_progress: bool) {
        self.show_progress = show_progress;
    }

    /// Format and print DAG execution start
    pub fn print_dag_start(&self, dag_name: &str, execution_id: &str) {
        if self.colored {
            println!("{}", format!("🚀 Starting DAG execution: {} (ID: {})", dag_name, execution_id).cyan());
        } else {
            println!("Starting DAG execution: {} (ID: {})", dag_name, execution_id);
        }
    }

    /// Format and print executor selection
    pub fn print_executor_selection(&self, is_native: bool) {
        if is_native {
            if self.colored {
                println!("{}", "🦀 Using native Rust executor".cyan());
            } else {
                println!("Using native Rust executor");
            }
        } else {
            if self.colored {
                println!("{}", "🐍 Using Python executor".cyan());
            } else {
                println!("Using Python executor");
            }
        }
    }

    /// Format and print task start
    pub fn print_task_start(&self, task_name: &str) {
        if !self.show_progress {
            return;
        }

        if self.colored {
            println!("{}", format!("🔄 Starting task: {}", task_name).blue());
        } else {
            println!("Starting task: {}", task_name);
        }
    }

    /// Format and print task completion
    pub fn print_task_completion(&self, task_name: &str, result: &TaskResult) {
        if !self.show_progress {
            return;
        }

        match result.status {
            TaskStatus::Success => {
                if self.colored {
                    println!("{}", format!("✅ Task '{}' completed successfully", task_name).green());
                } else {
                    println!("Task '{}' completed successfully", task_name);
                }

                // Show task output in verbose mode
                if self.verbosity >= 2 && !result.stdout.is_empty() {
                    println!("   Output: {}", result.stdout.trim());
                }
            }
            TaskStatus::Failed => {
                if self.colored {
                    println!("{}", format!("❌ Task '{}' failed", task_name).red());
                } else {
                    println!("Task '{}' failed", task_name);
                }

                // Show error details
                if let Some(error) = &result.error_message {
                    if self.colored {
                        println!("   {}: {}", "Error".red().bold(), error);
                    } else {
                        println!("   Error: {}", error);
                    }
                }

                if !result.stderr.is_empty() {
                    if self.colored {
                        println!("   {}: {}", "Stderr".red(), result.stderr.trim());
                    } else {
                        println!("   Stderr: {}", result.stderr.trim());
                    }
                }

                if let Some(exit_code) = result.exit_code {
                    if self.colored {
                        println!("   {}: {}", "Exit code".red(), exit_code);
                    } else {
                        println!("   Exit code: {}", exit_code);
                    }
                }
            }
            TaskStatus::Timeout => {
                if self.colored {
                    println!("{}", format!("⏰ Task '{}' timed out", task_name).yellow());
                } else {
                    println!("Task '{}' timed out", task_name);
                }
            }
            TaskStatus::Skipped => {
                if self.colored {
                    println!("{}", format!("⏭️ Task '{}' skipped", task_name).yellow());
                } else {
                    println!("Task '{}' skipped", task_name);
                }
            }
            TaskStatus::Cancelled => {
                if self.colored {
                    println!("{}", format!("🚫 Task '{}' cancelled", task_name).yellow());
                } else {
                    println!("Task '{}' cancelled", task_name);
                }
            }
            _ => {
                if self.colored {
                    println!("{}", format!("🔄 Task '{}' status: {:?}", task_name, result.status).blue());
                } else {
                    println!("Task '{}' status: {:?}", task_name, result.status);
                }
            }
        }

        // Show timing information in verbose mode
        if self.verbosity >= 2 {
            println!("   Duration: {:.2}s", result.duration.as_secs_f64());
            if result.retry_count > 0 {
                println!("   Retries: {}", result.retry_count);
            }
        }
    }

    /// Format and print execution summary
    pub fn print_execution_summary(&self, result: &ExecutionResult) {
        println!();

        match result.status {
            ExecutionStatus::Success => {
                if self.colored {
                    println!("{}", "✅ DAG execution completed successfully".green().bold());
                } else {
                    println!("DAG execution completed successfully");
                }
            }
            ExecutionStatus::Failed => {
                if self.colored {
                    println!("{}", "❌ DAG execution failed".red().bold());
                } else {
                    println!("DAG execution failed");
                }
            }
            ExecutionStatus::PartialSuccess => {
                if self.colored {
                    println!("{}", "⚠️ DAG execution partially successful".yellow().bold());
                } else {
                    println!("DAG execution partially successful");
                }
            }
            ExecutionStatus::Timeout => {
                if self.colored {
                    println!("{}", "⏰ DAG execution timed out".yellow().bold());
                } else {
                    println!("DAG execution timed out");
                }
            }
            ExecutionStatus::Cancelled => {
                if self.colored {
                    println!("{}", "🚫 DAG execution cancelled".yellow().bold());
                } else {
                    println!("DAG execution cancelled");
                }
            }
            _ => {
                if self.colored {
                    println!("{}", format!("🔄 DAG execution status: {:?}", result.status).blue());
                } else {
                    println!("DAG execution status: {:?}", result.status);
                }
            }
        }

        // Print execution metrics
        self.print_execution_metrics(&result.metrics);

        // Print timing information
        if let Some(duration) = result.total_duration {
            if self.colored {
                println!("⏱️  Total execution time: {}", format_duration(duration).cyan());
            } else {
                println!("Total execution time: {}", format_duration(duration));
            }
        }

        // Print success rate
        let success_rate = result.success_rate();
        if self.colored {
            let color = if success_rate == 100.0 {
                "green"
            } else if success_rate >= 80.0 {
                "yellow"
            } else {
                "red"
            };

            match color {
                "green" => println!("📊 Success rate: {:.1}%", success_rate.to_string().green()),
                "yellow" => println!("📊 Success rate: {:.1}%", success_rate.to_string().yellow()),
                "red" => println!("📊 Success rate: {:.1}%", success_rate.to_string().red()),
                _ => println!("📊 Success rate: {:.1}%", success_rate),
            }
        } else {
            println!("Success rate: {:.1}%", success_rate);
        }
    }

    /// Format and print execution metrics
    pub fn print_execution_metrics(&self, metrics: &ExecutionMetrics) {
        if self.colored {
            println!("{}", "📊 Execution Summary:".bold());
        } else {
            println!("Execution Summary:");
        }

        println!("  • Total tasks: {}", metrics.total_tasks);
        
        if self.colored {
            println!("  • Successful: {}", metrics.successful_tasks.to_string().green());
            if metrics.failed_tasks > 0 {
                println!("  • Failed: {}", metrics.failed_tasks.to_string().red());
            }
            if metrics.skipped_tasks > 0 {
                println!("  • Skipped: {}", metrics.skipped_tasks.to_string().yellow());
            }
        } else {
            println!("  • Successful: {}", metrics.successful_tasks);
            if metrics.failed_tasks > 0 {
                println!("  • Failed: {}", metrics.failed_tasks);
            }
            if metrics.skipped_tasks > 0 {
                println!("  • Skipped: {}", metrics.skipped_tasks);
            }
        }

        println!("  • Peak concurrency: {}", metrics.peak_concurrent_tasks);

        if let Some(avg_duration) = metrics.average_task_duration {
            println!("  • Average task time: {}", format_duration(avg_duration));
        }

        if metrics.total_retries > 0 {
            println!("  • Total retries: {}", metrics.total_retries);
        }

        if let Some(peak_memory) = metrics.peak_memory_usage {
            println!("  • Peak memory: {}", format_bytes(peak_memory));
        }
    }

    /// Format and print configuration summary
    pub fn print_configuration_summary(&self, config: &crate::executor::ExecutorConfig) {
        if self.verbosity == 0 {
            return;
        }

        if self.colored {
            println!("{}", "📋 Configuration Summary:".bold());
        } else {
            println!("Configuration Summary:");
        }

        println!("  • Max concurrent tasks: {}", config.max_concurrent_tasks);
        println!("  • Default timeout: {}s", config.default_timeout.as_secs());
        
        if config.enable_resource_throttling {
            println!("  • Resource throttling: enabled");
        }

        if let Some(retry_config) = &config.retry_config {
            println!("  • Default retries: {}", retry_config.max_attempts);
        }
    }

    /// Format and print task details (debug mode)
    pub fn print_task_details(&self, task_name: &str, result: &TaskResult) {
        if self.verbosity < 3 {
            return;
        }

        if self.colored {
            println!("{}", format!("🔍 Task Details: {}", task_name).bold());
        } else {
            println!("Task Details: {}", task_name);
        }

        println!("  • Status: {:?}", result.status);
        println!("  • Started: {}", result.started_at.format("%H:%M:%S"));
        println!("  • Completed: {}", result.completed_at.format("%H:%M:%S"));
        println!("  • Duration: {}", format_duration(result.duration));
        
        if let Some(exit_code) = result.exit_code {
            println!("  • Exit code: {}", exit_code);
        }

        if result.retry_count > 0 {
            println!("  • Retry attempts: {}", result.retry_count);
        }

        if !result.stdout.is_empty() {
            println!("  • Stdout: {}", result.stdout.trim());
        }

        if !result.stderr.is_empty() {
            println!("  • Stderr: {}", result.stderr.trim());
        }

        if let Some(error) = &result.error_message {
            println!("  • Error: {}", error);
        }

        if !result.metadata.is_empty() {
            println!("  • Metadata: {:?}", result.metadata);
        }
    }

    /// Format and print resource usage information
    pub fn print_resource_usage(&self, usage: &crate::executor::ResourceUsage) {
        if self.verbosity < 2 {
            return;
        }

        if self.colored {
            println!("{}", "📊 Resource Usage:".bold());
        } else {
            println!("Resource Usage:");
        }

        let cpu_color = if usage.cpu_percent > 80.0 { "red" } else if usage.cpu_percent > 60.0 { "yellow" } else { "green" };
        let memory_color = if usage.memory_percent > 80.0 { "red" } else if usage.memory_percent > 60.0 { "yellow" } else { "green" };

        if self.colored {
            match cpu_color {
                "red" => println!("  • CPU: {:.1}%", usage.cpu_percent.to_string().red()),
                "yellow" => println!("  • CPU: {:.1}%", usage.cpu_percent.to_string().yellow()),
                "green" => println!("  • CPU: {:.1}%", usage.cpu_percent.to_string().green()),
                _ => println!("  • CPU: {:.1}%", usage.cpu_percent),
            }

            match memory_color {
                "red" => println!("  • Memory: {:.1}%", usage.memory_percent.to_string().red()),
                "yellow" => println!("  • Memory: {:.1}%", usage.memory_percent.to_string().yellow()),
                "green" => println!("  • Memory: {:.1}%", usage.memory_percent.to_string().green()),
                _ => println!("  • Memory: {:.1}%", usage.memory_percent),
            }
        } else {
            println!("  • CPU: {:.1}%", usage.cpu_percent);
            println!("  • Memory: {:.1}%", usage.memory_percent);
        }

        println!("  • Memory used: {}", format_bytes(usage.memory_bytes));
        println!("  • Active tasks: {}", usage.active_tasks);
    }

    /// Format and print error with context
    pub fn print_error(&self, error: &anyhow::Error) {
        if self.colored {
            eprintln!("{} {}", "❌ Error:".red().bold(), error);
        } else {
            eprintln!("Error: {}", error);
        }

        // Print error chain in verbose mode
        if self.verbosity >= 2 {
            let mut source = error.source();
            let mut level = 1;
            
            while let Some(err) = source {
                if self.colored {
                    eprintln!("  {}: {}", format!("Caused by ({})", level).red(), err);
                } else {
                    eprintln!("  Caused by ({}): {}", level, err);
                }
                source = err.source();
                level += 1;
            }
        }
    }

    /// Format and print warning
    pub fn print_warning(&self, message: &str) {
        if self.colored {
            println!("{} {}", "⚠️ Warning:".yellow().bold(), message);
        } else {
            println!("Warning: {}", message);
        }
    }

    /// Format and print info message
    pub fn print_info(&self, message: &str) {
        if self.verbosity >= 1 {
            if self.colored {
                println!("{} {}", "ℹ️ Info:".blue(), message);
            } else {
                println!("Info: {}", message);
            }
        }
    }

    /// Format and print debug message
    pub fn print_debug(&self, message: &str) {
        if self.verbosity >= 3 {
            if self.colored {
                println!("{} {}", "🐛 Debug:".purple(), message);
            } else {
                println!("Debug: {}", message);
            }
        }
    }

    /// Create a progress bar for long-running operations
    pub fn create_progress_bar(&self, total: usize, message: &str) -> Option<ProgressBar> {
        if !self.show_progress || self.verbosity == 0 {
            return None;
        }

        Some(ProgressBar::new(total, message.to_string(), self.colored))
    }
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple progress bar implementation
pub struct ProgressBar {
    total: usize,
    current: usize,
    message: String,
    colored: bool,
    last_printed_len: usize,
}

impl ProgressBar {
    fn new(total: usize, message: String, colored: bool) -> Self {
        Self {
            total,
            current: 0,
            message,
            colored,
            last_printed_len: 0,
        }
    }

    /// Update progress
    pub fn update(&mut self, current: usize) {
        self.current = current;
        self.render();
    }

    /// Increment progress by 1
    pub fn increment(&mut self) {
        self.current += 1;
        self.render();
    }

    /// Finish the progress bar
    pub fn finish(&mut self) {
        self.current = self.total;
        self.render();
        println!(); // New line after completion
    }

    fn render(&mut self) {
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64) * 100.0
        } else {
            100.0
        };

        let bar_width = 30;
        let filled = ((percentage / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar = if self.colored {
            format!("{}{}",
                "█".repeat(filled).green(),
                "░".repeat(empty).dimmed()
            )
        } else {
            format!("{}{}",
                "█".repeat(filled),
                "░".repeat(empty)
            )
        };

        let progress_text = if self.colored {
            format!("\r{} [{}] {:.1}% ({}/{})",
                self.message.blue(),
                bar,
                percentage,
                self.current,
                self.total
            )
        } else {
            format!("\r{} [{}] {:.1}% ({}/{})",
                self.message,
                bar,
                percentage,
                self.current,
                self.total
            )
        };

        // Clear previous line if it was longer
        if progress_text.len() < self.last_printed_len {
            print!("\r{}", " ".repeat(self.last_printed_len));
        }

        print!("{}", progress_text);
        io::stdout().flush().unwrap_or(());
        
        self.last_printed_len = progress_text.len();
    }
}

/// Format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = duration.subsec_millis();

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 0 {
        format!("{}.{:03}s", seconds, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Format bytes in human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Output compatibility layer for Python engine compatibility
pub struct CompatibilityFormatter;

impl CompatibilityFormatter {
    /// Format execution result in Python engine compatible format
    pub fn format_python_compatible(result: &ExecutionResult) -> String {
        let mut output = String::new();
        
        // Header
        output.push_str(&format!("DAG: {}\n", result.dag_id));
        output.push_str(&format!("Execution ID: {}\n", result.execution_id));
        output.push_str(&format!("Status: {:?}\n", result.status));
        
        if let Some(duration) = result.total_duration {
            output.push_str(&format!("Duration: {:.2}s\n", duration.as_secs_f64()));
        }
        
        output.push_str("\nTasks:\n");
        
        // Task results
        for (task_id, task_result) in &result.task_results {
            output.push_str(&format!("  {}: {:?}", task_id, task_result.status));
            
            if let Some(exit_code) = task_result.exit_code {
                output.push_str(&format!(" (exit: {})", exit_code));
            }
            
            output.push_str(&format!(" [{:.2}s]", task_result.duration.as_secs_f64()));
            output.push('\n');
            
            if !task_result.stdout.is_empty() {
                output.push_str(&format!("    stdout: {}\n", task_result.stdout.trim()));
            }
            
            if !task_result.stderr.is_empty() {
                output.push_str(&format!("    stderr: {}\n", task_result.stderr.trim()));
            }
        }
        
        // Summary
        output.push_str("\nSummary:\n");
        output.push_str(&format!("  Total: {}\n", result.metrics.total_tasks));
        output.push_str(&format!("  Successful: {}\n", result.metrics.successful_tasks));
        output.push_str(&format!("  Failed: {}\n", result.metrics.failed_tasks));
        output.push_str(&format!("  Success Rate: {:.1}%\n", result.success_rate()));
        
        output
    }

    /// Format task result in Python engine compatible format
    pub fn format_task_python_compatible(task_result: &TaskResult) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Task: {}\n", task_result.task_id));
        output.push_str(&format!("Status: {:?}\n", task_result.status));
        output.push_str(&format!("Duration: {:.2}s\n", task_result.duration.as_secs_f64()));
        
        if let Some(exit_code) = task_result.exit_code {
            output.push_str(&format!("Exit Code: {}\n", exit_code));
        }
        
        if !task_result.stdout.is_empty() {
            output.push_str(&format!("Stdout:\n{}\n", task_result.stdout));
        }
        
        if !task_result.stderr.is_empty() {
            output.push_str(&format!("Stderr:\n{}\n", task_result.stderr));
        }
        
        if let Some(error) = &task_result.error_message {
            output.push_str(&format!("Error: {}\n", error));
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(5)), "5.000s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_output_formatter_creation() {
        let formatter = OutputFormatter::new();
        assert!(formatter.colored);
        assert_eq!(formatter.verbosity, 1);
        assert!(formatter.show_progress);

        let custom_formatter = OutputFormatter::with_settings(false, 2, false);
        assert!(!custom_formatter.colored);
        assert_eq!(custom_formatter.verbosity, 2);
        assert!(!custom_formatter.show_progress);
    }

    #[test]
    fn test_progress_bar() {
        let mut progress = ProgressBar::new(10, "Testing".to_string(), false);
        progress.update(5);
        assert_eq!(progress.current, 5);
        
        progress.increment();
        assert_eq!(progress.current, 6);
        
        progress.finish();
        assert_eq!(progress.current, 10);
    }
}