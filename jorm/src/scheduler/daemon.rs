use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::signal;
use tokio::sync::mpsc;

use crate::core::engine::JormEngine;
use crate::scheduler::cron::{Schedule, ScheduleError, Scheduler};

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("Daemon is already running")]
    AlreadyRunning,

    #[error("Daemon is not running")]
    NotRunning,

    #[error("Failed to read daemon state: {0}")]
    StateReadError(String),

    #[error("Failed to write daemon state: {0}")]
    StateWriteError(String),

    #[error("Schedule error: {0}")]
    ScheduleError(#[from] ScheduleError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DaemonState {
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub schedules_file: Option<String>,
    pub log_file: Option<String>,
}

pub struct Daemon {
    engine: Arc<JormEngine>,
    scheduler: Scheduler,
    state_file: PathBuf,
    schedules_file: Option<PathBuf>,
    log_file: Option<PathBuf>,
}

impl Daemon {
    pub fn new(
        engine: Arc<JormEngine>,
        state_file: PathBuf,
        schedules_file: Option<PathBuf>,
        log_file: Option<PathBuf>,
    ) -> Self {
        let scheduler = Scheduler::new(engine.clone());

        Self {
            engine,
            scheduler,
            state_file,
            schedules_file,
            log_file,
        }
    }

    pub async fn start(&mut self) -> Result<(), DaemonError> {
        // Check if daemon is already running
        if self.is_running().await? {
            return Err(DaemonError::AlreadyRunning);
        }

        // Load schedules if file is provided
        if let Some(schedules_file) = self.schedules_file.clone() {
            self.load_schedules(&schedules_file).await?;
        }

        // Write daemon state
        let state = DaemonState {
            pid: Some(std::process::id()),
            started_at: Some(Utc::now()),
            schedules_file: self
                .schedules_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            log_file: self
                .log_file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        };
        self.write_state(&state).await?;

        println!("Jorm daemon started (PID: {})", std::process::id());
        if let Some(log_file) = &self.log_file {
            println!("Logging to: {}", log_file.display());
        }

        // Set up graceful shutdown handling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Handle shutdown signals
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            let _ = signal::ctrl_c().await;
            println!("Received shutdown signal, stopping daemon...");
            let _ = shutdown_tx_clone.send(()).await;
        });

        // Start scheduler in a separate task
        let mut scheduler_clone = Scheduler::new(self.engine.clone());

        // Copy schedules to the new scheduler
        for schedule in self.scheduler.list_schedules() {
            scheduler_clone.add_schedule(schedule.clone())?;
        }

        let scheduler_handle = tokio::spawn(async move {
            if let Err(e) = scheduler_clone.start().await {
                eprintln!("Scheduler error: {}", e);
            }
        });

        // Wait for shutdown signal
        let _ = shutdown_rx.recv().await;

        // Stop scheduler
        scheduler_handle.abort();

        // Clean up daemon state
        self.stop().await?;

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), DaemonError> {
        if !self.is_running().await? {
            return Err(DaemonError::NotRunning);
        }

        // Stop scheduler
        self.scheduler.stop();

        // Remove state file
        if self.state_file.exists() {
            fs::remove_file(&self.state_file).await?;
        }

        println!("Jorm daemon stopped");
        Ok(())
    }

    pub async fn status(&self) -> Result<DaemonState, DaemonError> {
        self.read_state().await
    }

    pub async fn is_running(&self) -> Result<bool, DaemonError> {
        match self.read_state().await {
            Ok(state) => {
                if let Some(pid) = state.pid {
                    // Check if process is actually running
                    Ok(self.is_process_running(pid))
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false), // No state file means not running
        }
    }

    pub async fn add_schedule(&mut self, schedule: Schedule) -> Result<(), DaemonError> {
        self.scheduler.add_schedule(schedule)?;

        // Save schedules if file is configured
        if let Some(schedules_file) = &self.schedules_file {
            self.save_schedules(schedules_file).await?;
        }

        Ok(())
    }

    pub async fn remove_schedule(&mut self, id: &str) -> Result<Schedule, DaemonError> {
        let schedule = self.scheduler.remove_schedule(id)?;

        // Save schedules if file is configured
        if let Some(schedules_file) = &self.schedules_file {
            self.save_schedules(schedules_file).await?;
        }

        Ok(schedule)
    }

    pub fn list_schedules(&self) -> Vec<&Schedule> {
        self.scheduler.list_schedules()
    }

    async fn read_state(&self) -> Result<DaemonState, DaemonError> {
        if !self.state_file.exists() {
            return Ok(DaemonState::default());
        }

        let content = fs::read_to_string(&self.state_file)
            .await
            .map_err(|e| DaemonError::StateReadError(e.to_string()))?;

        serde_json::from_str(&content).map_err(|e| DaemonError::StateReadError(e.to_string()))
    }

    async fn write_state(&self, state: &DaemonState) -> Result<(), DaemonError> {
        let content = serde_json::to_string_pretty(state)
            .map_err(|e| DaemonError::StateWriteError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.state_file, content)
            .await
            .map_err(|e| DaemonError::StateWriteError(e.to_string()))?;

        Ok(())
    }

    async fn load_schedules(&mut self, schedules_file: &PathBuf) -> Result<(), DaemonError> {
        if !schedules_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(schedules_file)
            .await
            .map_err(|e| DaemonError::StateReadError(e.to_string()))?;

        let schedules: Vec<Schedule> = serde_json::from_str(&content)
            .map_err(|e| DaemonError::StateReadError(e.to_string()))?;

        for schedule in schedules {
            self.scheduler.add_schedule(schedule)?;
        }

        println!(
            "Loaded {} schedules from {}",
            self.scheduler.list_schedules().len(),
            schedules_file.display()
        );
        Ok(())
    }

    async fn save_schedules(&self, schedules_file: &PathBuf) -> Result<(), DaemonError> {
        let schedules: Vec<&Schedule> = self.scheduler.list_schedules();
        let content = serde_json::to_string_pretty(&schedules)
            .map_err(|e| DaemonError::StateWriteError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = schedules_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(schedules_file, content)
            .await
            .map_err(|e| DaemonError::StateWriteError(e.to_string()))?;

        Ok(())
    }

    fn is_process_running(&self, pid: u32) -> bool {
        // On Windows, we can check if process exists
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}")])
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains(&pid.to_string())
                }
                Err(_) => false,
            }
        }

        // On Unix-like systems, we can check /proc or use kill -0
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_daemon_state_operations() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let state_file = temp_dir.path().join("daemon.state");

        let engine = Arc::new(JormEngine::new().await?);
        let daemon = Daemon::new(engine, state_file.clone(), None, None);

        // Test initial state
        let state = daemon.read_state().await?;
        assert!(state.pid.is_none());

        // Test writing state
        let test_state = DaemonState {
            pid: Some(12345),
            started_at: Some(Utc::now()),
            schedules_file: None,
            log_file: None,
        };
        daemon.write_state(&test_state).await?;

        // Test reading state
        let read_state = daemon.read_state().await?;
        assert_eq!(read_state.pid, Some(12345));
        assert!(read_state.started_at.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_daemon_not_running_initially() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let state_file = temp_dir.path().join("daemon.state");

        let engine = Arc::new(JormEngine::new().await?);
        let daemon = Daemon::new(engine, state_file, None, None);

        assert!(!daemon.is_running().await?);

        Ok(())
    }
}
