//! HTTP task executor implementation

use crate::executor::{
    AuthConfig, ExecutionContext, ExecutorError, Task, TaskConfig, TaskResult, TaskStatus,
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// HTTP task executor for making HTTP requests
pub struct HttpTaskExecutor {
    /// HTTP client for making requests
    client: Client,

    /// Default timeout for HTTP requests
    default_timeout: Duration,
}

impl HttpTaskExecutor {
    /// Create a new HTTP task executor
    pub fn new() -> Result<Self, ExecutorError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ExecutorError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            client,
            default_timeout: Duration::from_secs(30),
        })
    }

    /// Create a new HTTP task executor with custom timeout
    pub fn with_timeout(timeout: Duration) -> Result<Self, ExecutorError> {
        let client = Client::builder().timeout(timeout).build().map_err(|e| {
            ExecutorError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            }
        })?;

        Ok(Self {
            client,
            default_timeout: timeout,
        })
    }

    /// Create a new HTTP task executor with custom client configuration
    pub fn with_client_config<F>(config_fn: F) -> Result<Self, ExecutorError>
    where
        F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
    {
        let builder = Client::builder();
        let client = config_fn(builder)
            .build()
            .map_err(|e| ExecutorError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            client,
            default_timeout: Duration::from_secs(30),
        })
    }

    /// Execute an HTTP request with the given configuration
    async fn execute_http_request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        auth: Option<&AuthConfig>,
        request_timeout: Option<Duration>,
        context: &ExecutionContext,
        task_timeout: Duration,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        println!("🌐 Making HTTP request: {} {}", method.to_uppercase(), url);

        // Parse HTTP method
        let http_method = Method::from_str(method.to_uppercase().as_str()).map_err(|e| {
            ExecutorError::ConfigurationError {
                message: format!("Invalid HTTP method '{}': {}", method, e),
            }
        })?;

        // Build the request
        let mut request_builder = self.client.request(http_method, url);

        // Add headers
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
            println!("📋 Header: {}: {}", key, value);
        }

        // Add authentication
        if let Some(auth_config) = auth {
            request_builder = self.apply_authentication(request_builder, auth_config)?;
        }

        // Add body if provided
        if let Some(body_content) = body {
            request_builder = request_builder.body(body_content.to_string());
            println!("📝 Request body: {} bytes", body_content.len());
        }

        // Apply request-specific timeout if provided
        if let Some(req_timeout) = request_timeout {
            request_builder = request_builder.timeout(req_timeout);
        }

        // Build the request
        let request = request_builder
            .build()
            .map_err(|e| ExecutorError::ConfigurationError {
                message: format!("Failed to build HTTP request: {}", e),
            })?;

        // Execute the request with task timeout
        let execution_result = timeout(task_timeout, self.client.execute(request)).await;

        let duration = start_time.elapsed();
        let completed_at = Utc::now();

        self.process_http_response(
            execution_result,
            context,
            duration,
            started_at,
            completed_at,
            task_timeout,
        )
        .await
    }

    /// Apply authentication to the request builder
    fn apply_authentication(
        &self,
        mut request_builder: RequestBuilder,
        auth: &AuthConfig,
    ) -> Result<RequestBuilder, ExecutorError> {
        match auth {
            AuthConfig::Basic { username, password } => {
                println!("🔐 Using Basic authentication for user: {}", username);
                request_builder = request_builder.basic_auth(username, Some(password));
            }
            AuthConfig::Bearer { token } => {
                println!("🔐 Using Bearer token authentication");
                request_builder = request_builder.bearer_auth(token);
            }
            AuthConfig::ApiKey { key, header } => {
                println!("🔐 Using API key authentication with header: {}", header);
                request_builder = request_builder.header(header, key);
            }
        }

        Ok(request_builder)
    }

    /// Process the HTTP response and create a TaskResult
    async fn process_http_response(
        &self,
        execution_result: Result<Result<Response, reqwest::Error>, tokio::time::error::Elapsed>,
        context: &ExecutionContext,
        duration: Duration,
        started_at: chrono::DateTime<Utc>,
        completed_at: chrono::DateTime<Utc>,
        task_timeout: Duration,
    ) -> Result<TaskResult, ExecutorError> {
        match execution_result {
            Ok(Ok(response)) => {
                let status_code = response.status().as_u16();
                let headers = response.headers().clone();

                println!(
                    "📡 HTTP response: {} {}",
                    status_code,
                    response.status().canonical_reason().unwrap_or("")
                );

                // Log response headers
                for (name, value) in headers.iter() {
                    if let Ok(value_str) = value.to_str() {
                        println!("📋 Response header: {}: {}", name, value_str);
                    }
                }

                // Read response body
                let response_body =
                    response
                        .text()
                        .await
                        .map_err(|e| ExecutorError::TaskExecutionFailed {
                            task_id: context.task_id.clone(),
                            source: anyhow::anyhow!("Failed to read response body: {}", e),
                        })?;

                println!("📤 Response body: {} bytes", response_body.len());
                if response_body.len() <= 500 {
                    println!("📄 Response content: {}", response_body.trim());
                }

                // Determine task status based on HTTP status code
                let status = if (200..300).contains(&status_code) {
                    TaskStatus::Success
                } else {
                    TaskStatus::Failed
                };

                // Try to parse response as JSON for structured output
                let output_data = if response_body.trim().starts_with('{')
                    || response_body.trim().starts_with('[')
                {
                    match serde_json::from_str::<serde_json::Value>(&response_body) {
                        Ok(json) => Some(json),
                        Err(_) => None,
                    }
                } else {
                    None
                };

                // Create response metadata
                let mut metadata = HashMap::new();
                metadata.insert("status_code".to_string(), serde_json::json!(status_code));
                metadata.insert(
                    "content_length".to_string(),
                    serde_json::json!(response_body.len()),
                );

                // Add response headers to metadata
                let mut response_headers = HashMap::new();
                for (name, value) in headers.iter() {
                    if let Ok(value_str) = value.to_str() {
                        response_headers.insert(name.to_string(), value_str.to_string());
                    }
                }
                metadata.insert(
                    "response_headers".to_string(),
                    serde_json::json!(response_headers),
                );

                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status,
                    stdout: response_body,
                    stderr: String::new(),
                    exit_code: Some(status_code as i32),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data,
                    error_message: if status == TaskStatus::Failed {
                        Some(format!(
                            "HTTP request failed with status code: {}",
                            status_code
                        ))
                    } else {
                        None
                    },
                    metadata,
                })
            }
            Ok(Err(reqwest_error)) => {
                let error_msg = format!("HTTP request failed: {}", reqwest_error);
                println!("❌ {}", error_msg);

                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(reqwest_error),
                })
            }
            Err(_timeout_error) => {
                let error_msg = format!("HTTP request timed out after {:?}", task_timeout);
                println!("⏰ {}", error_msg);

                Err(ExecutorError::TaskTimeout {
                    task_id: context.task_id.clone(),
                    timeout: task_timeout,
                })
            }
        }
    }
}

#[async_trait]
impl crate::executor::TaskExecutor for HttpTaskExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        // Extract HTTP configuration from task
        match &task.config {
            TaskConfig::HttpRequest {
                method,
                url,
                headers,
                body,
                auth,
                timeout,
            } => {
                // Determine timeout (task-specific, request-specific, or default)
                let task_timeout = task.effective_timeout(self.default_timeout);

                // Execute the HTTP request
                self.execute_http_request(
                    method,
                    url,
                    headers,
                    body.as_deref(),
                    auth.as_ref(),
                    *timeout,
                    context,
                    task_timeout,
                )
                .await
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected HTTP request task, got: {:?}", task.config),
            }),
        }
    }

    fn task_type(&self) -> &'static str {
        "http"
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
        match &task.config {
            TaskConfig::HttpRequest {
                method,
                url,
                headers: _,
                body: _,
                auth: _,
                timeout: _,
            } => {
                // Validate HTTP method
                if method.trim().is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "HTTP method cannot be empty".to_string(),
                    });
                }

                // Validate method is supported
                if Method::from_str(method.to_uppercase().as_str()).is_err() {
                    return Err(ExecutorError::ConfigurationError {
                        message: format!("Unsupported HTTP method: {}", method),
                    });
                }

                // Validate URL
                if url.trim().is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "HTTP URL cannot be empty".to_string(),
                    });
                }

                // Basic URL validation
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(ExecutorError::ConfigurationError {
                        message: format!("Invalid URL format: {}", url),
                    });
                }

                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: "Expected HTTP request task configuration".to_string(),
            }),
        }
    }
}

impl Default for HttpTaskExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP executor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExecutionContext, ExecutorConfig, TaskExecutor};

    #[tokio::test]
    async fn test_http_executor_creation() {
        let executor = HttpTaskExecutor::new().unwrap();
        assert_eq!(executor.task_type(), "http");
        assert!(executor.supports_parallel());
        assert_eq!(executor.default_timeout(), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_http_executor_with_custom_timeout() {
        let timeout = Duration::from_secs(60);
        let executor = HttpTaskExecutor::with_timeout(timeout).unwrap();
        assert_eq!(executor.default_timeout(), timeout);
    }

    #[tokio::test]
    async fn test_http_task_validation() {
        let executor = HttpTaskExecutor::new().unwrap();

        // Valid HTTP task
        let valid_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "GET".to_string(),
                url: "https://httpbin.org/get".to_string(),
                headers: HashMap::new(),
                body: None,
                auth: None,
                timeout: None,
            },
        );

        assert!(executor.validate_task(&valid_task).is_ok());

        // Invalid HTTP task (empty method)
        let invalid_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "".to_string(),
                url: "https://httpbin.org/get".to_string(),
                headers: HashMap::new(),
                body: None,
                auth: None,
                timeout: None,
            },
        );

        assert!(executor.validate_task(&invalid_task).is_err());

        // Invalid HTTP task (invalid URL)
        let invalid_url_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "GET".to_string(),
                url: "not-a-url".to_string(),
                headers: HashMap::new(),
                body: None,
                auth: None,
                timeout: None,
            },
        );

        assert!(executor.validate_task(&invalid_url_task).is_err());
    }

    #[tokio::test]
    async fn test_http_get_request() {
        let executor = HttpTaskExecutor::new().unwrap();
        let config = ExecutorConfig::default();
        let context =
            ExecutionContext::new("test_dag".to_string(), "test_task".to_string(), config);

        let task = Task::new(
            "http_get_test".to_string(),
            "HTTP GET Test".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "GET".to_string(),
                url: "https://httpbin.org/get".to_string(),
                headers: HashMap::new(),
                body: None,
                auth: None,
                timeout: None,
            },
        );

        let result = executor.execute(&task, &context).await;

        // This test might fail if network is not available, so we'll handle that gracefully
        match result {
            Ok(task_result) => {
                assert_eq!(task_result.status, TaskStatus::Success);
                assert_eq!(task_result.exit_code, Some(200));
                assert!(!task_result.stdout.is_empty());

                // Check if response contains expected JSON structure
                if let Some(output_data) = &task_result.output_data {
                    // httpbin.org/get returns JSON with request info
                    assert!(output_data.is_object());
                }

                // Check metadata
                assert!(task_result.metadata.contains_key("status_code"));
                assert_eq!(task_result.metadata["status_code"], serde_json::json!(200));
            }
            Err(e) => {
                // Network might not be available in test environment
                let error_msg = e.to_string();
                if error_msg.contains("network")
                    || error_msg.contains("dns")
                    || error_msg.contains("connection")
                {
                    println!("Network not available in test environment, skipping HTTP test");
                } else {
                    panic!("Unexpected error in HTTP test: {:?}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_http_post_request_with_body() {
        let executor = HttpTaskExecutor::new().unwrap();
        let config = ExecutorConfig::default();
        let context =
            ExecutionContext::new("test_dag".to_string(), "test_task".to_string(), config);

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let task = Task::new(
            "http_post_test".to_string(),
            "HTTP POST Test".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "POST".to_string(),
                url: "https://httpbin.org/post".to_string(),
                headers,
                body: Some(r#"{"test": "data"}"#.to_string()),
                auth: None,
                timeout: None,
            },
        );

        let result = executor.execute(&task, &context).await;

        // This test might fail if network is not available
        match result {
            Ok(task_result) => {
                assert_eq!(task_result.status, TaskStatus::Success);
                assert_eq!(task_result.exit_code, Some(200));
                assert!(!task_result.stdout.is_empty());

                // Check if response contains the posted data
                if let Some(output_data) = &task_result.output_data {
                    if let Some(json_data) = output_data.get("json") {
                        assert_eq!(json_data.get("test"), Some(&serde_json::json!("data")));
                    }
                }
            }
            Err(e) => {
                // Network might not be available in test environment
                let error_msg = e.to_string();
                if error_msg.contains("network")
                    || error_msg.contains("dns")
                    || error_msg.contains("connection")
                {
                    println!("Network not available in test environment, skipping HTTP POST test");
                } else {
                    panic!("Unexpected error in HTTP POST test: {:?}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_http_authentication() {
        let executor = HttpTaskExecutor::new().unwrap();
        let config = ExecutorConfig::default();
        let context =
            ExecutionContext::new("test_dag".to_string(), "test_task".to_string(), config);

        let task = Task::new(
            "http_auth_test".to_string(),
            "HTTP Auth Test".to_string(),
            "http".to_string(),
            TaskConfig::HttpRequest {
                method: "GET".to_string(),
                url: "https://httpbin.org/basic-auth/user/pass".to_string(),
                headers: HashMap::new(),
                body: None,
                auth: Some(AuthConfig::Basic {
                    username: "user".to_string(),
                    password: "pass".to_string(),
                }),
                timeout: None,
            },
        );

        let result = executor.execute(&task, &context).await;

        // This test might fail if network is not available
        match result {
            Ok(task_result) => {
                assert_eq!(task_result.status, TaskStatus::Success);
                assert_eq!(task_result.exit_code, Some(200));

                // Check if authentication was successful
                if let Some(output_data) = &task_result.output_data {
                    if let Some(authenticated) = output_data.get("authenticated") {
                        assert_eq!(authenticated, &serde_json::json!(true));
                    }
                }
            }
            Err(e) => {
                // Network might not be available in test environment
                let error_msg = e.to_string();
                if error_msg.contains("network")
                    || error_msg.contains("dns")
                    || error_msg.contains("connection")
                {
                    println!("Network not available in test environment, skipping HTTP auth test");
                } else {
                    panic!("Unexpected error in HTTP auth test: {:?}", e);
                }
            }
        }
    }
}
