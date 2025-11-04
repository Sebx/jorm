use crate::core::engine::{ExecutionResult, JormEngine};
use crate::core::error::JormError;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ExecuteFileRequest {
    pub dag_file: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteNlRequest {
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub success: bool,
    pub message: String,
    pub execution_id: Option<String>,
    pub results: Option<ExecutionResult>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub struct HttpServer {
    engine: Arc<JormEngine>,
    port: u16,
    auth_token: Option<String>,
}

impl HttpServer {
    pub fn new(engine: Arc<JormEngine>, port: u16) -> Self {
        Self {
            engine,
            port,
            auth_token: std::env::var("JORM_AUTH_TOKEN").ok(),
        }
    }

    pub fn with_auth_token(engine: Arc<JormEngine>, port: u16, auth_token: String) -> Self {
        Self {
            engine,
            port,
            auth_token: Some(auth_token),
        }
    }

    pub async fn start(&self) -> Result<(), JormError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let engine = Arc::clone(&self.engine);
        let auth_token = self.auth_token.clone();

        let make_svc = make_service_fn(move |_conn| {
            let engine = Arc::clone(&engine);
            let auth_token = auth_token.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let engine = Arc::clone(&engine);
                    let auth_token = auth_token.clone();
                    handle_request(req, engine, auth_token)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        log_info(&format!("HTTP server listening on http://{}", addr));
        if self.auth_token.is_some() {
            log_info("Authentication enabled - use Authorization: Bearer <token> header");
        } else {
            log_warn(
                "Authentication disabled - set JORM_AUTH_TOKEN environment variable to enable",
            );
        }

        if let Err(e) = server.await {
            return Err(JormError::ServerError(format!("Server error: {}", e)));
        }

        Ok(())
    }
}

async fn handle_request(
    req: Request<Body>,
    engine: Arc<JormEngine>,
    auth_token: Option<String>,
) -> Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start_time = std::time::Instant::now();

    log_info(&format!("Incoming request: {} {}", method, path));

    let response = match (req.method(), req.uri().path()) {
        (&Method::POST, "/execute") => {
            if let Some(auth_error) = check_authentication(req.headers(), &auth_token) {
                auth_error
            } else {
                handle_execute_file(req, engine).await
            }
        }
        (&Method::POST, "/execute/nl") => {
            if let Some(auth_error) = check_authentication(req.headers(), &auth_token) {
                auth_error
            } else {
                handle_execute_nl(req, engine).await
            }
        }
        (&Method::GET, "/health") => handle_health().await,
        _ => handle_not_found().await,
    };

    let duration = start_time.elapsed();
    log_info(&format!(
        "Request completed: {} {} - {} - {:?}",
        method,
        path,
        response.status(),
        duration
    ));

    Ok(response)
}

async fn handle_execute_file(req: Request<Body>, engine: Arc<JormEngine>) -> Response<Body> {
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to read request body: {}", e);
            log_error(&error_msg);
            return create_error_response(
                StatusCode::BAD_REQUEST,
                "Invalid request body",
                &error_msg,
            );
        }
    };

    let request: ExecuteFileRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            let error_msg = format!("Failed to parse JSON: {}", e);
            log_error(&error_msg);
            return create_error_response(StatusCode::BAD_REQUEST, "Invalid JSON", &error_msg);
        }
    };

    log_info(&format!("Executing DAG file: {}", request.dag_file));

    match engine.execute_from_file(&request.dag_file).await {
        Ok(result) => {
            let execution_id = generate_execution_id();
            log_info(&format!(
                "DAG execution completed - ID: {} - Success: {}",
                execution_id, result.success
            ));

            let response = ExecuteResponse {
                success: result.success,
                message: result.message.clone(),
                execution_id: Some(execution_id),
                results: Some(result),
            };
            create_json_response(StatusCode::OK, &response)
        }
        Err(e) => {
            let error_msg = e.to_string();
            log_error(&format!("DAG execution failed: {}", error_msg));
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Execution failed",
                &error_msg,
            )
        }
    }
}

async fn handle_execute_nl(req: Request<Body>, engine: Arc<JormEngine>) -> Response<Body> {
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to read request body: {}", e);
            log_error(&error_msg);
            return create_error_response(
                StatusCode::BAD_REQUEST,
                "Invalid request body",
                &error_msg,
            );
        }
    };

    let request: ExecuteNlRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            let error_msg = format!("Failed to parse JSON: {}", e);
            log_error(&error_msg);
            return create_error_response(StatusCode::BAD_REQUEST, "Invalid JSON", &error_msg);
        }
    };

    log_info(&format!(
        "Executing natural language request: {}",
        request.description
    ));

    match engine
        .execute_from_natural_language(&request.description)
        .await
    {
        Ok(result) => {
            let execution_id = generate_execution_id();
            log_info(&format!(
                "Natural language execution completed - ID: {} - Success: {}",
                execution_id, result.success
            ));

            let response = ExecuteResponse {
                success: result.success,
                message: result.message.clone(),
                execution_id: Some(execution_id),
                results: Some(result),
            };
            create_json_response(StatusCode::OK, &response)
        }
        Err(e) => {
            let error_msg = e.to_string();
            log_error(&format!("Natural language execution failed: {}", error_msg));
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Execution failed",
                &error_msg,
            )
        }
    }
}

async fn handle_health() -> Response<Body> {
    let response = serde_json::json!({
        "status": "healthy",
        "service": "jorm-http-server"
    });
    create_json_response(StatusCode::OK, &response)
}

async fn handle_not_found() -> Response<Body> {
    create_error_response(
        StatusCode::NOT_FOUND,
        "Not Found",
        "The requested endpoint does not exist",
    )
}

fn create_json_response<T: Serialize>(status: StatusCode, data: &T) -> Response<Body> {
    let json = match serde_json::to_string(data) {
        Ok(json) => json,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error": "Serialization failed"}"#))
                .unwrap();
        }
    };

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

fn create_error_response(status: StatusCode, error: &str, message: &str) -> Response<Body> {
    let error_response = ErrorResponse {
        error: error.to_string(),
        message: message.to_string(),
    };
    create_json_response(status, &error_response)
}

fn check_authentication(
    headers: &HeaderMap,
    auth_token: &Option<String>,
) -> Option<Response<Body>> {
    if let Some(expected_token) = auth_token {
        if let Some(auth_header) = headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    // Remove "Bearer " prefix
                    if token == expected_token {
                        return None; // Authentication successful
                    }
                }
            }
        }

        log_warn("Authentication failed - invalid or missing token");
        Some(create_error_response(
            StatusCode::UNAUTHORIZED,
            "Authentication required",
            "Valid Authorization: Bearer <token> header required",
        ))
    } else {
        None // No authentication required
    }
}

fn generate_execution_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("exec_{}", timestamp)
}

fn log_info(message: &str) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    println!("[{}] INFO: {}", timestamp, message);
}

fn log_warn(message: &str) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    println!("[{}] WARN: {}", timestamp, message);
}

fn log_error(message: &str) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    eprintln!("[{}] ERROR: {}", timestamp, message);
}
