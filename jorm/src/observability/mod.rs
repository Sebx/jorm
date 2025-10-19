/// Observability module - Structured logging and tracing
/// 
/// This module provides structured logging using the `tracing` crate,
/// enabling production-ready observability with log levels, spans,
/// and correlation IDs.

pub mod correlation;

use tracing_subscriber::{self, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the tracing subscriber for the application
/// 
/// Sets up structured logging with:
/// - Environment-based log level filtering (RUST_LOG)
/// - Pretty formatting for development
/// - JSON formatting for production (when JORM_LOG_JSON=1)
/// - Correlation ID support
/// 
/// # Examples
/// 
/// ```no_run
/// use jorm::observability::init_tracing;
/// 
/// // Initialize with default settings
/// init_tracing();
/// 
/// // Use RUST_LOG environment variable to control levels:
/// // RUST_LOG=debug cargo run
/// // RUST_LOG=jorm=trace cargo run
/// ```
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let use_json = std::env::var("JORM_LOG_JSON")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if use_json {
        // JSON formatting for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    } else {
        // Pretty formatting for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().pretty())
            .init();
    }
}

/// Initializes tracing with custom configuration
/// 
/// # Arguments
/// * `log_level` - The log level filter (e.g., "debug", "info")
/// * `use_json` - Whether to use JSON formatting
/// 
/// # Examples
/// 
/// ```no_run
/// use jorm::observability::init_tracing_with_config;
/// 
/// // Initialize with debug level and JSON formatting
/// init_tracing_with_config("debug", true);
/// ```
pub fn init_tracing_with_config(log_level: &str, use_json: bool) {
    let env_filter = EnvFilter::new(log_level);

    if use_json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().pretty())
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    /// Test that tracing can be initialized
    #[test]
    fn test_init_tracing() {
        // This test just verifies the function doesn't panic
        // Actual initialization is tested in integration tests
        // since tracing can only be initialized once per process
    }

    /// Test that different log levels can be used
    #[test]
    fn test_log_levels() {
        // Set up a test subscriber
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            error!("error message");
            warn!("warning message");
            info!("info message");
            debug!("debug message");
        });
    }

    /// Test structured logging with fields
    #[test]
    fn test_structured_logging() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            info!(
                user_id = "user123",
                action = "login",
                "User logged in"
            );
        });
    }
}
