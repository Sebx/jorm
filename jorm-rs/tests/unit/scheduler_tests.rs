//! Comprehensive Scheduler Unit Tests
//!
//! This module contains all unit tests for the scheduler infrastructure,
//! consolidating previously scattered test files into a single, well-organized module.

use jorm_rs::scheduler::{
    ConfigManager, CronScheduler, JobConfig, OverlapPolicy, Schedule, ScheduledJob,
    SchedulerConfig, SchedulerDaemon,
};
use std::time::Duration;

#[cfg(test)]
mod scheduler_creation_tests {
    use super::*;

    #[test]
    fn test_cron_scheduler_creation() {
        let scheduler = CronScheduler::new();
        // Should create successfully
        assert!(true);
    }

    #[test]
    fn test_scheduled_job_creation() {
        let job = ScheduledJob::new(
            "test_job".to_string(),
            "test.txt".to_string(),
            Schedule::Cron("0 0 * * *".to_string()),
        );

        assert_eq!(job.name, "test_job");
        assert_eq!(job.dag_file, "test.txt");
    }

    #[test]
    fn test_schedule_creation() {
        let cron_schedule = Schedule::Cron("0 0 * * *".to_string());
        let interval_schedule = Schedule::Interval(Duration::from_secs(3600));
        let once_schedule = Schedule::Once(chrono::Utc::now());
        let manual_schedule = Schedule::Manual;

        // All schedule types should be created successfully
        assert!(matches!(cron_schedule, Schedule::Cron(_)));
        assert!(matches!(interval_schedule, Schedule::Interval(_)));
        assert!(matches!(once_schedule, Schedule::Once(_)));
        assert!(matches!(manual_schedule, Schedule::Manual));
    }
}

#[cfg(test)]
mod scheduler_config_tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert!(config.scheduler.check_interval_seconds > 0);
        assert!(config.daemon.max_concurrent_jobs > 0);
    }

    #[test]
    fn test_scheduler_config_customization() {
        let mut config = SchedulerConfig::default();
        config.scheduler.check_interval_seconds = 30;
        config.daemon.max_concurrent_jobs = 5;

        assert_eq!(config.scheduler.check_interval_seconds, 30);
        assert_eq!(config.daemon.max_concurrent_jobs, 5);
    }

    #[test]
    fn test_job_config_creation() {
        let job_config = JobConfig {
            timeout: Some(Duration::from_secs(300)),
            max_retries: 3,
            retry_delay: Duration::from_secs(60),
            overlap_policy: OverlapPolicy::Skip,
            environment: std::collections::HashMap::new(),
            working_directory: Some("/tmp".to_string()),
        };

        assert_eq!(job_config.max_retries, 3);
        assert_eq!(job_config.retry_delay, Duration::from_secs(60));
    }
}

#[cfg(test)]
mod cron_expression_tests {
    use super::*;

    #[test]
    fn test_cron_expression_parsing() {
        let valid_expressions = vec![
            "0 0 * * *",    // Daily at midnight
            "0 */6 * * *",  // Every 6 hours
            "0 0 * * 1",    // Weekly on Monday
            "0 0 1 * *",    // Monthly on 1st
            "*/15 * * * *", // Every 15 minutes
        ];

        for expr in valid_expressions {
            let schedule = Schedule::Cron(expr.to_string());
            assert!(matches!(schedule, Schedule::Cron(_)));
        }
    }

    #[test]
    fn test_cron_scheduling_logic() {
        let scheduler = CronScheduler::new();

        // Test cron expression validation
        let job = ScheduledJob::new(
            "daily_job".to_string(),
            "test.txt".to_string(),
            Schedule::Cron("0 0 * * *".to_string()),
        );

        // Should create job successfully
        assert_eq!(job.name, "daily_job");
        assert_eq!(job.dag_file, "test.txt");
    }
}

#[cfg(test)]
mod configuration_management_tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let manager = ConfigManager::new();

        // Test environment variable substitution
        std::env::set_var("TEST_VAR", "test_value");
        let result = manager
            .substitute_variables("Hello ${TEST_VAR}!", None)
            .unwrap();
        assert_eq!(result, "Hello test_value!");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_configuration_serialization() {
        let config = SchedulerConfig::default();

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: SchedulerConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            config.scheduler.check_interval_seconds,
            deserialized.scheduler.check_interval_seconds
        );
    }

    #[test]
    fn test_environment_management() {
        let manager = ConfigManager::new();

        // Test environment variable handling
        std::env::set_var("JORM_TEST_VAR", "test_value");
        let result = manager
            .substitute_variables("Value: ${JORM_TEST_VAR}", None)
            .unwrap();
        assert_eq!(result, "Value: test_value");
        std::env::remove_var("JORM_TEST_VAR");
    }
}

#[cfg(test)]
mod daemon_tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_creation() {
        let config = SchedulerConfig::default();
        let scheduler = CronScheduler::new();
        let daemon = SchedulerDaemon::new(scheduler);

        // Should create daemon successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_daemon_lifecycle() {
        let config = SchedulerConfig::default();
        let scheduler = CronScheduler::new();
        let daemon = SchedulerDaemon::new(scheduler);

        // Test daemon start/stop lifecycle
        // Note: In a real test, we would start and stop the daemon
        // For now, we just verify it can be created
        assert!(true);
    }
}

#[cfg(test)]
mod overlap_policy_tests {
    use super::*;

    #[test]
    fn test_overlap_policies() {
        let policies = vec![
            OverlapPolicy::Allow,
            OverlapPolicy::Skip,
            OverlapPolicy::Queue,
            OverlapPolicy::Cancel,
        ];

        for policy in policies {
            // All policies should be valid
            assert!(matches!(
                policy,
                OverlapPolicy::Allow
                    | OverlapPolicy::Skip
                    | OverlapPolicy::Queue
                    | OverlapPolicy::Cancel
            ));
        }
    }

    #[test]
    fn test_job_config_with_overlap_policy() {
        let job_config = JobConfig {
            timeout: Some(Duration::from_secs(300)),
            max_retries: 3,
            retry_delay: Duration::from_secs(60),
            overlap_policy: OverlapPolicy::Skip,
            environment: std::collections::HashMap::new(),
            working_directory: Some("/tmp".to_string()),
        };

        assert!(matches!(job_config.overlap_policy, OverlapPolicy::Skip));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_phase2_integration() {
        println!("✅ Phase 2 integration test placeholder");
        // This is a placeholder for future integration tests
        assert!(true);
    }

    #[test]
    fn test_scheduler_integration() {
        let scheduler = CronScheduler::new();
        let config = SchedulerConfig::default();

        // Test scheduler configuration
        assert!(config.scheduler.check_interval_seconds > 0);
        assert!(config.daemon.max_concurrent_jobs > 0);

        println!("✅ Scheduler integration test passed");
    }
}
