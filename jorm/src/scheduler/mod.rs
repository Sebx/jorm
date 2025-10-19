pub mod config;
pub mod cron_scheduler;
pub mod daemon;
pub mod triggers;

pub use config::*;
pub use cron_scheduler::*;
pub use daemon::*;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: Uuid,
    pub name: String,
    pub dag_file: String,
    pub schedule: Schedule,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub failure_count: u64,
    pub config: JobConfig,
}

#[derive(Debug, Clone)]
pub enum Schedule {
    Cron(String),
    Interval(std::time::Duration),
    Once(DateTime<Utc>),
    Manual,
}

#[derive(Debug, Clone)]
pub struct JobConfig {
    pub timeout: Option<std::time::Duration>,
    pub max_retries: u32,
    pub retry_delay: std::time::Duration,
    pub overlap_policy: OverlapPolicy,
    pub environment: HashMap<String, String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OverlapPolicy {
    Allow,
    Skip,
    Queue,
    Cancel,
}

#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    JobScheduled(Uuid),
    JobStarted(Uuid),
    JobCompleted(Uuid, JobResult),
    JobFailed(Uuid, String),
    JobCancelled(Uuid),
    SchedulerStarted,
    SchedulerStopped,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub success: bool,
    pub duration: std::time::Duration,
    pub output: String,
    pub error: Option<String>,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            timeout: Some(std::time::Duration::from_secs(3600)), // 1 hour
            max_retries: 3,
            retry_delay: std::time::Duration::from_secs(60), // 1 minute
            overlap_policy: OverlapPolicy::Skip,
            environment: HashMap::new(),
            working_directory: None,
        }
    }
}

impl ScheduledJob {
    pub fn new(name: String, dag_file: String, schedule: Schedule) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            dag_file,
            schedule,
            enabled: true,
            last_run: None,
            next_run: None,
            run_count: 0,
            failure_count: 0,
            config: JobConfig::default(),
        }
    }

    pub fn with_config(mut self, config: JobConfig) -> Self {
        self.config = config;
        self
    }

    pub fn calculate_next_run(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match &self.schedule {
            Schedule::Cron(expr) => {
                use std::str::FromStr;
                if let Ok(schedule) = cron::Schedule::from_str(expr) {
                    schedule
                        .upcoming(chrono::Utc)
                        .next()
                        .map(|dt| dt.with_timezone(&Utc))
                } else {
                    None
                }
            }
            Schedule::Interval(duration) => {
                Some(from + chrono::Duration::from_std(*duration).unwrap_or_default())
            }
            Schedule::Once(when) => {
                if *when > from {
                    Some(*when)
                } else {
                    None
                }
            }
            Schedule::Manual => None,
        }
    }
}
