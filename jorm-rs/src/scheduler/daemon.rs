use super::{CronScheduler, SchedulerEvent};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(unix)]
use tokio::signal;

pub struct SchedulerDaemon {
    scheduler: Arc<CronScheduler>,
    pid_file: Option<PathBuf>,
    log_file: Option<PathBuf>,
}

impl SchedulerDaemon {
    pub fn new(scheduler: CronScheduler) -> Self {
        Self {
            scheduler: Arc::new(scheduler),
            pid_file: None,
            log_file: None,
        }
    }

    pub fn with_pid_file(mut self, pid_file: impl Into<std::path::PathBuf>) -> Self {
        self.pid_file = Some(pid_file.into());
        self
    }

    pub fn with_log_file(mut self, log_file: impl Into<std::path::PathBuf>) -> Self {
        self.log_file = Some(log_file.into());
        self
    }

    pub async fn start(&self) -> Result<()> {
        // Write PID file if specified
        if let Some(pid_file) = &self.pid_file {
            let pid = std::process::id();
            tokio::fs::write(pid_file, pid.to_string())
                .await
                .context("Failed to write PID file")?;
        }

        // Start the scheduler
        self.scheduler
            .start()
            .await
            .context("Failed to start scheduler")?;

        // Set up event logging
        let event_receiver = self.scheduler.event_receiver();
        let log_file = self.log_file.clone();

        tokio::spawn(async move {
            let mut receiver = event_receiver.lock().await;

            while let Some(event) = receiver.recv().await {
                let log_message = format_event(&event);

                if let Some(log_file) = &log_file {
                    // Write to log file
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                    let log_line = format!("[{timestamp}] {log_message}\n");

                    let write_result = async {
                        use tokio::io::AsyncWriteExt;
                        let mut file = tokio::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(log_file)
                            .await?;
                        file.write_all(log_line.as_bytes()).await
                    }
                    .await;

                    if let Err(e) = write_result {
                        eprintln!("Failed to write to log file: {e}");
                    }
                } else {
                    // Write to stdout
                    println!("{log_message}");
                }
            }
        });

        // Set up signal handlers (Unix only)
        #[cfg(unix)]
        {
            let scheduler_for_signal = Arc::clone(&self.scheduler);
            let pid_file_for_signal = self.pid_file.clone();

            tokio::spawn(async move {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        println!("Received SIGTERM, shutting down gracefully...");
                    }
                    _ = sigint.recv() => {
                        println!("Received SIGINT, shutting down gracefully...");
                    }
                }

                // Stop the scheduler
                if let Err(e) = scheduler_for_signal.stop().await {
                    eprintln!("Error stopping scheduler: {}", e);
                }

                // Clean up PID file
                if let Some(pid_file) = pid_file_for_signal {
                    if let Err(e) = tokio::fs::remove_file(&pid_file).await {
                        eprintln!("Failed to remove PID file: {}", e);
                    }
                }

                std::process::exit(0);
            });
        }

        // Windows signal handling
        #[cfg(windows)]
        {
            let scheduler_for_signal = Arc::clone(&self.scheduler);
            let pid_file_for_signal = self.pid_file.clone();

            tokio::spawn(async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for ctrl+c");
                println!("Received Ctrl+C, shutting down gracefully...");

                // Stop the scheduler
                if let Err(e) = scheduler_for_signal.stop().await {
                    eprintln!("Error stopping scheduler: {e}");
                }

                // Clean up PID file
                if let Some(pid_file) = pid_file_for_signal {
                    if let Err(e) = tokio::fs::remove_file(&pid_file).await {
                        eprintln!("Failed to remove PID file: {e}");
                    }
                }

                std::process::exit(0);
            });
        }

        // Keep the daemon running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn stop(&self) -> Result<()> {
        self.scheduler.stop().await?;

        // Clean up PID file
        if let Some(pid_file) = &self.pid_file {
            tokio::fs::remove_file(pid_file)
                .await
                .context("Failed to remove PID file")?;
        }

        Ok(())
    }

    pub fn is_running(&self) -> Result<bool> {
        if let Some(pid_file) = &self.pid_file {
            if pid_file.exists() {
                let pid_str =
                    std::fs::read_to_string(pid_file).context("Failed to read PID file")?;
                let pid: u32 = pid_str.trim().parse().context("Invalid PID in PID file")?;

                // Check if process is still running
                Ok(is_process_running(pid))
            } else {
                Ok(false)
            }
        } else {
            // No PID file, can't determine status
            Ok(false)
        }
    }
}

fn format_event(event: &SchedulerEvent) -> String {
    match event {
        SchedulerEvent::JobScheduled(job_id) => {
            format!("Job {job_id} scheduled")
        }
        SchedulerEvent::JobStarted(job_id) => {
            format!("Job {job_id} started")
        }
        SchedulerEvent::JobCompleted(job_id, result) => {
            format!(
                "Job {} completed successfully in {:?}: {}",
                job_id, result.duration, result.output
            )
        }
        SchedulerEvent::JobFailed(job_id, error) => {
            format!("Job {job_id} failed: {error}")
        }
        SchedulerEvent::JobCancelled(job_id) => {
            format!("Job {job_id} cancelled")
        }
        SchedulerEvent::SchedulerStarted => "Scheduler started".to_string(),
        SchedulerEvent::SchedulerStopped => "Scheduler stopped".to_string(),
    }
}

#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;

    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;

    Command::new("tasklist")
        .arg("/FI")
        .arg(format!("PID eq {pid}"))
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(&pid.to_string())
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_daemon_creation() {
        let scheduler = CronScheduler::new();
        let daemon = SchedulerDaemon::new(scheduler);

        // Test that daemon can be created
        assert!(daemon.pid_file.is_none());
        assert!(daemon.log_file.is_none());
    }

    #[tokio::test]
    async fn test_daemon_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let pid_file = temp_dir.path().join("test.pid");
        let log_file = temp_dir.path().join("test.log");

        let scheduler = CronScheduler::new();
        let daemon = SchedulerDaemon::new(scheduler)
            .with_pid_file(pid_file.clone())
            .with_log_file(log_file.clone());

        assert_eq!(daemon.pid_file, Some(pid_file));
        assert_eq!(daemon.log_file, Some(log_file));
    }
}
