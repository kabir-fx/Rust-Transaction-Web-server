//! Health check endpoint for service monitoring.

use crate::{db::DbPool, error::AppError};
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Health check response.
///
/// Returns service status and database connectivity.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Overall service status
    pub status: String,

    /// Database connection status
    pub database: String,

    /// Current server timestamp
    pub timestamp: DateTime<Utc>,
}

/// Health check handler.
///
/// # Checks
///
/// - Database connectivity (executes simple query)
///
/// # Response (200 OK)
///
/// ```json
/// {
///   "status": "healthy",
///   "database": "connected",
///   "timestamp": "2025-12-21T19:00:00Z"
/// }
/// ```
///
/// # Response (500 Internal Server Error)
///
/// If database is unreachable, returns standard error response.
pub async fn health_check(State(pool): State<DbPool>) -> Result<Json<HealthResponse>, AppError> {
    // Verify database connectivity with simple query
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        database: "connected".to_string(),
        timestamp: Utc::now(),
    }))
}
