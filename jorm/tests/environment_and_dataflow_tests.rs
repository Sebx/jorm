//! Integration tests for environment variable management and task output chaining

use jorm::executor::{
    DataflowManager, EnvironmentConfig, EnvironmentManager, InterpolationContext, 
    ParameterValue, ParameterValidation, ParameterType, SecureParameter, TaskOutput
};
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;

#[test]
fn test_environment_variable_inheritance() {
    // Test environment variable inheritance from executor to task
    let executor_env = HashMap::from([
        ("GLOBAL_VAR".to_string(), "global_value".to_string()),
        ("OVERRIDE_VAR".to_string(), "executor_value".to_string()),
    ]);
    
    let config = EnvironmentConfig {
        inherit_parent: false, // Don't inherit from system for test consistency
        inherit_executor: true,
        ..Default::default()
    };
    
    let manager = EnvironmentManager::new(executor_env, config);
    
    let task_env = HashMap::from([
        ("TASK_VAR".to_string(), "task_value".to_string()),
        ("OVERRIDE_VAR".to_string(), "task_override".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, None).unwrap();
    
    // Check that executor variables are inherited
    assert_eq!(result.get("GLOBAL_VAR").unwrap().value(), "global_value");
    
    // Check that task variables override executor variables
    assert_eq!(result.get("OVERRIDE_VAR").unwrap().value(), "task_override");
    
    // Check that task-specific variables are present
    assert_eq!(result.get("TASK_VAR").unwrap().value(), "task_value");
}

#[test]
fn test_environment_variable_exclusion() {
    let executor_env = HashMap::from([
        ("KEEP_VAR".to_string(), "keep_value".to_string()),
        ("EXCLUDE_VAR".to_string(), "exclude_value".to_string()),
        ("ANOTHER_EXCLUDE".to_string(), "another_exclude".to_string()),
    ]);
    
    let config = EnvironmentConfig {
        inherit_parent: false,
        inherit_executor: true,
        exclude: vec!["EXCLUDE_VAR".to_string(), "ANOTHER_EXCLUDE".to_string()],
        ..Default::default()
    };
    
    let manager = EnvironmentManager::new(executor_env, config);
    let task_env = HashMap::new();
    
    let result = manager.build_task_environment(&task_env, None).unwrap();
    
    // Check that non-excluded variables are present
    assert!(result.contains_key("KEEP_VAR"));
    
    // Check that excluded variables are not present
    assert!(!result.contains_key("EXCLUDE_VAR"));
    assert!(!result.contains_key("ANOTHER_EXCLUDE"));
}

#[test]
fn test_secure_environment_variable_detection() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);
    
    let task_env = HashMap::from([
        ("API_KEY".to_string(), "secret123".to_string()),
        ("DATABASE_PASSWORD".to_string(), "password123".to_string()),
        ("SECRET_TOKEN".to_string(), "token123".to_string()),
        ("PUBLIC_URL".to_string(), "https://example.com".to_string()),
        ("NORMAL_VAR".to_string(), "normal_value".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, None).unwrap();
    
    // Check that variables with sensitive names are marked as secure
    assert!(result.get("API_KEY").unwrap().is_secure());
    assert!(result.get("DATABASE_PASSWORD").unwrap().is_secure());
    assert!(result.get("SECRET_TOKEN").unwrap().is_secure());
    
    // Check that normal variables are not marked as secure
    assert!(!result.get("PUBLIC_URL").unwrap().is_secure());
    assert!(!result.get("NORMAL_VAR").unwrap().is_secure());
}

#[test]
fn test_environment_variable_interpolation() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);
    
    let mut context = InterpolationContext::new();
    context.add_variable("BASE_PATH".to_string(), "/app".to_string());
    context.add_variable("VERSION".to_string(), "1.0.0".to_string());
    
    let task_env = HashMap::from([
        ("FULL_PATH".to_string(), "${BASE_PATH}/data".to_string()),
        ("APP_VERSION".to_string(), "v${VERSION}".to_string()),
        ("COMPLEX_PATH".to_string(), "${BASE_PATH}/logs/v${VERSION}".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();
    
    assert_eq!(result.get("FULL_PATH").unwrap().value(), "/app/data");
    assert_eq!(result.get("APP_VERSION").unwrap().value(), "v1.0.0");
    assert_eq!(result.get("COMPLEX_PATH").unwrap().value(), "/app/logs/v1.0.0");
}

#[test]
fn test_task_output_interpolation() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);
    
    let mut context = InterpolationContext::new();
    context.add_task_result(
        "task1".to_string(),
        "output_file.txt".to_string(),
        "warning: deprecated".to_string(),
        Some(0),
        "Success".to_string(),
    );
    context.add_task_result(
        "task2".to_string(),
        "42".to_string(),
        "".to_string(),
        Some(0),
        "Success".to_string(),
    );
    
    let task_env = HashMap::from([
        ("INPUT_FILE".to_string(), "${task.task1.stdout}".to_string()),
        ("WARNING_MSG".to_string(), "${task.task1.stderr}".to_string()),
        ("EXIT_STATUS".to_string(), "${task.task1.exit_code}".to_string()),
        ("TASK_STATUS".to_string(), "${task.task1.status}".to_string()),
        ("COUNT".to_string(), "${task.task2.stdout}".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();
    
    assert_eq!(result.get("INPUT_FILE").unwrap().value(), "output_file.txt");
    assert_eq!(result.get("WARNING_MSG").unwrap().value(), "warning: deprecated");
    assert_eq!(result.get("EXIT_STATUS").unwrap().value(), "0");
    assert_eq!(result.get("TASK_STATUS").unwrap().value(), "Success");
    assert_eq!(result.get("COUNT").unwrap().value(), "42");
}

#[test]
fn test_system_info_interpolation() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig::default();
    let manager = EnvironmentManager::new(executor_env, config);
    
    let context = InterpolationContext::new();
    
    let task_env = HashMap::from([
        ("LOG_FILE".to_string(), "/logs/${system.hostname}.log".to_string()),
        ("TIMESTAMP".to_string(), "${system.timestamp}".to_string()),
        ("OS_INFO".to_string(), "${system.os}".to_string()),
    ]);
    
    let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();
    
    // Check that system variables are interpolated (exact values may vary by system)
    let log_file = result.get("LOG_FILE").unwrap().value();
    assert!(log_file.starts_with("/logs/"));
    assert!(log_file.ends_with(".log"));
    assert!(!log_file.contains("${system.hostname}"));
    
    let timestamp = result.get("TIMESTAMP").unwrap().value();
    assert!(!timestamp.contains("${system.timestamp}"));
    
    let os_info = result.get("OS_INFO").unwrap().value();
    assert!(!os_info.is_empty());
    assert!(!os_info.contains("${system.os}"));
}

#[test]
fn test_circular_reference_detection() {
    let executor_env = HashMap::new();
    let config = EnvironmentConfig {
        variables: HashMap::from([
            ("VAR_A".to_string(), "${VAR_B}".to_string()),
            ("VAR_B".to_string(), "${VAR_C}".to_string()),
            ("VAR_C".to_string(), "${VAR_A}".to_string()),
        ]),
        ..Default::default()
    };
    
    let manager = EnvironmentManager::new(executor_env, config);
    
    // Should detect circular reference and return error
    assert!(manager.validate_config().is_err());
}

#[test]
fn test_dataflow_static_parameter_resolution() {
    let dataflow = DataflowManager::new();
    
    let mut parameters = HashMap::new();
    parameters.insert("static_param".to_string(), ParameterValue::Static {
        value: "static_value".to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["static_param"].value, "static_value");
    assert!(!resolved["static_param"].is_sensitive);
}

#[test]
fn test_dataflow_task_output_parameter_resolution() {
    let mut dataflow = DataflowManager::new();
    
    // Add task output
    let task_output = TaskOutput {
        task_id: "build_task".to_string(),
        stdout: "build_output.jar".to_string(),
        stderr: "warning: deprecated API".to_string(),
        exit_code: Some(0),
        data: None,
        duration: Duration::from_secs(30),
        completed_at: Utc::now(),
    };
    dataflow.add_task_output(task_output);
    
    // Resolve parameters from task output
    let mut parameters = HashMap::new();
    parameters.insert("artifact_file".to_string(), ParameterValue::TaskOutput {
        task_id: "build_task".to_string(),
        field: "stdout".to_string(),
    });
    parameters.insert("build_warnings".to_string(), ParameterValue::TaskOutput {
        task_id: "build_task".to_string(),
        field: "stderr".to_string(),
    });
    parameters.insert("build_status".to_string(), ParameterValue::TaskOutput {
        task_id: "build_task".to_string(),
        field: "exit_code".to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["artifact_file"].value, "build_output.jar");
    assert_eq!(resolved["build_warnings"].value, "warning: deprecated API");
    assert_eq!(resolved["build_status"].value, "0");
}

#[test]
fn test_dataflow_task_data_parameter_resolution() {
    let mut dataflow = DataflowManager::new();
    
    // Add task output with structured data
    let task_output = TaskOutput {
        task_id: "analysis_task".to_string(),
        stdout: "".to_string(),
        stderr: "".to_string(),
        exit_code: Some(0),
        data: Some(serde_json::json!({
            "results": {
                "total_count": 150,
                "success_rate": 0.95,
                "errors": ["timeout", "network_error"]
            },
            "metadata": {
                "version": "2.1.0",
                "timestamp": "2023-10-01T12:00:00Z"
            }
        })),
        duration: Duration::from_secs(60),
        completed_at: Utc::now(),
    };
    dataflow.add_task_output(task_output);
    
    // Resolve parameters from structured data
    let mut parameters = HashMap::new();
    parameters.insert("total_count".to_string(), ParameterValue::TaskData {
        task_id: "analysis_task".to_string(),
        json_path: "$.results.total_count".to_string(),
    });
    parameters.insert("success_rate".to_string(), ParameterValue::TaskData {
        task_id: "analysis_task".to_string(),
        json_path: "$.results.success_rate".to_string(),
    });
    parameters.insert("version".to_string(), ParameterValue::TaskData {
        task_id: "analysis_task".to_string(),
        json_path: "$.metadata.version".to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["total_count"].value, "150");
    assert_eq!(resolved["success_rate"].value, "0.95");
    assert_eq!(resolved["version"].value, "2.1.0");
}

#[test]
fn test_dataflow_secure_parameter_resolution() {
    let mut dataflow = DataflowManager::new();
    
    // Add secure parameters
    let api_key = SecureParameter::new("api_key".to_string(), "secret-api-key-123".to_string());
    let db_password = SecureParameter::new("db_password".to_string(), "super-secret-password".to_string());
    let public_config = SecureParameter::plain("public_url".to_string(), "https://api.example.com".to_string());
    
    dataflow.add_secure_parameter(api_key);
    dataflow.add_secure_parameter(db_password);
    dataflow.add_secure_parameter(public_config);
    
    // Resolve parameters
    let mut parameters = HashMap::new();
    parameters.insert("auth_token".to_string(), ParameterValue::SecureReference {
        parameter_name: "api_key".to_string(),
    });
    parameters.insert("database_password".to_string(), ParameterValue::SecureReference {
        parameter_name: "db_password".to_string(),
    });
    parameters.insert("endpoint_url".to_string(), ParameterValue::SecureReference {
        parameter_name: "public_url".to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["auth_token"].value, "secret-api-key-123");
    assert!(resolved["auth_token"].is_sensitive);
    
    assert_eq!(resolved["database_password"].value, "super-secret-password");
    assert!(resolved["database_password"].is_sensitive);
    
    assert_eq!(resolved["endpoint_url"].value, "https://api.example.com");
    assert!(!resolved["endpoint_url"].is_sensitive);
}

#[test]
fn test_dataflow_expression_evaluation() {
    let dataflow = DataflowManager::new();
    
    let mut parameters = HashMap::new();
    parameters.insert("concat_result".to_string(), ParameterValue::Expression {
        expression: r#"concat("hello", "world")"#.to_string(),
    });
    parameters.insert("upper_result".to_string(), ParameterValue::Expression {
        expression: r#"upper("test string")"#.to_string(),
    });
    parameters.insert("lower_result".to_string(), ParameterValue::Expression {
        expression: r#"lower("TEST STRING")"#.to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["concat_result"].value, "helloworld");
    assert_eq!(resolved["upper_result"].value, "TEST STRING");
    assert_eq!(resolved["lower_result"].value, "test string");
}

#[test]
fn test_dataflow_parameter_validation() {
    let mut dataflow = DataflowManager::new();
    
    // Add validation rules
    let number_validation = ParameterValidation {
        parameter_type: ParameterType::Number,
        required: true,
        min_value: Some(0.0),
        max_value: Some(100.0),
        ..Default::default()
    };
    
    let string_validation = ParameterValidation {
        parameter_type: ParameterType::String,
        required: true,
        pattern: Some(r"^[a-zA-Z0-9_]+$".to_string()),
        ..Default::default()
    };
    
    let enum_validation = ParameterValidation {
        parameter_type: ParameterType::String,
        required: true,
        allowed_values: Some(vec!["dev".to_string(), "staging".to_string(), "prod".to_string()]),
        ..Default::default()
    };
    
    dataflow.add_validation_rule("score".to_string(), number_validation);
    dataflow.add_validation_rule("identifier".to_string(), string_validation);
    dataflow.add_validation_rule("environment".to_string(), enum_validation);
    
    // Test valid parameters
    let mut valid_parameters = HashMap::new();
    valid_parameters.insert("score".to_string(), ParameterValue::Static {
        value: "85".to_string(),
    });
    valid_parameters.insert("identifier".to_string(), ParameterValue::Static {
        value: "user_123".to_string(),
    });
    valid_parameters.insert("environment".to_string(), ParameterValue::Static {
        value: "prod".to_string(),
    });
    
    assert!(dataflow.resolve_parameters(&valid_parameters).is_ok());
    
    // Test invalid number (out of range)
    let mut invalid_parameters = HashMap::new();
    invalid_parameters.insert("score".to_string(), ParameterValue::Static {
        value: "150".to_string(),
    });
    assert!(dataflow.resolve_parameters(&invalid_parameters).is_err());
    
    // Test invalid string (doesn't match pattern)
    invalid_parameters.clear();
    invalid_parameters.insert("identifier".to_string(), ParameterValue::Static {
        value: "user-123!".to_string(),
    });
    assert!(dataflow.resolve_parameters(&invalid_parameters).is_err());
    
    // Test invalid enum value
    invalid_parameters.clear();
    invalid_parameters.insert("environment".to_string(), ParameterValue::Static {
        value: "invalid_env".to_string(),
    });
    assert!(dataflow.resolve_parameters(&invalid_parameters).is_err());
}

#[test]
fn test_dataflow_parameter_chaining() {
    let mut dataflow = DataflowManager::new();
    
    // Add multiple task outputs to simulate a pipeline
    let task1_output = TaskOutput {
        task_id: "extract_data".to_string(),
        stdout: "raw_data.csv".to_string(),
        stderr: "".to_string(),
        exit_code: Some(0),
        data: Some(serde_json::json!({
            "record_count": 1000,
            "file_size": "2.5MB"
        })),
        duration: Duration::from_secs(10),
        completed_at: Utc::now(),
    };
    
    let task2_output = TaskOutput {
        task_id: "transform_data".to_string(),
        stdout: "processed_data.json".to_string(),
        stderr: "".to_string(),
        exit_code: Some(0),
        data: Some(serde_json::json!({
            "processed_records": 950,
            "validation_errors": 50
        })),
        duration: Duration::from_secs(30),
        completed_at: Utc::now(),
    };
    
    dataflow.add_task_output(task1_output);
    dataflow.add_task_output(task2_output);
    
    // Chain parameters through multiple tasks
    let mut parameters = HashMap::new();
    parameters.insert("input_file".to_string(), ParameterValue::TaskOutput {
        task_id: "extract_data".to_string(),
        field: "stdout".to_string(),
    });
    parameters.insert("processed_file".to_string(), ParameterValue::TaskOutput {
        task_id: "transform_data".to_string(),
        field: "stdout".to_string(),
    });
    parameters.insert("original_count".to_string(), ParameterValue::TaskData {
        task_id: "extract_data".to_string(),
        json_path: "$.record_count".to_string(),
    });
    parameters.insert("processed_count".to_string(), ParameterValue::TaskData {
        task_id: "transform_data".to_string(),
        json_path: "$.processed_records".to_string(),
    });
    parameters.insert("error_count".to_string(), ParameterValue::TaskData {
        task_id: "transform_data".to_string(),
        json_path: "$.validation_errors".to_string(),
    });
    
    let resolved = dataflow.resolve_parameters(&parameters).unwrap();
    
    assert_eq!(resolved["input_file"].value, "raw_data.csv");
    assert_eq!(resolved["processed_file"].value, "processed_data.json");
    assert_eq!(resolved["original_count"].value, "1000");
    assert_eq!(resolved["processed_count"].value, "950");
    assert_eq!(resolved["error_count"].value, "50");
    
    // Verify parameter sources are correctly tracked
    assert!(matches!(resolved["input_file"].source, jorm::executor::ParameterSource::TaskOutput { .. }));
    assert!(matches!(resolved["original_count"].source, jorm::executor::ParameterSource::TaskData { .. }));
}

#[test]
fn test_dataflow_error_handling() {
    let dataflow = DataflowManager::new();
    
    // Test missing task output
    let mut parameters = HashMap::new();
    parameters.insert("missing_output".to_string(), ParameterValue::TaskOutput {
        task_id: "nonexistent_task".to_string(),
        field: "stdout".to_string(),
    });
    
    assert!(dataflow.resolve_parameters(&parameters).is_err());
    
    // Test invalid JSON path
    let mut dataflow_with_data = DataflowManager::new();
    let task_output = TaskOutput {
        task_id: "test_task".to_string(),
        stdout: "".to_string(),
        stderr: "".to_string(),
        exit_code: Some(0),
        data: Some(serde_json::json!({"field": "value"})),
        duration: Duration::from_secs(1),
        completed_at: Utc::now(),
    };
    dataflow_with_data.add_task_output(task_output);
    
    parameters.clear();
    parameters.insert("invalid_path".to_string(), ParameterValue::TaskData {
        task_id: "test_task".to_string(),
        json_path: "$.nonexistent.field".to_string(),
    });
    
    assert!(dataflow_with_data.resolve_parameters(&parameters).is_err());
    
    // Test missing secure parameter
    parameters.clear();
    parameters.insert("missing_secure".to_string(), ParameterValue::SecureReference {
        parameter_name: "nonexistent_param".to_string(),
    });
    
    assert!(dataflow.resolve_parameters(&parameters).is_err());
}