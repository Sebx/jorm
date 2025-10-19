//! Unit tests for environment variable management

use jorm::executor::{
    EnvironmentConfig, EnvironmentManager, InterpolationContext, SecureValue,
    ExecutionContext, ExecutorConfig, Task, TaskConfig,
};
use std::collections::HashMap;

#[test]
fn test_secure_value_creation() {
    let secure = SecureValue::secure("secret123".to_string());
    let plain = SecureValue::plain("public_value".to_string());

    assert!(secure.is_secure());
    assert!(!plain.is_secure());
    assert_eq!(secure.value(), "secret123");
    assert_eq!(plain.value(), "public_value");
}

#[test]
fn test_secure_value_display() {
    let secure = SecureValue::secure("secret123".to_string());
    let plain = SecureValue::plain("public_value".to_string());

    assert_eq!(format!("{}", secure), "***");
    assert_eq!(format!("{}", plain), "public_value");
    assert_eq!(format!("{:?}", secure), "SecureValue(***)");
    assert!(format!("{:?}", plain).contains("public_value"));
}

#[test]
fn test_environment_config_default() {
    let config = EnvironmentConfig::default();
    
    assert!(config.inherit_parent);
    assert!(config.inherit_executor);
    assert!(config.enable_interpolation);
    assert!(config.exclude.is_empty());
    assert!(config.variables.is_empty());
    assert!(config.secure_prefixes.contains(&"PASSWORD".to_string()));
    assert!(config.secure_prefixes.contains(&"SECRET".to_string()));
}

#[test]
fn test_environment_manager_basic() {
    let executor_env = HashMap::from([
        ("EXECUTOR_VAR".to_string(), "executor_value".to_string()),
        ("SHARED_VAR".to_string(), "executor_shared".to_string()),
    ]);
    
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);

    let task_env = HashMap::from([
        ("TASK_VAR".to_string(), "task_value".to_string()),
        ("SHARED_VAR".to_string(), "task_shared".to_string()), // Should override executor
    ]);

    let result = manager.build_task_environment(&task_env, None).unwrap();

    assert!(result.contains_key("EXECUTOR_VAR"));
    assert!(result.contains_key("TASK_VAR"));
    assert!(result.contains_key("SHARED_VAR"));
    
    assert_eq!(result.get("EXECUTOR_VAR").unwrap().value(), "executor_value");
    assert_eq!(result.get("TASK_VAR").unwrap().value(), "task_value");
    assert_eq!(result.get("SHARED_VAR").unwrap().value(), "task_shared"); // Task overrides executor
}

#[test]
fn test_secure_variable_detection() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);

    let task_env = HashMap::from([
        ("API_KEY".to_string(), "secret123".to_string()),
        ("DATABASE_PASSWORD".to_string(), "password123".to_string()),
        ("MY_SECRET_TOKEN".to_string(), "token123".to_string()),
        ("PUBLIC_URL".to_string(), "https://example.com".to_string()),
        ("DEBUG_MODE".to_string(), "true".to_string()),
    ]);

    let result = manager.build_task_environment(&task_env, None).unwrap();

    // Should be secure
    assert!(result.get("API_KEY").unwrap().is_secure());
    assert!(result.get("DATABASE_PASSWORD").unwrap().is_secure());
    assert!(result.get("MY_SECRET_TOKEN").unwrap().is_secure());
    
    // Should not be secure
    assert!(!result.get("PUBLIC_URL").unwrap().is_secure());
    assert!(!result.get("DEBUG_MODE").unwrap().is_secure());
}#
[test]
fn test_environment_exclusion() {
    let executor_env = HashMap::from([
        ("KEEP_VAR".to_string(), "keep".to_string()),
        ("EXCLUDE_VAR".to_string(), "exclude".to_string()),
        ("ANOTHER_KEEP".to_string(), "keep2".to_string()),
    ]);
    
    let config = EnvironmentConfig {
        exclude: vec!["EXCLUDE_VAR".to_string()],
        inherit_parent: false, // Disable to avoid system env interference
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    let task_env = HashMap::from([
        ("TASK_VAR".to_string(), "task_value".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, None).unwrap();

    assert!(result.contains_key("KEEP_VAR"));
    assert!(result.contains_key("ANOTHER_KEEP"));
    assert!(result.contains_key("TASK_VAR"));
    assert!(!result.contains_key("EXCLUDE_VAR"));
}

#[test]
fn test_variable_interpolation_basic() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        inherit_parent: false, // Disable to avoid system env interference
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    let mut context = InterpolationContext::new();
    context.add_variable("BASE_PATH".to_string(), "/app".to_string());
    context.add_variable("VERSION".to_string(), "1.0.0".to_string());

    let task_env = HashMap::from([
        ("FULL_PATH".to_string(), "${BASE_PATH}/data".to_string()),
        ("APP_VERSION".to_string(), "v${VERSION}".to_string()),
        ("COMPLEX_PATH".to_string(), "${BASE_PATH}/logs/${VERSION}".to_string()),
    ]);

    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

    assert_eq!(result.get("FULL_PATH").unwrap().value(), "/app/data");
    assert_eq!(result.get("APP_VERSION").unwrap().value(), "v1.0.0");
    assert_eq!(result.get("COMPLEX_PATH").unwrap().value(), "/app/logs/1.0.0");
}

#[test]
fn test_task_output_interpolation() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        inherit_parent: false,
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    let mut context = InterpolationContext::new();
    context.add_task_output("task1".to_string(), "output_value_123".to_string());
    context.add_task_output("task2".to_string(), "/tmp/generated_file.txt".to_string());

    let task_env = HashMap::from([
        ("PREVIOUS_OUTPUT".to_string(), "${task.task1.stdout}".to_string()),
        ("INPUT_FILE".to_string(), "${task.task2.stdout}".to_string()),
        ("COMBINED".to_string(), "Result: ${task.task1.stdout}".to_string()),
    ]);

    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

    assert_eq!(result.get("PREVIOUS_OUTPUT").unwrap().value(), "output_value_123");
    assert_eq!(result.get("INPUT_FILE").unwrap().value(), "/tmp/generated_file.txt");
    assert_eq!(result.get("COMBINED").unwrap().value(), "Result: output_value_123");
}

#[test]
fn test_system_info_interpolation() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        inherit_parent: false,
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    let context = InterpolationContext::new(); // Contains system info by default

    let task_env = HashMap::from([
        ("LOG_FILE".to_string(), "/logs/${system.hostname}.log".to_string()),
        ("USER_DIR".to_string(), "/home/${system.user}".to_string()),
        ("TIMESTAMP_FILE".to_string(), "backup_${system.timestamp}.tar.gz".to_string()),
    ]);

    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

    let log_file = result.get("LOG_FILE").unwrap().value();
    assert!(log_file.starts_with("/logs/"));
    assert!(log_file.ends_with(".log"));
    assert!(!log_file.contains("${system.hostname}"));

    let timestamp_file = result.get("TIMESTAMP_FILE").unwrap().value();
    assert!(timestamp_file.starts_with("backup_"));
    assert!(timestamp_file.ends_with(".tar.gz"));
    assert!(!timestamp_file.contains("${system.timestamp}"));
}

#[test]
fn test_circular_reference_detection() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        variables: HashMap::from([
            ("VAR_A".to_string(), "${VAR_B}".to_string()),
            ("VAR_B".to_string(), "${VAR_A}".to_string()),
        ]),
        inherit_parent: false,
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    // Should detect circular reference
    assert!(manager.validate_config().is_err());
}

#[test]
fn test_complex_circular_reference_detection() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        variables: HashMap::from([
            ("VAR_A".to_string(), "${VAR_B}/path".to_string()),
            ("VAR_B".to_string(), "${VAR_C}".to_string()),
            ("VAR_C".to_string(), "${VAR_A}".to_string()),
        ]),
        inherit_parent: false,
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    // Should detect circular reference in chain
    assert!(manager.validate_config().is_err());
}

#[test]
fn test_no_circular_reference() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        variables: HashMap::from([
            ("BASE_PATH".to_string(), "/app".to_string()),
            ("DATA_PATH".to_string(), "${BASE_PATH}/data".to_string()),
            ("LOG_PATH".to_string(), "${BASE_PATH}/logs".to_string()),
        ]),
        inherit_parent: false,
        ..Default::default()
    };
    let manager = EnvironmentManager::new(executor_env, config);

    // Should not detect circular reference
    assert!(manager.validate_config().is_ok());
}

#[tokio::test]
async fn test_execution_context_environment_integration() {
    let config = ExecutorConfig {
        environment_variables: HashMap::from([
            ("EXECUTOR_VAR".to_string(), "executor_value".to_string()),
            ("API_SECRET".to_string(), "secret123".to_string()),
        ]),
        inherit_environment: false, // Disable to avoid system env interference
        ..Default::default()
    };

    let task = Task::new(
        "test_task".to_string(),
        "Test Task".to_string(),
        "shell".to_string(),
        TaskConfig::Shell {
            command: "echo test".to_string(),
            working_dir: None,
            shell: None,
        },
    ).with_env_var("TASK_VAR".to_string(), "task_value".to_string());

    let context = ExecutionContext::for_task_execution(
        "test_dag".to_string(),
        "test_task".to_string(),
        config,
        &task,
    );

    // Check that environment variables are properly set
    assert!(context.get_env_var("EXECUTOR_VAR").is_some());
    assert!(context.get_env_var("TASK_VAR").is_some());
    assert!(context.get_env_var("API_SECRET").is_some());

    // Check secure variable detection
    assert!(!context.get_env_var("EXECUTOR_VAR").unwrap().is_secure());
    assert!(!context.get_env_var("TASK_VAR").unwrap().is_secure());
    assert!(context.get_env_var("API_SECRET").unwrap().is_secure());

    // Check process environment conversion
    let process_env = context.env_vars_for_process();
    assert!(process_env.iter().any(|(k, _)| k == "EXECUTOR_VAR"));
    assert!(process_env.iter().any(|(k, _)| k == "TASK_VAR"));
    assert!(process_env.iter().any(|(k, _)| k == "API_SECRET"));
}