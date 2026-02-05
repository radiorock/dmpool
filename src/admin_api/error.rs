// Admin API Error Types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Admin API error type
#[derive(Debug)]
pub enum AdminError {
    Database(String),
    NotFound(String),
    InvalidInput(String),
    Unauthorized(String),
    Forbidden(String),
    Internal(String),
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminError::Database(msg) => write!(f, "Database error: {}", msg),
            AdminError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AdminError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AdminError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AdminError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AdminError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AdminError {}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            AdminError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error", "DATABASE_ERROR")
            }
            AdminError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.as_str(), "NOT_FOUND")
            }
            AdminError::InvalidInput(msg) => {
                (StatusCode::BAD_REQUEST, msg.as_str(), "INVALID_INPUT")
            }
            AdminError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, msg.as_str(), "UNAUTHORIZED")
            }
            AdminError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, msg.as_str(), "FORBIDDEN")
            }
            AdminError::Internal(msg) => {
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

impl From<anyhow::Error> for AdminError {
    fn from(err: anyhow::Error) -> Self {
        AdminError::Internal(err.to_string())
    }
}

impl From<tokio_postgres::Error> for AdminError {
    fn from(err: tokio_postgres::Error) -> Self {
        AdminError::Database(err.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for AdminError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        AdminError::Database(err.to_string())
    }
}
