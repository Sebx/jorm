use chrono::{DateTime, Local, Utc};
use cron::Schedule as CronSchedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};

use crate::core::engine::{ExecutionResult, JormEngine};
use crate::core::error::JormError;

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Schedule not found: {0}")]
    ScheduleNotFound(String),

    #[error("DAG file not found: {0}")]
    DagFileNotFound(String),

    #[error("Execution error: {0}")]
    ExecutionError(#[from] JormError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    pub id: String,
    pub cron_expression: String,
    pub dag_file: String,
    pub enabled: bool,
    pub last_execution: Option<DateTime<Utc>>,
    pub next_execution: Option<DateTime<Utc>>,
}

impl Schedule {
    pub fn new(
        id: String,
        cron_expression: String,
        dag_file: String,
    ) -> Result<Self, ScheduleError> {
        // Validate cron expression
        let cron_schedule = CronSchedule::from_str(&cron_expression)
            .map_err(|e| ScheduleError::InvalidCron(format!("{}: {}", cron_expression, e)))?;

        // Calculate next execution time
        let next_execution = cron_schedule
            .upcoming(Utc)
            .next()
            .map(|dt| dt.with_timezone(&Utc));

        Ok(Self {
            id,
            cron_expression,
            dag_file,
            enabled: true,
            last_execution: None,
            next_execution,
        })
    }

    pub fn update_next_execution(&mut self) -> Result<(), ScheduleError> {
        let cron_schedule = CronSchedule::from_str(&self.cron_expression)
            .map_err(|e| ScheduleError::InvalidCron(format!("{}: {}", self.cron_expression, e)))?;

        self.next_execution = cron_schedule
            .upcoming(Utc)
            .next()
            .map(|dt| dt.with_timezone(&Utc));

        Ok(())
    }

    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }

        match self.next_execution {
            Some(next) => Utc::now() >= next,
            None => false,
        }
    }
}

pub struct Scheduler {
    engine: Arc<JormEngine>,
    schedules: HashMap<String, Schedule>,
    running: bool,
}

impl Scheduler {
    pub fn new(engine: Arc<JormEngine>) -> Self {
        Self {
            engine,
            schedules: HashMap::new(),
            running: false,
        }
    }

    pub fn add_schedule(&mut self, schedule: Schedule) -> Result<(), ScheduleError> {
        // Validate that the DAG file exists
        if !std::path::Path::new(&schedule.dag_file).exists() {
            return Err(ScheduleError::DagFileNotFound(schedule.dag_file.clone()));
        }

        let id = schedule.id.clone();
        self.schedules.insert(id, schedule);
        Ok(())
    }

    pub fn remove_schedule(&mut self, id: &str) -> Result<Schedule, ScheduleError> {
        self.schedules
            .remove(id)
            .ok_or_else(|| ScheduleError::ScheduleNotFound(id.to_string()))
    }

    pub fn get_schedule(&self, id: &str) -> Option<&Schedule> {
        self.schedules.get(id)
    }

    pub fn list_schedules(&self) -> Vec<&Schedule> {
        self.schedules.values().collect()
    }

    pub fn enable_schedule(&mut self, id: &str) -> Result<(), ScheduleError> {
        let schedule = self
            .schedules
            .get_mut(id)
            .ok_or_else(|| ScheduleError::ScheduleNotFound(id.to_string()))?;

        schedule.enabled = true;
        schedule.update_next_execution()?;
        Ok(())
    }

    pub fn disable_schedule(&mut self, id: &str) -> Result<(), ScheduleError> {
        let schedule = self
            .schedules
            .get_mut(id)
            .ok_or_else(|| ScheduleError::ScheduleNotFound(id.to_string()))?;

        schedule.enabled = false;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), ScheduleError> {
        if self.running {
            return Ok(());
        }

        self.running = true;
        println!("Scheduler started with {} schedules", self.schedules.len());

        while self.running {
            let now = Instant::now();

            // Check for due schedules
            let mut due_schedules = Vec::new();
            for (id, schedule) in &self.schedules {
                if schedule.is_due() {
                    due_schedules.push(id.clone());
                }
            }

            // Execute due schedules
            for schedule_id in due_schedules {
                if let Some(schedule) = self.schedules.get_mut(&schedule_id) {
                    println!(
                        "Executing scheduled DAG: {} ({})",
                        schedule.dag_file, schedule.id
                    );

                    let execution_start = Utc::now();
                    let result = self.engine.execute_from_file(&schedule.dag_file).await;

                    match result {
                        Ok(execution_result) => {
                            println!(
                                "Scheduled execution completed successfully: {}",
                                schedule.id
                            );
                            if !execution_result.success {
                                eprintln!(
                                    "Scheduled DAG execution had failures: {}",
                                    execution_result.message
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Scheduled execution failed for {}: {}", schedule.id, e);
                        }
                    }

                    // Update schedule timing
                    schedule.last_execution = Some(execution_start);
                    if let Err(e) = schedule.update_next_execution() {
                        eprintln!(
                            "Failed to update next execution time for {}: {}",
                            schedule.id, e
                        );
                    } else if let Some(next) = schedule.next_execution {
                        println!(
                            "Next execution for {} scheduled at: {}",
                            schedule.id,
                            next.with_timezone(&Local)
                        );
                    }
                }
            }

            // Sleep for a short interval before checking again
            // We check every 30 seconds to balance responsiveness with CPU usage
            let elapsed = now.elapsed();
            let sleep_duration = Duration::from_secs(30).saturating_sub(elapsed);
            sleep(sleep_duration).await;
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        if self.running {
            println!("Stopping scheduler...");
            self.running = false;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Execute a specific schedule immediately (for testing or manual triggers)
    pub async fn execute_schedule(&mut self, id: &str) -> Result<ExecutionResult, ScheduleError> {
        let schedule = self
            .schedules
            .get_mut(id)
            .ok_or_else(|| ScheduleError::ScheduleNotFound(id.to_string()))?;

        let execution_start = Utc::now();
        let result = self.engine.execute_from_file(&schedule.dag_file).await?;

        // Update last execution time
        schedule.last_execution = Some(execution_start);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_schedule_creation() {
        let schedule = Schedule::new(
            "test-schedule".to_string(),
            "0 0 0 * * *".to_string(), // Daily at midnight (sec min hour day month weekday)
            "test.dag".to_string(),
        )
        .unwrap();

        assert_eq!(schedule.id, "test-schedule");
        assert_eq!(schedule.cron_expression, "0 0 0 * * *");
        assert_eq!(schedule.dag_file, "test.dag");
        assert!(schedule.enabled);
        assert!(schedule.next_execution.is_some());
    }

    #[tokio::test]
    async fn test_invalid_cron_expression() {
        let result = Schedule::new(
            "test-schedule".to_string(),
            "invalid cron".to_string(),
            "test.dag".to_string(),
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScheduleError::InvalidCron(_)));
    }

    #[tokio::test]
    async fn test_scheduler_add_remove_schedule() -> Result<(), Box<dyn std::error::Error>> {
        let engine = Arc::new(JormEngine::new().await?);
        let mut scheduler = Scheduler::new(engine);

        // Create a temporary DAG file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(
            temp_file,
            "task test {{ type: shell, command: \"echo hello\" }}"
        )?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let schedule = Schedule::new(
            "test-schedule".to_string(),
            "0 0 0 * * *".to_string(),
            temp_path,
        )?;

        // Add schedule
        scheduler.add_schedule(schedule)?;
        assert_eq!(scheduler.list_schedules().len(), 1);

        // Remove schedule
        let removed = scheduler.remove_schedule("test-schedule")?;
        assert_eq!(removed.id, "test-schedule");
        assert_eq!(scheduler.list_schedules().len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_schedule_enable_disable() -> Result<(), Box<dyn std::error::Error>> {
        let engine = Arc::new(JormEngine::new().await?);
        let mut scheduler = Scheduler::new(engine);

        // Create a temporary DAG file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(
            temp_file,
            "task test {{ type: shell, command: \"echo hello\" }}"
        )?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let schedule = Schedule::new(
            "test-schedule".to_string(),
            "0 0 0 * * *".to_string(),
            temp_path,
        )?;

        scheduler.add_schedule(schedule)?;

        // Disable schedule
        scheduler.disable_schedule("test-schedule")?;
        let schedule = scheduler.get_schedule("test-schedule").unwrap();
        assert!(!schedule.enabled);

        // Enable schedule
        scheduler.enable_schedule("test-schedule")?;
        let schedule = scheduler.get_schedule("test-schedule").unwrap();
        assert!(schedule.enabled);

        Ok(())
    }
}
