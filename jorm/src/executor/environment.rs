//! Environment variable management and interpolation for task execution

use crate::executor::ExecutorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;

#[cfg(feature = "hostname")]
use hostname;

/// Environment variable configuration for tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment variables to set for this task
    pub variables: HashMap<String, String>,
    /// Whether to inherit environment from parent process
    pub inherit_parent: bool,
    /// Whether to inherit environment from executor config
    pub inherit_executor: bool,
    /// Environment variables to explicitly exclude/unset
    pub exclude: Vec<String>,
    /// Whether to enable variable interpolation
    pub enable_interpolation: bool,
    /// Prefix for secure environment variables (will be masked in logs)
    pub secure_prefixes: Vec<String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            inherit_parent: true,
            inherit_executor: true,
            exclude: Vec::new(),
            enable_interpolation: true,
            secure_prefixes: vec![
                "PASSWORD".to_string(),
                "SECRET".to_string(),
                "TOKEN".to_string(),
                "KEY".to_string(),
                "CREDENTIAL".to_string(),
            ],
        }
    }
}

/// Environment variable manager for secure handling and interpolation
#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    /// Base environment variables from executor config
    executor_env: HashMap<String, String>,
    /// Environment configuration
    config: EnvironmentConfig,
}

/// Environment variable interpolation context
#[derive(Debug, Clone)]
pub struct InterpolationContext {
    /// Variables available for interpolation
    pub variables: HashMap<String, String>,
    /// Task outputs from previous tasks (stdout)
    pub task_outputs: HashMap<String, String>,
    /// Task stderr outputs from previous tasks
    pub task_stderr: HashMap<String, String>,
    /// Task exit codes from previous tasks
    pub task_exit_codes: HashMap<String, i32>,
    /// Task statuses from previous tasks
    pub task_statuses: HashMap<String, String>,
    /// System information
    pub system_info: HashMap<String, String>,
}

/// Secure environment variable value that masks sensitive data in logs
#[derive(Clone, PartialEq, Eq)]
pub struct SecureValue {
    value: String,
    is_secure: bool,
}

impl SecureValue {
    /// Create a new secure value
    pub fn new(value: String, is_secure: bool) -> Self {
        Self { value, is_secure }
    }

    /// Create a secure value (will be masked in logs)
    pub fn secure(value: String) -> Self {
        Self {
            value,
            is_secure: true,
        }
    }

    /// Create a non-secure value (will be shown in logs)
    pub fn plain(value: String) -> Self {
        Self {
            value,
            is_secure: false,
        }
    }

    /// Get the actual value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Check if this value is secure
    pub fn is_secure(&self) -> bool {
        self.is_secure
    }

    /// Convert to string for process environment
    pub fn to_string(&self) -> String {
        self.value.clone()
    }
}

impl fmt::Debug for SecureValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_secure {
            write!(f, "SecureValue(***)")
        } else {
            write!(f, "SecureValue({})", self.value)
        }
    }
}

impl fmt::Display for SecureValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_secure {
            write!(f, "***")
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl EnvironmentManager {
    /// Create a new environment manager
    pub fn new(executor_env: HashMap<String, String>, config: EnvironmentConfig) -> Self {
        Self {
            executor_env,
            config,
        }
    }

    /// Create environment manager with default configuration
    pub fn with_executor_env(executor_env: HashMap<String, String>) -> Self {
        Self::new(executor_env, EnvironmentConfig::default())
    }

    /// Build environment variables for a task
    pub fn build_task_environment(
        &self,
        task_env: &HashMap<String, String>,
        interpolation_context: Option<&InterpolationContext>,
    ) -> Result<HashMap<String, SecureValue>, ExecutorError> {
        let mut env_vars = HashMap::new();

        // 1. Start with parent process environment if enabled
        if self.config.inherit_parent {
            for (key, value) in env::vars() {
                if !self.config.exclude.contains(&key) {
                    let is_secure = self.is_secure_variable(&key);
                    env_vars.insert(key, SecureValue::new(value, is_secure));
                }
            }
        }

        // 2. Add executor-level environment variables if enabled
        if self.config.inherit_executor {
            for (key, value) in &self.executor_env {
                if !self.config.exclude.contains(key) {
                    let is_secure = self.is_secure_variable(key);
                    env_vars.insert(key.clone(), SecureValue::new(value.clone(), is_secure));
                }
            }
        }

        // 3. Add task-specific environment variables
        for (key, value) in task_env {
            if !self.config.exclude.contains(key) {
                let processed_value = if self.config.enable_interpolation {
                    if let Some(context) = interpolation_context {
                        self.interpolate_value(value, context)?
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                };

                let is_secure = self.is_secure_variable(key);
                env_vars.insert(key.clone(), SecureValue::new(processed_value, is_secure));
            }
        }

        // 4. Add configuration-level environment variables
        for (key, value) in &self.config.variables {
            if !self.config.exclude.contains(key) {
                let processed_value = if self.config.enable_interpolation {
                    if let Some(context) = interpolation_context {
                        self.interpolate_value(value, context)?
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                };

                let is_secure = self.is_secure_variable(key);
                env_vars.insert(key.clone(), SecureValue::new(processed_value, is_secure));
            }
        }

        // 5. Remove explicitly excluded variables
        for excluded in &self.config.exclude {
            env_vars.remove(excluded);
        }

        Ok(env_vars)
    }

    /// Check if a variable name indicates it contains secure data
    fn is_secure_variable(&self, key: &str) -> bool {
        let key_upper = key.to_uppercase();
        self.config.secure_prefixes.iter().any(|prefix| {
            key_upper.contains(&prefix.to_uppercase())
        })
    }

    /// Interpolate environment variable value with context
    fn interpolate_value(
        &self,
        value: &str,
        context: &InterpolationContext,
    ) -> Result<String, ExecutorError> {
        let mut result = value.to_string();

        // Replace ${VAR} and $VAR patterns
        result = self.replace_variable_references(&result, context)?;

        // Replace task output references ${task.output}
        result = self.replace_task_output_references(&result, context)?;

        // Replace system info references ${system.info}
        result = self.replace_system_info_references(&result, context)?;

        Ok(result)
    }

    /// Replace variable references like ${VAR} or $VAR
    fn replace_variable_references(
        &self,
        value: &str,
        context: &InterpolationContext,
    ) -> Result<String, ExecutorError> {
        let mut result = value.to_string();

        // Handle ${VAR} format
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let replacement = if let Some(var_value) = context.variables.get(var_name) {
                    var_value.clone()
                } else if let Ok(env_value) = env::var(var_name) {
                    env_value
                } else {
                    format!("${{{}}}", var_name)
                };

                result.replace_range(start..start + end + 1, &replacement);
            } else {
                break; // Malformed variable reference
            }
        }

        // Handle $VAR format (simple variable names only)
        let words: Vec<&str> = result.split_whitespace().collect();
        let mut new_words = Vec::new();

        for word in words {
            if word.starts_with('$') && word.len() > 1 {
                let var_name = &word[1..];
                // Only replace if it's a valid variable name (alphanumeric + underscore)
                if var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    let replacement = if let Some(var_value) = context.variables.get(var_name) {
                        var_value.clone()
                    } else if let Ok(env_value) = env::var(var_name) {
                        env_value
                    } else {
                        word.to_string()
                    };
                    new_words.push(replacement);
                } else {
                    new_words.push(word.to_string());
                }
            } else {
                new_words.push(word.to_string());
            }
        }

        if !new_words.is_empty() {
            result = new_words.join(" ");
        }

        Ok(result)
    }

    /// Replace task output references like ${task.task_id.stdout}
    fn replace_task_output_references(
        &self,
        value: &str,
        context: &InterpolationContext,
    ) -> Result<String, ExecutorError> {
        let mut result = value.to_string();

        // Handle ${task.task_id.field} format
        while let Some(start) = result.find("${task.") {
            if let Some(end) = result[start..].find('}') {
                let reference = &result[start + 7..start + end]; // Skip "${task."
                let parts: Vec<&str> = reference.split('.').collect();

                if parts.len() >= 2 {
                    let task_id = parts[0];
                    let field = parts[1];

                    // Support multiple task output fields
                    let replacement_string = match field {
                        "stdout" => {
                            context.task_outputs.get(task_id)
                                .cloned()
                                .unwrap_or_else(|| format!("${{task.{}.{}}}", task_id, field))
                        }
                        "stderr" => {
                            // Get stderr from task outputs if available
                            context.task_stderr.get(task_id)
                                .cloned()
                                .unwrap_or_else(|| format!("${{task.{}.{}}}", task_id, field))
                        }
                        "exit_code" => {
                            // Get exit code from task outputs if available
                            context.task_exit_codes.get(task_id)
                                .map(|code| code.to_string())
                                .unwrap_or_else(|| format!("${{task.{}.{}}}", task_id, field))
                        }
                        "status" => {
                            // Get task status if available
                            context.task_statuses.get(task_id)
                                .cloned()
                                .unwrap_or_else(|| format!("${{task.{}.{}}}", task_id, field))
                        }
                        _ => format!("${{task.{}.{}}}", task_id, field)
                    };

                    result.replace_range(start..start + end + 1, &replacement_string);
                } else {
                    break; // Malformed task reference
                }
            } else {
                break; // Malformed task reference
            }
        }

        Ok(result)
    }

    /// Replace system info references like ${system.hostname}
    fn replace_system_info_references(
        &self,
        value: &str,
        context: &InterpolationContext,
    ) -> Result<String, ExecutorError> {
        let mut result = value.to_string();

        // Handle ${system.info} format
        while let Some(start) = result.find("${system.") {
            if let Some(end) = result[start..].find('}') {
                let info_key = &result[start + 9..start + end]; // Skip "${system."
                let fallback = format!("${{system.{}}}", info_key);
                let replacement = context
                    .system_info
                    .get(info_key)
                    .map(|s| s.as_str())
                    .unwrap_or(&fallback);

                result.replace_range(start..start + end + 1, replacement);
            } else {
                break; // Malformed system reference
            }
        }

        Ok(result)
    }

    /// Convert secure environment to process environment (strings only)
    pub fn to_process_env(env_vars: &HashMap<String, SecureValue>) -> Vec<(String, String)> {
        env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect()
    }

    /// Log environment variables (masking secure ones)
    pub fn log_environment(env_vars: &HashMap<String, SecureValue>, task_id: &str) {
        println!("🌍 Environment variables for task '{}':", task_id);
        let mut sorted_vars: Vec<_> = env_vars.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| k.as_str());

        for (key, value) in sorted_vars {
            println!("  {}={}", key, value);
        }
    }

    /// Validate environment configuration
    pub fn validate_config(&self) -> Result<(), ExecutorError> {
        // Check for circular references in variables
        for (key, value) in &self.config.variables {
            if self.has_circular_reference(key, value, &self.config.variables)? {
                return Err(ExecutorError::ConfigurationError {
                    message: format!("Circular reference detected in environment variable: {}", key),
                });
            }
        }

        Ok(())
    }

    /// Check for circular references in variable definitions
    fn has_circular_reference(
        &self,
        key: &str,
        value: &str,
        all_vars: &HashMap<String, String>,
    ) -> Result<bool, ExecutorError> {
        let mut visited = std::collections::HashSet::new();
        self.check_circular_reference_recursive(key, value, all_vars, &mut visited)
    }

    /// Recursive helper for circular reference detection
    fn check_circular_reference_recursive(
        &self,
        current_key: &str,
        current_value: &str,
        all_vars: &HashMap<String, String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<bool, ExecutorError> {
        if visited.contains(current_key) {
            return Ok(true); // Circular reference found
        }

        visited.insert(current_key.to_string());

        // Find variable references in the current value
        let mut pos = 0;
        while let Some(start) = current_value[pos..].find("${") {
            let start = pos + start;
            if let Some(end) = current_value[start..].find('}') {
                let var_name = &current_value[start + 2..start + end];
                if let Some(referenced_value) = all_vars.get(var_name) {
                    if self.check_circular_reference_recursive(
                        var_name,
                        referenced_value,
                        all_vars,
                        visited,
                    )? {
                        return Ok(true);
                    }
                }
                pos = start + end + 1;
            } else {
                break;
            }
        }

        visited.remove(current_key);
        Ok(false)
    }
}

impl InterpolationContext {
    /// Create a new interpolation context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            task_outputs: HashMap::new(),
            task_stderr: HashMap::new(),
            task_exit_codes: HashMap::new(),
            task_statuses: HashMap::new(),
            system_info: Self::gather_system_info(),
        }
    }

    /// Create context with variables
    pub fn with_variables(variables: HashMap<String, String>) -> Self {
        Self {
            variables,
            task_outputs: HashMap::new(),
            task_stderr: HashMap::new(),
            task_exit_codes: HashMap::new(),
            task_statuses: HashMap::new(),
            system_info: Self::gather_system_info(),
        }
    }

    /// Add a variable to the context
    pub fn add_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Add task output to the context (stdout only - for backward compatibility)
    pub fn add_task_output(&mut self, task_id: String, stdout: String) {
        self.task_outputs.insert(task_id, stdout);
    }

    /// Add complete task result to the context
    pub fn add_task_result(&mut self, task_id: String, stdout: String, stderr: String, exit_code: Option<i32>, status: String) {
        self.task_outputs.insert(task_id.clone(), stdout);
        self.task_stderr.insert(task_id.clone(), stderr);
        if let Some(code) = exit_code {
            self.task_exit_codes.insert(task_id.clone(), code);
        }
        self.task_statuses.insert(task_id, status);
    }

    /// Get task output (stdout)
    pub fn get_task_output(&self, task_id: &str) -> Option<&String> {
        self.task_outputs.get(task_id)
    }

    /// Get task stderr
    pub fn get_task_stderr(&self, task_id: &str) -> Option<&String> {
        self.task_stderr.get(task_id)
    }

    /// Get task exit code
    pub fn get_task_exit_code(&self, task_id: &str) -> Option<i32> {
        self.task_exit_codes.get(task_id).copied()
    }

    /// Get task status
    pub fn get_task_status(&self, task_id: &str) -> Option<&String> {
        self.task_statuses.get(task_id)
    }

    /// Clear all task data
    pub fn clear_task_data(&mut self) {
        self.task_outputs.clear();
        self.task_stderr.clear();
        self.task_exit_codes.clear();
        self.task_statuses.clear();
    }

    /// Gather system information for interpolation
    fn gather_system_info() -> HashMap<String, String> {
        let mut info = HashMap::new();

        // Add hostname (if available)
        #[cfg(feature = "hostname")]
        {
            if let Ok(hostname_result) = hostname::get() {
                if let Some(hostname_str) = hostname_result.to_str() {
                    info.insert("hostname".to_string(), hostname_str.to_string());
                }
            }
        }
        
        // Fallback hostname detection
        #[cfg(not(feature = "hostname"))]
        {
            if let Ok(hostname_env) = env::var("HOSTNAME").or_else(|_| env::var("COMPUTERNAME")) {
                info.insert("hostname".to_string(), hostname_env);
            } else {
                info.insert("hostname".to_string(), "localhost".to_string());
            }
        }

        // Add current user
        if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
            info.insert("user".to_string(), user);
        }

        // Add current working directory
        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                info.insert("cwd".to_string(), cwd_str.to_string());
            }
        }

        // Add platform information
        info.insert("os".to_string(), env::consts::OS.to_string());
        info.insert("arch".to_string(), env::consts::ARCH.to_string());

        // Add timestamp
        info.insert(
            "timestamp".to_string(),
            chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string(),
        );

        info
    }
}

impl Default for InterpolationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_value_display() {
        let secure = SecureValue::secure("secret123".to_string());
        let plain = SecureValue::plain("public_value".to_string());

        assert_eq!(format!("{}", secure), "***");
        assert_eq!(format!("{}", plain), "public_value");
        assert_eq!(secure.value(), "secret123");
        assert_eq!(plain.value(), "public_value");
    }

    #[test]
    fn test_environment_manager_basic() {
        let executor_env = HashMap::from([
            ("EXECUTOR_VAR".to_string(), "executor_value".to_string()),
        ]);
        let config = EnvironmentConfig::default();
        let manager = EnvironmentManager::new(executor_env, config);

        let task_env = HashMap::from([
            ("TASK_VAR".to_string(), "task_value".to_string()),
        ]);

        let result = manager.build_task_environment(&task_env, None).unwrap();

        assert!(result.contains_key("EXECUTOR_VAR"));
        assert!(result.contains_key("TASK_VAR"));
        assert_eq!(result.get("TASK_VAR").unwrap().value(), "task_value");
    }

    #[test]
    fn test_variable_interpolation() {
        let executor_env = HashMap::new();
        let config = EnvironmentConfig::default();
        let manager = EnvironmentManager::new(executor_env, config);

        let mut context = InterpolationContext::new();
        context.add_variable("BASE_PATH".to_string(), "/app".to_string());

        let task_env = HashMap::from([
            ("FULL_PATH".to_string(), "${BASE_PATH}/data".to_string()),
        ]);

        let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

        assert_eq!(result.get("FULL_PATH").unwrap().value(), "/app/data");
    }

    #[test]
    fn test_task_output_interpolation() {
        let executor_env = HashMap::new();
        let config = EnvironmentConfig::default();
        let manager = EnvironmentManager::new(executor_env, config);

        let mut context = InterpolationContext::new();
        context.add_task_result(
            "task1".to_string(), 
            "output_value".to_string(), 
            "error_output".to_string(), 
            Some(0), 
            "Success".to_string()
        );

        let task_env = HashMap::from([
            ("PREVIOUS_OUTPUT".to_string(), "${task.task1.stdout}".to_string()),
            ("PREVIOUS_ERROR".to_string(), "${task.task1.stderr}".to_string()),
            ("PREVIOUS_EXIT_CODE".to_string(), "${task.task1.exit_code}".to_string()),
            ("PREVIOUS_STATUS".to_string(), "${task.task1.status}".to_string()),
        ]);

        let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

        assert_eq!(result.get("PREVIOUS_OUTPUT").unwrap().value(), "output_value");
        assert_eq!(result.get("PREVIOUS_ERROR").unwrap().value(), "error_output");
        assert_eq!(result.get("PREVIOUS_EXIT_CODE").unwrap().value(), "0");
        assert_eq!(result.get("PREVIOUS_STATUS").unwrap().value(), "Success");
    }

    #[test]
    fn test_secure_variable_detection() {
        let executor_env = HashMap::new();
        let config = EnvironmentConfig::default();
        let manager = EnvironmentManager::new(executor_env, config);

        let task_env = HashMap::from([
            ("API_KEY".to_string(), "secret123".to_string()),
            ("DATABASE_PASSWORD".to_string(), "password123".to_string()),
            ("PUBLIC_URL".to_string(), "https://example.com".to_string()),
        ]);

        let result = manager.build_task_environment(&task_env, None).unwrap();

        assert!(result.get("API_KEY").unwrap().is_secure());
        assert!(result.get("DATABASE_PASSWORD").unwrap().is_secure());
        assert!(!result.get("PUBLIC_URL").unwrap().is_secure());
    }

    #[test]
    fn test_environment_exclusion() {
        let executor_env = HashMap::from([
            ("KEEP_VAR".to_string(), "keep".to_string()),
            ("EXCLUDE_VAR".to_string(), "exclude".to_string()),
        ]);
        let config = EnvironmentConfig {
            exclude: vec!["EXCLUDE_VAR".to_string()],
            ..Default::default()
        };
        let manager = EnvironmentManager::new(executor_env, config);

        let task_env = HashMap::new();
        let result = manager.build_task_environment(&task_env, None).unwrap();

        assert!(result.contains_key("KEEP_VAR"));
        assert!(!result.contains_key("EXCLUDE_VAR"));
    }

    #[test]
    fn test_circular_reference_detection() {
        let executor_env = HashMap::new();
        let config = EnvironmentConfig {
            variables: HashMap::from([
                ("VAR_A".to_string(), "${VAR_B}".to_string()),
                ("VAR_B".to_string(), "${VAR_A}".to_string()),
            ]),
            ..Default::default()
        };
        let manager = EnvironmentManager::new(executor_env, config);

        assert!(manager.validate_config().is_err());
    }

    #[test]
    fn test_system_info_interpolation() {
        let executor_env = HashMap::new();
        let config = EnvironmentConfig::default();
        let manager = EnvironmentManager::new(executor_env, config);

        let context = InterpolationContext::new();
        let task_env = HashMap::from([
            ("LOG_FILE".to_string(), "/logs/${system.hostname}.log".to_string()),
        ]);

        let result = manager.build_task_environment(&task_env, Some(&context)).unwrap();

        let log_file = result.get("LOG_FILE").unwrap().value();
        assert!(log_file.starts_with("/logs/"));
        assert!(log_file.ends_with(".log"));
        assert!(!log_file.contains("${system.hostname}"));
    }
}