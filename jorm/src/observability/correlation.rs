/// Correlation ID management for distributed tracing
/// 
/// Correlation IDs allow tracking a request or operation across
/// multiple services and components. Each operation gets a unique
/// ID that is propagated through all related operations.

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

thread_local! {
    /// Thread-local storage for correlation ID
    static CORRELATION_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// Gets the current correlation ID for this thread
/// 
/// # Returns
/// * `Option<String>` - The correlation ID if set, None otherwise
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::{set_correlation_id, get_correlation_id};
/// 
/// set_correlation_id("req-123".to_string());
/// assert_eq!(get_correlation_id(), Some("req-123".to_string()));
/// ```
pub fn get_correlation_id() -> Option<String> {
    CORRELATION_ID.with(|id| id.borrow().clone())
}

/// Sets the correlation ID for this thread
/// 
/// # Arguments
/// * `id` - The correlation ID to set
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::set_correlation_id;
/// 
/// set_correlation_id("req-123".to_string());
/// ```
pub fn set_correlation_id(id: String) {
    CORRELATION_ID.with(|correlation_id| {
        *correlation_id.borrow_mut() = Some(id);
    });
}

/// Clears the correlation ID for this thread
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::{set_correlation_id, clear_correlation_id, get_correlation_id};
/// 
/// set_correlation_id("req-123".to_string());
/// clear_correlation_id();
/// assert_eq!(get_correlation_id(), None);
/// ```
pub fn clear_correlation_id() {
    CORRELATION_ID.with(|id| {
        *id.borrow_mut() = None;
    });
}

/// Generates a new correlation ID
/// 
/// # Returns
/// * `String` - A new UUID-based correlation ID
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::generate_correlation_id;
/// 
/// let id = generate_correlation_id();
/// assert!(!id.is_empty());
/// ```
pub fn generate_correlation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Executes a function with a correlation ID
/// 
/// Sets the correlation ID before executing the function and
/// clears it afterwards, even if the function panics.
/// 
/// # Arguments
/// * `id` - The correlation ID to use
/// * `f` - The function to execute
/// 
/// # Returns
/// * `T` - The result of the function
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::{with_correlation_id, get_correlation_id};
/// 
/// let result = with_correlation_id("req-123".to_string(), || {
///     assert_eq!(get_correlation_id(), Some("req-123".to_string()));
///     42
/// });
/// assert_eq!(result, 42);
/// assert_eq!(get_correlation_id(), None);
/// ```
pub fn with_correlation_id<F, T>(id: String, f: F) -> T
where
    F: FnOnce() -> T,
{
    set_correlation_id(id);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    clear_correlation_id();
    
    match result {
        Ok(value) => value,
        Err(e) => std::panic::resume_unwind(e),
    }
}

/// Executes an async function with a correlation ID
/// 
/// Sets the correlation ID before executing the async function and
/// clears it afterwards.
/// 
/// # Arguments
/// * `id` - The correlation ID to use
/// * `f` - The async function to execute
/// 
/// # Returns
/// * `T` - The result of the async function
/// 
/// # Examples
/// 
/// ```
/// use jorm::observability::correlation::{with_correlation_id_async, get_correlation_id};
/// 
/// # tokio_test::block_on(async {
/// let result = with_correlation_id_async("req-123".to_string(), async {
///     assert_eq!(get_correlation_id(), Some("req-123".to_string()));
///     42
/// }).await;
/// assert_eq!(result, 42);
/// # });
/// ```
pub async fn with_correlation_id_async<F, T>(id: String, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    set_correlation_id(id);
    let result = f.await;
    clear_correlation_id();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test setting and getting correlation ID
    #[test]
    fn test_set_get_correlation_id() {
        let id = "test-123".to_string();
        set_correlation_id(id.clone());
        assert_eq!(get_correlation_id(), Some(id));
    }

    /// Test clearing correlation ID
    #[test]
    fn test_clear_correlation_id() {
        set_correlation_id("test-123".to_string());
        clear_correlation_id();
        assert_eq!(get_correlation_id(), None);
    }

    /// Test generating correlation ID
    #[test]
    fn test_generate_correlation_id() {
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();
        
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2); // Should be unique
    }

    /// Test with_correlation_id function
    #[test]
    fn test_with_correlation_id() {
        let id = "test-456".to_string();
        
        let result = with_correlation_id(id.clone(), || {
            assert_eq!(get_correlation_id(), Some(id));
            42
        });
        
        assert_eq!(result, 42);
        assert_eq!(get_correlation_id(), None); // Should be cleared
    }

    /// Test with_correlation_id clears on panic
    #[test]
    fn test_with_correlation_id_panic() {
        let id = "test-789".to_string();
        
        let result = std::panic::catch_unwind(|| {
            with_correlation_id(id, || {
                panic!("test panic");
            })
        });
        
        assert!(result.is_err());
        assert_eq!(get_correlation_id(), None); // Should be cleared even on panic
    }

    /// Test async correlation ID
    #[tokio::test]
    async fn test_with_correlation_id_async() {
        let id = "test-async".to_string();
        
        let result = with_correlation_id_async(id.clone(), async {
            assert_eq!(get_correlation_id(), Some(id));
            42
        }).await;
        
        assert_eq!(result, 42);
        assert_eq!(get_correlation_id(), None); // Should be cleared
    }
}
