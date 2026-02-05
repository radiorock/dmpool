// Observer API Error Types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Observer API error type
#[derive(Debug)]
pub enum ObserverError {
    /// Database error
    Database(String),
    /// Not found
    NotFound(String),
    /// Invalid input
    InvalidInput(String),
    /// Internal server error
    Internal(String),
}

impl std::fmt::Display for ObserverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserverError::Database(msg) => write!(f, "Database error: {}", msg),
            ObserverError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ObserverError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ObserverError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ObserverError {}

impl IntoResponse for ObserverError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            ObserverError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error", "DATABASE_ERROR")
            }
            ObserverError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.as_str(), "NOT_FOUND")
            }
            ObserverError::InvalidInput(msg) => {
                (StatusCode::BAD_REQUEST, msg.as_str(), "INVALID_INPUT")
            }
            ObserverError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error", "INTERNAL_ERROR")
            }
        };

        let body = json!({
            "error": error_code,
            "message": error_message,
        });

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ObserverError {
    fn from(err: anyhow::Error) -> Self {
        ObserverError::Internal(err.to_string())
    }
}

impl From<tokio_postgres::Error> for ObserverError {
    fn from(err: tokio_postgres::Error) -> Self {
        ObserverError::Database(err.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for ObserverError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        ObserverError::Database(err.to_string())
    }
}
