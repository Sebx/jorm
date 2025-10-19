//! Task output chaining and parameter passing
//!
//! This module provides functionality for passing data between tasks in a DAG execution.
//! It supports secure parameter handling, output capture, and parameter substitution.
//!
//! # Features
//!
//! - **Output Capture**: Capture and store task outputs for use by dependent tasks
//! - **Parameter Substitution**: Replace parameter placeholders with actual values from previous tasks
//! - **Secure Handling**: Mask sensitive data in logs and ensure secure parameter passing
//! - **Type Safety**: Support for different data types (strings, numbers, JSON objects)
//! - **Validation**: Validate parameter types and formats before task execution
//!
//! # Examples
//!
//! ## Basic Parameter Passing
//!
//! ```rust
//! use jorm::executor::{DataflowManager, TaskOutput, ParameterValue};
//! use std::collections::HashMap;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut dataflow = DataflowManager::new();
//!
//! // Add output from a previous task
//! let task_output = TaskOutput {
//!     task_id: "task1".to_string(),
//!     stdout: "processed_data.json".to_string(),
//!     stderr: String::new(),
//!     exit_code: Some(0),
//!     data: Some(serde_json::json!({"result": "success", "count": 42})),
//!     duration: std::time::Duration::from_secs(1),
//!     completed_at: chrono::Utc::now(),
//! };
//!
//! dataflow.add_task_output(task_output);
//!
//! // Use output in parameter substitution
//! let mut parameters = HashMap::new();
//! parameters.insert("input_file".to_string(), ParameterValue::TaskOutput {
//!     task_id: "task1".to_string(),
//!     field: "stdout".to_string(),
//! });
//! parameters.insert("record_count".to_string(), ParameterValue::TaskData {
//!     task_id: "task1".to_string(),
//!     json_path: "$.count".to_string(),
//! });
//!
//! let resolved = dataflow.resolve_parameters(&parameters)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Secure Parameter Handling
//!
//! ```rust
//! use jorm::executor::{DataflowManager, ParameterValue, SecureParameter};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut dataflow = DataflowManager::new();
//!
//! // Add secure parameter (will be masked in logs)
//! let secure_param = SecureParameter::new("api_key".to_string(), "secret-key-123".to_string());
//! dataflow.add_secure_parameter(secure_param);
//!
//! // Use in parameter resolution
//! let mut parameters = std::collections::HashMap::new();
//! parameters.insert("auth_token".to_string(), ParameterValue::SecureReference {
//!     parameter_name: "api_key".to_string(),
//! });
//!
//! let resolved = dataflow.resolve_parameters(&parameters)?;
//! # Ok(())
//! # }
//! ```

use crate::executor::{ExecutorError, TaskOutput};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Manager for task output chaining and parameter passing
#[derive(Debug, Clone)]
pub struct DataflowManager {
    /// Task outputs from completed tasks
    task_outputs: HashMap<String, TaskOutput>,
    /// Secure parameters that should be masked in logs
    secure_parameters: HashMap<String, SecureParameter>,
    /// Parameter validation rules
    validation_rules: HashMap<String, ParameterValidation>,
}

/// A parameter value that can be resolved from various sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterValue {
    /// Static string value
    Static { value: String },
    
    /// Reference to task output field (stdout, stderr, exit_code)
    TaskOutput { task_id: String, field: String },
    
    /// Reference to structured data from task output using JSON path
    TaskData { task_id: String, json_path: String },
    
    /// Reference to a secure parameter
    SecureReference { parameter_name: String },
    
    /// Environment variable reference
    Environment { variable_name: String },
    
    /// Computed value using expression
    Expression { expression: String },
}

/// Secure parameter that will be masked in logs
#[derive(Debug, Clone)]
pub struct SecureParameter {
    name: String,
    value: String,
    is_sensitive: bool,
}

/// Parameter validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValidation {
    /// Parameter type (string, number, boolean, json)
    pub parameter_type: ParameterType,
    /// Whether the parameter is required
    pub required: bool,
    /// Pattern validation (regex for strings)
    pub pattern: Option<String>,
    /// Minimum value (for numbers)
    pub min_value: Option<f64>,
    /// Maximum value (for numbers)
    pub max_value: Option<f64>,
    /// Allowed values (enum validation)
    pub allowed_values: Option<Vec<String>>,
}

/// Parameter data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Json,
    Array,
}

/// Resolved parameter with its value and metadata
#[derive(Debug, Clone)]
pub struct ResolvedParameter {
    pub name: String,
    pub value: String,
    pub parameter_type: ParameterType,
    pub is_sensitive: bool,
    pub source: ParameterSource,
}

/// Source of a resolved parameter
#[derive(Debug, Clone)]
pub enum ParameterSource {
    Static,
    TaskOutput { task_id: String, field: String },
    TaskData { task_id: String, json_path: String },
    SecureParameter { parameter_name: String },
    Environment { variable_name: String },
    Expression { expression: String },
}

impl DataflowManager {
    /// Create a new dataflow manager
    pub fn new() -> Self {
        Self {
            task_outputs: HashMap::new(),
            secure_parameters: HashMap::new(),
            validation_rules: HashMap::new(),
        }
    }

    /// Add task output for use in parameter resolution
    pub fn add_task_output(&mut self, output: TaskOutput) {
        println!("📊 Adding task output for dataflow: {}", output.task_id);
        self.task_outputs.insert(output.task_id.clone(), output);
    }

    /// Add secure parameter
    pub fn add_secure_parameter(&mut self, parameter: SecureParameter) {
        println!("🔐 Adding secure parameter: {}", parameter.name);
        self.secure_parameters.insert(parameter.name.clone(), parameter);
    }

    /// Add parameter validation rule
    pub fn add_validation_rule(&mut self, parameter_name: String, validation: ParameterValidation) {
        self.validation_rules.insert(parameter_name, validation);
    }

    /// Resolve parameters from their definitions
    pub fn resolve_parameters(
        &self,
        parameters: &HashMap<String, ParameterValue>,
    ) -> Result<HashMap<String, ResolvedParameter>, ExecutorError> {
        let mut resolved = HashMap::new();

        for (param_name, param_value) in parameters {
            let resolved_param = self.resolve_single_parameter(param_name, param_value)?;
            
            // Validate the resolved parameter
            if let Some(validation) = self.validation_rules.get(param_name) {
                self.validate_parameter(&resolved_param, validation)?;
            }
            
            resolved.insert(param_name.clone(), resolved_param);
        }

        Ok(resolved)
    }

    /// Resolve a single parameter value
    fn resolve_single_parameter(
        &self,
        param_name: &str,
        param_value: &ParameterValue,
    ) -> Result<ResolvedParameter, ExecutorError> {
        match param_value {
            ParameterValue::Static { value } => Ok(ResolvedParameter {
                name: param_name.to_string(),
                value: value.clone(),
                parameter_type: ParameterType::String,
                is_sensitive: false,
                source: ParameterSource::Static,
            }),

            ParameterValue::TaskOutput { task_id, field } => {
                let task_output = self.task_outputs.get(task_id)
                    .ok_or_else(|| ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("Task output not found: {}", task_id),
                    })?;

                let value = match field.as_str() {
                    "stdout" => task_output.stdout.clone(),
                    "stderr" => task_output.stderr.clone(),
                    "exit_code" => task_output.exit_code.map(|c| c.to_string()).unwrap_or_default(),
                    _ => return Err(ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("Unknown task output field: {}", field),
                    }),
                };

                Ok(ResolvedParameter {
                    name: param_name.to_string(),
                    value,
                    parameter_type: ParameterType::String,
                    is_sensitive: false,
                    source: ParameterSource::TaskOutput {
                        task_id: task_id.clone(),
                        field: field.clone(),
                    },
                })
            }

            ParameterValue::TaskData { task_id, json_path } => {
                let task_output = self.task_outputs.get(task_id)
                    .ok_or_else(|| ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("Task output not found: {}", task_id),
                    })?;

                let data = task_output.data.as_ref()
                    .ok_or_else(|| ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("No structured data available for task: {}", task_id),
                    })?;

                // Simple JSON path resolution (could be enhanced with a proper JSON path library)
                let value = self.resolve_json_path(data, json_path)?;

                Ok(ResolvedParameter {
                    name: param_name.to_string(),
                    value,
                    parameter_type: ParameterType::Json,
                    is_sensitive: false,
                    source: ParameterSource::TaskData {
                        task_id: task_id.clone(),
                        json_path: json_path.clone(),
                    },
                })
            }

            ParameterValue::SecureReference { parameter_name } => {
                let secure_param = self.secure_parameters.get(parameter_name)
                    .ok_or_else(|| ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("Secure parameter not found: {}", parameter_name),
                    })?;

                Ok(ResolvedParameter {
                    name: param_name.to_string(),
                    value: secure_param.value.clone(),
                    parameter_type: ParameterType::String,
                    is_sensitive: secure_param.is_sensitive,
                    source: ParameterSource::SecureParameter {
                        parameter_name: parameter_name.clone(),
                    },
                })
            }

            ParameterValue::Environment { variable_name } => {
                let value = std::env::var(variable_name)
                    .map_err(|_| ExecutorError::ParameterResolutionError {
                        parameter: param_name.to_string(),
                        message: format!("Environment variable not found: {}", variable_name),
                    })?;

                // Check if this environment variable should be treated as sensitive
                let is_sensitive = variable_name.to_uppercase().contains("PASSWORD") ||
                                 variable_name.to_uppercase().contains("SECRET") ||
                                 variable_name.to_uppercase().contains("TOKEN") ||
                                 variable_name.to_uppercase().contains("KEY");

                Ok(ResolvedParameter {
                    name: param_name.to_string(),
                    value,
                    parameter_type: ParameterType::String,
                    is_sensitive,
                    source: ParameterSource::Environment {
                        variable_name: variable_name.clone(),
                    },
                })
            }

            ParameterValue::Expression { expression } => {
                // Simple expression evaluation (could be enhanced with a proper expression engine)
                let value = self.evaluate_expression(expression)?;

                Ok(ResolvedParameter {
                    name: param_name.to_string(),
                    value,
                    parameter_type: ParameterType::String,
                    is_sensitive: false,
                    source: ParameterSource::Expression {
                        expression: expression.clone(),
                    },
                })
            }
        }
    }

    /// Simple JSON path resolution
    fn resolve_json_path(&self, data: &serde_json::Value, json_path: &str) -> Result<String, ExecutorError> {
        // Simple implementation - supports basic paths like $.field, $.field.subfield
        if json_path.starts_with("$.") {
            let path = &json_path[2..]; // Remove "$."
            let parts: Vec<&str> = path.split('.').collect();
            
            let mut current = data;
            for part in parts {
                current = current.get(part)
                    .ok_or_else(|| ExecutorError::ParameterResolutionError {
                        parameter: "json_path".to_string(),
                        message: format!("JSON path not found: {}", json_path),
                    })?;
            }
            
            // Convert to string
            match current {
                serde_json::Value::String(s) => Ok(s.clone()),
                serde_json::Value::Number(n) => Ok(n.to_string()),
                serde_json::Value::Bool(b) => Ok(b.to_string()),
                serde_json::Value::Null => Ok("null".to_string()),
                _ => Ok(current.to_string()),
            }
        } else {
            Err(ExecutorError::ParameterResolutionError {
                parameter: "json_path".to_string(),
                message: format!("Invalid JSON path format: {}", json_path),
            })
        }
    }

    /// Simple expression evaluation
    fn evaluate_expression(&self, expression: &str) -> Result<String, ExecutorError> {
        // Simple implementation - could be enhanced with a proper expression engine
        // For now, just support basic string operations
        if expression.starts_with("concat(") && expression.ends_with(')') {
            let inner = &expression[7..expression.len()-1]; // Remove "concat(" and ")"
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_matches('"')).collect();
            Ok(parts.join(""))
        } else if expression.starts_with("upper(") && expression.ends_with(')') {
            let inner = &expression[6..expression.len()-1]; // Remove "upper(" and ")"
            Ok(inner.trim_matches('"').to_uppercase())
        } else if expression.starts_with("lower(") && expression.ends_with(')') {
            let inner = &expression[6..expression.len()-1]; // Remove "lower(" and ")"
            Ok(inner.trim_matches('"').to_lowercase())
        } else {
            // Treat as literal string
            Ok(expression.to_string())
        }
    }

    /// Validate a resolved parameter against its validation rules
    fn validate_parameter(
        &self,
        parameter: &ResolvedParameter,
        validation: &ParameterValidation,
    ) -> Result<(), ExecutorError> {
        // Type validation
        match validation.parameter_type {
            ParameterType::Number => {
                parameter.value.parse::<f64>()
                    .map_err(|_| ExecutorError::ParameterValidationError {
                        parameter: parameter.name.clone(),
                        message: "Parameter must be a valid number".to_string(),
                    })?;
            }
            ParameterType::Boolean => {
                parameter.value.parse::<bool>()
                    .map_err(|_| ExecutorError::ParameterValidationError {
                        parameter: parameter.name.clone(),
                        message: "Parameter must be a valid boolean".to_string(),
                    })?;
            }
            ParameterType::Json => {
                serde_json::from_str::<serde_json::Value>(&parameter.value)
                    .map_err(|_| ExecutorError::ParameterValidationError {
                        parameter: parameter.name.clone(),
                        message: "Parameter must be valid JSON".to_string(),
                    })?;
            }
            _ => {} // String and Array types don't need special validation
        }

        // Pattern validation
        if let Some(pattern) = &validation.pattern {
            let regex = regex::Regex::new(pattern)
                .map_err(|_| ExecutorError::ParameterValidationError {
                    parameter: parameter.name.clone(),
                    message: format!("Invalid regex pattern: {}", pattern),
                })?;
            
            if !regex.is_match(&parameter.value) {
                return Err(ExecutorError::ParameterValidationError {
                    parameter: parameter.name.clone(),
                    message: format!("Parameter does not match pattern: {}", pattern),
                });
            }
        }

        // Range validation for numbers
        if matches!(validation.parameter_type, ParameterType::Number) {
            if let Ok(num_value) = parameter.value.parse::<f64>() {
                if let Some(min) = validation.min_value {
                    if num_value < min {
                        return Err(ExecutorError::ParameterValidationError {
                            parameter: parameter.name.clone(),
                            message: format!("Parameter value {} is below minimum {}", num_value, min),
                        });
                    }
                }
                if let Some(max) = validation.max_value {
                    if num_value > max {
                        return Err(ExecutorError::ParameterValidationError {
                            parameter: parameter.name.clone(),
                            message: format!("Parameter value {} is above maximum {}", num_value, max),
                        });
                    }
                }
            }
        }

        // Allowed values validation
        if let Some(allowed) = &validation.allowed_values {
            if !allowed.contains(&parameter.value) {
                return Err(ExecutorError::ParameterValidationError {
                    parameter: parameter.name.clone(),
                    message: format!("Parameter value '{}' is not in allowed values: {:?}", parameter.value, allowed),
                });
            }
        }

        Ok(())
    }

    /// Get task output by ID
    pub fn get_task_output(&self, task_id: &str) -> Option<&TaskOutput> {
        self.task_outputs.get(task_id)
    }

    /// Check if task output exists
    pub fn has_task_output(&self, task_id: &str) -> bool {
        self.task_outputs.contains_key(task_id)
    }

    /// Get all task output IDs
    pub fn task_output_ids(&self) -> Vec<&String> {
        self.task_outputs.keys().collect()
    }

    /// Clear all task outputs
    pub fn clear_task_outputs(&mut self) {
        self.task_outputs.clear();
    }

    /// Log resolved parameters (masking sensitive ones)
    pub fn log_resolved_parameters(&self, parameters: &HashMap<String, ResolvedParameter>) {
        println!("🔗 Resolved parameters:");
        for (name, param) in parameters {
            let display_value = if param.is_sensitive {
                "***".to_string()
            } else {
                param.value.clone()
            };
            println!("  {} = {} (from {:?})", name, display_value, param.source);
        }
    }

    /// Convert resolved parameters to environment variables
    pub fn parameters_to_env_vars(&self, parameters: &HashMap<String, ResolvedParameter>) -> HashMap<String, String> {
        parameters.iter()
            .map(|(name, param)| (name.clone(), param.value.clone()))
            .collect()
    }
}

impl SecureParameter {
    /// Create a new secure parameter
    pub fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            is_sensitive: true,
        }
    }

    /// Create a non-sensitive parameter
    pub fn plain(name: String, value: String) -> Self {
        Self {
            name,
            value,
            is_sensitive: false,
        }
    }

    /// Get parameter name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get parameter value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Check if parameter is sensitive
    pub fn is_sensitive(&self) -> bool {
        self.is_sensitive
    }
}

impl fmt::Display for SecureParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_sensitive {
            write!(f, "{}=***", self.name)
        } else {
            write!(f, "{}={}", self.name, self.value)
        }
    }
}

impl Default for DataflowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParameterValidation {
    fn default() -> Self {
        Self {
            parameter_type: ParameterType::String,
            required: false,
            pattern: None,
            min_value: None,
            max_value: None,
            allowed_values: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    #[test]
    fn test_static_parameter_resolution() {
        let dataflow = DataflowManager::new();
        let mut parameters = HashMap::new();
        parameters.insert("test_param".to_string(), ParameterValue::Static {
            value: "test_value".to_string(),
        });

        let resolved = dataflow.resolve_parameters(&parameters).unwrap();
        assert_eq!(resolved["test_param"].value, "test_value");
        assert!(!resolved["test_param"].is_sensitive);
    }

    #[test]
    fn test_task_output_parameter_resolution() {
        let mut dataflow = DataflowManager::new();
        
        // Add task output
        let task_output = TaskOutput {
            task_id: "task1".to_string(),
            stdout: "output_data".to_string(),
            stderr: "error_data".to_string(),
            exit_code: Some(0),
            data: None,
            duration: Duration::from_secs(1),
            completed_at: Utc::now(),
        };
        dataflow.add_task_output(task_output);

        // Resolve parameters
        let mut parameters = HashMap::new();
        parameters.insert("stdout_param".to_string(), ParameterValue::TaskOutput {
            task_id: "task1".to_string(),
            field: "stdout".to_string(),
        });
        parameters.insert("stderr_param".to_string(), ParameterValue::TaskOutput {
            task_id: "task1".to_string(),
            field: "stderr".to_string(),
        });

        let resolved = dataflow.resolve_parameters(&parameters).unwrap();
        assert_eq!(resolved["stdout_param"].value, "output_data");
        assert_eq!(resolved["stderr_param"].value, "error_data");
    }

    #[test]
    fn test_task_data_parameter_resolution() {
        let mut dataflow = DataflowManager::new();
        
        // Add task output with structured data
        let task_output = TaskOutput {
            task_id: "task1".to_string(),
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            data: Some(serde_json::json!({
                "result": "success",
                "count": 42,
                "nested": {
                    "value": "nested_data"
                }
            })),
            duration: Duration::from_secs(1),
            completed_at: Utc::now(),
        };
        dataflow.add_task_output(task_output);

        // Resolve parameters
        let mut parameters = HashMap::new();
        parameters.insert("result_param".to_string(), ParameterValue::TaskData {
            task_id: "task1".to_string(),
            json_path: "$.result".to_string(),
        });
        parameters.insert("count_param".to_string(), ParameterValue::TaskData {
            task_id: "task1".to_string(),
            json_path: "$.count".to_string(),
        });
        parameters.insert("nested_param".to_string(), ParameterValue::TaskData {
            task_id: "task1".to_string(),
            json_path: "$.nested.value".to_string(),
        });

        let resolved = dataflow.resolve_parameters(&parameters).unwrap();
        assert_eq!(resolved["result_param"].value, "success");
        assert_eq!(resolved["count_param"].value, "42");
        assert_eq!(resolved["nested_param"].value, "nested_data");
    }

    #[test]
    fn test_secure_parameter_resolution() {
        let mut dataflow = DataflowManager::new();
        
        // Add secure parameter
        let secure_param = SecureParameter::new("api_key".to_string(), "secret123".to_string());
        dataflow.add_secure_parameter(secure_param);

        // Resolve parameters
        let mut parameters = HashMap::new();
        parameters.insert("auth_token".to_string(), ParameterValue::SecureReference {
            parameter_name: "api_key".to_string(),
        });

        let resolved = dataflow.resolve_parameters(&parameters).unwrap();
        assert_eq!(resolved["auth_token"].value, "secret123");
        assert!(resolved["auth_token"].is_sensitive);
    }

    #[test]
    fn test_expression_evaluation() {
        let dataflow = DataflowManager::new();
        
        let mut parameters = HashMap::new();
        parameters.insert("concat_param".to_string(), ParameterValue::Expression {
            expression: r#"concat("hello", "world")"#.to_string(),
        });
        parameters.insert("upper_param".to_string(), ParameterValue::Expression {
            expression: r#"upper("test")"#.to_string(),
        });

        let resolved = dataflow.resolve_parameters(&parameters).unwrap();
        assert_eq!(resolved["concat_param"].value, "helloworld");
        assert_eq!(resolved["upper_param"].value, "TEST");
    }

    #[test]
    fn test_parameter_validation() {
        let mut dataflow = DataflowManager::new();
        
        // Add validation rule
        let validation = ParameterValidation {
            parameter_type: ParameterType::Number,
            required: true,
            min_value: Some(0.0),
            max_value: Some(100.0),
            ..Default::default()
        };
        dataflow.add_validation_rule("number_param".to_string(), validation);

        // Test valid parameter
        let mut parameters = HashMap::new();
        parameters.insert("number_param".to_string(), ParameterValue::Static {
            value: "50".to_string(),
        });
        assert!(dataflow.resolve_parameters(&parameters).is_ok());

        // Test invalid parameter (out of range)
        parameters.insert("number_param".to_string(), ParameterValue::Static {
            value: "150".to_string(),
        });
        assert!(dataflow.resolve_parameters(&parameters).is_err());

        // Test invalid parameter (not a number)
        parameters.insert("number_param".to_string(), ParameterValue::Static {
            value: "not_a_number".to_string(),
        });
        assert!(dataflow.resolve_parameters(&parameters).is_err());
    }
}