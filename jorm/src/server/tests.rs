use crate::server::http::HttpServer;
use crate::JormEngine;
use std::sync::Arc;

#[tokio::test]
async fn test_http_server_creation() {
    let engine = Arc::new(JormEngine::new().await.unwrap());
    let _server = HttpServer::new(engine, 8080);

    // Test that server can be created without authentication
    // (We can't access private fields, but creation success is sufficient)
}

#[tokio::test]
async fn test_http_server_with_auth() {
    let engine = Arc::new(JormEngine::new().await.unwrap());
    let _server = HttpServer::with_auth_token(engine, 8080, "test-token".to_string());

    // Test that server can be created with authentication
    // (We can't access private fields, but creation success is sufficient)
}

// Note: Additional unit tests for internal functions would require making them public
// For now, we focus on testing the public API of HttpServer
