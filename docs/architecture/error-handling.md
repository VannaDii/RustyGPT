# RustyGPT Error Handling Architecture

This document describes the comprehensive error handling strategy for the RustyGPT system, focusing on robustness, maintainability, and user experience.

## Overview

RustyGPT implements a comprehensive error handling strategy that leverages Rust's powerful type system and error handling capabilities while providing standardized patterns for error propagation, logging, and client-facing error responses.

## Error Design Principles

The following principles guide our error handling architecture:

1. **Type Safety**: Leverage Rust's type system for compile-time error handling verification.
2. **Clear Ownership**: Errors are created and logged at their point of origin.
3. **Contextual Information**: All errors carry meaningful context about what went wrong.
4. **Correlation**: Error instances maintain tracing identifiers for cross-system correlation.
5. **Internationalization**: Client-facing error messages support multiple languages.
6. **Security**: Error details are sanitized to prevent information leakage.
7. **Standardization**: Consistent error patterns across all components.
8. **Testability**: Error states can be reliably reproduced in tests.

## Error Type Hierarchy

The project uses [`thiserror`](https://docs.rs/thiserror) to define domain-specific error types with clear semantics:

### Entity Errors

```rust
use thiserror::Error;

/// Errors related to entity operations in the system.
#[derive(Debug, Error)]
pub enum EntityError {
    /// Occurs when an entity cannot be found by its identifier.
    #[error("Entity not found with ID: {id}")]
    NotFound {
        /// The ID that was not found
        id: String,
        /// The correlation ID for the request
        correlation_id: Uuid,
    },

    /// Occurs when validation fails for an entity's attributes.
    #[error("Invalid entity data: {reason}")]
    ValidationFailed {
        /// The reason validation failed
        reason: String,
        /// Field that failed validation, if applicable
        field: Option<String>,
        /// The correlation ID for the request
        correlation_id: Uuid,
    },

    /// Occurs when a database operation fails.
    #[error("Database operation failed")]
    DatabaseError(#[from] DatabaseError),

    /// Occurs when an entity operation times out.
    #[error("Entity operation timed out after {duration_ms} ms")]
    Timeout {
        /// Duration in milliseconds before timeout occurred
        duration_ms: u64,
        /// The correlation ID for the request
        correlation_id: Uuid,
    },
}
```

### Authentication Errors

```rust
/// Error type for authentication operations.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Occurs when credentials are invalid.
    #[error("Authentication failed")]
    InvalidCredentials {
        /// The correlation ID for the request
        correlation_id: Uuid,
    },

    /// Occurs when a session has expired.
    #[error("Session expired")]
    SessionExpired {
        /// The correlation ID for the request
        correlation_id: Uuid,
    },

    /// Occurs when a user doesn't have permission.
    #[error("Insufficient permissions for operation: {operation}")]
    InsufficientPermissions {
        /// The operation that was attempted
        operation: String,
        /// The correlation ID for the request
        correlation_id: Uuid,
    },
}
```

## Application Error Context

For application-level error handling, we use [`anyhow`](https://docs.rs/anyhow) to provide rich context for errors:

```rust
use anyhow::{Context, Result};

/// Attempts to create a new entity with the given properties.
///
/// # Arguments
///
/// * `name` - The name of the entity to create
/// * `properties` - A map of property names to values
///
/// # Returns
///
/// A [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html) containing the newly created
/// [`Entity`](crate::entities::Entity) or an error if creation fails.
pub async fn create_entity(
    name: String,
    properties: HashMap<String, Value>,
    correlation_id: Uuid,
) -> Result<Entity> {
    // Validate entity properties
    validate_properties(&properties)
        .with_context(|| format!("Invalid properties for entity '{}'", name))?;

    // Create entity in database
    let entity_id = db::insert_entity(&name, &properties)
        .await
        .with_context(|| format!("Failed to insert entity '{}' into database", name))?;

    // Generate embeddings for the entity
    let embedding = generate_embedding(&name, &properties)
        .await
        .with_context(|| "Failed to generate entity embeddings")?;

    // Store embedding in vector database
    db::store_embedding(entity_id, &embedding)
        .await
        .with_context(|| format!("Failed to store embeddings for entity ID {}", entity_id))?;

    Ok(Entity {
        id: entity_id,
        name,
        properties,
        embedding,
        created_at: Utc::now(),
    })
}
```

## Error Logging Strategy

All errors are logged using structured logging with the following principles:

### Logging Guidelines

1. **No String Interpolation**: Error context is always logged as separate fields, never interpolated into the message string.
2. **Proper Log Levels**:

   - `error`: For system-level errors requiring immediate attention
   - `warn`: For client-related errors (e.g., validation failures)
   - `info`: For expected but notable events
   - `debug`: For detailed debugging information
   - `trace`: For extremely detailed system behavior

3. **Correlation IDs**: Every error includes the `correlation_id` to trace the error through the system.

### Example Error Logging

```rust
/// Example of proper error logging
fn process_request(req: Request) -> Result<Response, ApiError> {
    let correlation_id = req.correlation_id();

    match authenticate_user(&req) {
        Ok(user) => {
            // Process authenticated request
            // ...
        }
        Err(err) => {
            // Log at warn level for client errors
            tracing::warn!(
                correlation_id = %correlation_id.to_string(),
                error_type = "authentication_failed",
                message = "User authentication failed",
                user_id = req.user_id(), // Structured context, not interpolated
                client_ip = req.client_ip(),  // Structured context, not interpolated
                // The %err formatter ensures proper Display implementation is used
                error = %err,
            );

            return Err(ApiError::AuthenticationFailed {
                correlation_id,
                i18n_key: "errors.auth.invalid_credentials",
                params: HashMap::new(),
            });
        }
    }
}
```

## API Error Response Format

Client-facing errors are returned as standardized JSON responses with consistent structure:

```json
{
  "error": {
    "code": "ENTITY_NOT_FOUND",
    "message": "The requested entity could not be found",
    "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
    "i18n": {
      "key": "errors.entity.not_found",
      "params": {
        "id": "entity-123"
      }
    },
    "details": [
      {
        "field": "id",
        "constraint": "must_exist",
        "message": "Entity ID does not exist in the system"
      }
    ]
  }
}
```

## Error Internationalization

Error messages support internationalization via translation keys:

```rust
#[derive(Debug, Serialize)]
pub struct I18nError {
    /// The translation key for this error
    pub key: String,

    /// Parameters to inject into the translated message
    pub params: HashMap<String, String>,
}

/// Converts an error into an internationalized response
fn make_i18n_error(error: &EntityError, lang: &str) -> ApiResponse {
    let (key, params) = match error {
        EntityError::NotFound { id, .. } => (
            "errors.entity.not_found",
            HashMap::from_iter([("id".to_string(), id.to_string())]),
        ),
        EntityError::ValidationFailed { reason, field, .. } => {
            let mut params = HashMap::new();
            params.insert("reason".to_string(), reason.to_string());
            if let Some(field_name) = field {
                params.insert("field".to_string(), field_name.to_string());
            }
            ("errors.entity.validation_failed", params)
        },
        // Other error variants...
    };

    // Lookup translated message using key and language
    let message = i18n::translate(key, &params, lang);

    ApiResponse::error(error.status_code(), message, key, params)
}
```

## Error Propagation Patterns

The project follows these error propagation patterns:

1. **Early Return Pattern**: Use the `?` operator to propagate errors up the call stack.
2. **Error Context Enrichment**: Add context at each level using `anyhow::Context`.
3. **Error Mapping**: Convert between error types using `map_err` or the `From` trait.
4. **Error Type Boundaries**:
   - Domain-specific errors with `thiserror` at library boundaries
   - Generic `anyhow::Error` for application-level code
   - API-specific error responses at service boundaries

## Testing Error Paths

The project places special emphasis on testing error paths:

```rust
#[tokio::test]
async fn test_entity_not_found_error() {
    // Arrange
    let non_existent_id = Uuid::new_v4();
    let correlation_id = Uuid::new_v4();

    // Act
    let result = find_entity_by_id(non_existent_id, correlation_id).await;

    // Assert
    assert!(result.is_err());
    if let Err(EntityError::NotFound { id, correlation_id: error_correlation_id }) = result {
        assert_eq!(id, non_existent_id.to_string());
        assert_eq!(error_correlation_id, correlation_id);
    } else {
        panic!("Expected EntityError::NotFound, got: {:?}", result);
    }
}

#[tokio::test]
async fn test_validation_error_logging() {
    // Arrange
    let invalid_entity_data = HashMap::new(); // Empty data that should fail validation
    let correlation_id = Uuid::new_v4();

    // Act
    let result = create_entity("test_entity".to_string(), invalid_entity_data, correlation_id).await;

    // Assert
    assert!(result.is_err());
    // Verify that appropriate error logs were generated
    // (In practice, this would use a test log capture mechanism)
}
```

## Error Recovery Strategies

### Retry Logic

For transient errors, implement exponential backoff retry logic:

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: usize,
    base_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let mut delay = base_delay;

    for attempt in 0..=max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(err) if attempt == max_retries => return Err(err),
            Err(err) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    max_retries = max_retries,
                    delay_ms = delay.as_millis(),
                    error = ?err,
                    message = "Operation failed, retrying with backoff"
                );
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }

    unreachable!()
}
```

### Circuit Breaker Pattern

For protecting external service calls:

```rust
pub struct CircuitBreaker {
    failure_count: AtomicUsize,
    last_failure_time: AtomicU64,
    state: AtomicU8, // 0 = Closed, 1 = Open, 2 = Half-Open
    failure_threshold: usize,
    timeout_duration: Duration,
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self.state() {
            CircuitState::Open => {
                if self.should_attempt_reset() {
                    self.transition_to_half_open();
                } else {
                    return Err(/* Circuit breaker open error */);
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited calls through
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        match operation() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(err) => {
                self.on_failure();
                Err(err)
            }
        }
    }
}
```

## Related Documents

For detailed information about specific aspects of the architecture, see:

- [Architecture Overview](./overview.md) - High-level system overview and key concepts
- [Requirements](./requirements.md) - Detailed functional and non-functional requirements
- [Reasoning DAG](./reasoning-dag.md) - In-depth DAG architecture and node orchestration
- [Database Schema](./database-schema.md) - Core database design and stored procedures
- [Database Optimization](./database-optimization.md) - Advanced optimization strategies
