// Admin API Middleware
//
// Provides authentication middleware for protecting admin endpoints

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use crate::admin_api::error::AdminError;

/// Authentication middleware for admin endpoints
pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AdminError> {
    // For now, we'll implement basic JWT authentication
    // In production, this should validate the JWT token

    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AdminError::Unauthorized("Missing Authorization header".to_string()))?;

    // Validate Bearer token format
    if !auth_header.starts_with("Bearer ") {
        return Err(AdminError::Unauthorized("Invalid Authorization format".to_string()));
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // TODO: Validate JWT token
    // For now, we'll do basic validation
    if token.is_empty() {
        return Err(AdminError::Unauthorized("Empty token".to_string()));
    }

    // Token is valid, proceed with request
    Ok(next.run(req).await)
}
