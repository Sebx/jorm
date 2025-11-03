use crate::core::{error::JormError, task::TaskType};
use crate::core::engine::TaskResult;
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

pub struct HttpExecutor {
    client: Client,
}

impl HttpExecutor {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn execute(&self, task_name: &str, task_type: &TaskType) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Http { method, url, headers, body } => {
                self.execute_http_request(task_name, method, url, headers.as_ref(), body.as_deref()).await
            }
            _ => Err(JormError::ExecutionError(
                format!("HTTP executor cannot handle task type: {:?}", task_type)
            )),
        }
    }

    async fn execute_http_request(
        &self,
        task_name: &str,
        method: &str,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        body: Option<&str>,
    ) -> Result<TaskResult, JormError> {
        // Parse HTTP method
        let http_method = Method::from_str(&method.to_uppercase())
            .map_err(|_| JormError::HttpError(format!("Invalid HTTP method: {}", method)))?;

        // Build request
        let mut request_builder = self.client.request(http_method, url);

        // Add headers if provided
        if let Some(headers_map) = headers {
            for (key, value) in headers_map {
                request_builder = request_builder.header(key, value);
            }
        }

        // Add body if provided
        if let Some(body_content) = body {
            request_builder = request_builder.body(body_content.to_string());
        }

        // Execute request
        match request_builder.send().await {
            Ok(response) => {
                let status = response.status();
                let success = status.is_success();
                
                // Get response body
                let response_text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => format!("Failed to read response body: {}", e),
                };

                let output = format!("Status: {}\nResponse: {}", status, response_text);
                let error = if !success {
                    Some(format!("HTTP request failed with status: {}", status))
                } else {
                    None
                };

                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success,
                    output,
                    error,
                })
            }
            Err(e) => {
                let error_msg = format!("HTTP request to {} failed: {}", url, e);
                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success: false,
                    output: String::new(),
                    error: Some(error_msg.clone()),
                })
            }
        }
    }
}