//! Error types and HTTP error response handling.
//!
//! This module defines all application errors and how they are converted
//! into HTTP responses with appropriate status codes and JSON bodies.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Application-wide error type.
///
/// This enum represents all possible errors that can occur in the application.
/// Each variant maps to a specific HTTP status code and error message.
///
/// # Error Categories
///
/// - **Database Errors**: Any sqlx::Error from database operations
/// - **Authentication Errors**: Invalid or missing API keys
/// - **Resource Errors**: Requested resources not found
/// - **Business Logic Errors**: Operations that violate business rules
/// - **Validation Errors**: Invalid request data
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Database operation failed (e.g., connection error, query error).
    ///
    /// This wraps any sqlx::Error using the `#[from]` attribute, which
    /// automatically implements `From<sqlx::Error> for AppError`.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// API key is missing, invalid, or inactive.
    ///
    /// Returns HTTP 401 Unauthorized.
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Requested account does not exist or doesn't belong to authenticated business.
    ///
    /// Returns HTTP 404 Not Found.
    #[error("Account not found")]
    AccountNotFound,

    /// Account has insufficient balance for the requested operation.
    ///
    /// Returns HTTP 422 Unprocessable Entity.
    #[error("Insufficient balance")]
    InsufficientBalance,

    /// Request body or parameters are invalid.
    ///
    /// Returns HTTP 400 Bad Request.
    /// The String contains details about what was invalid.
    #[error("Invalid request")]
    InvalidRequest(String),
}

/// Convert AppError into an HTTP response.
///
/// This implementation allows Axum handlers to return `Result<T, AppError>`
/// and have errors automatically converted to proper HTTP responses.
///
/// # Response Format
///
/// All errors return JSON in this format:
/// ```json
/// {
///   "error": {
///     "code": "error_type",
///     "message": "Human-readable error message"
///   }
/// }
/// ```
///
/// # Status Code Mapping
///
/// - `InvalidApiKey` → 401 Unauthorized
/// - `AccountNotFound` → 404 Not Found
/// - `InsufficientBalance` → 422 Unprocessable Entity
/// - `InvalidRequest` → 400 Bad Request
/// - `Database` → 500 Internal Server Error (hides details from client)
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Map each error variant to (HTTP status, error code, message)
        let (status, code, message) = match self {
            AppError::InvalidApiKey => (
                StatusCode::UNAUTHORIZED,
                "invalid_api_key",
                self.to_string(),
            ),
            AppError::AccountNotFound => {
                (StatusCode::NOT_FOUND, "account_not_found", self.to_string())
            }
            AppError::InsufficientBalance => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "insufficient_balance",
                self.to_string(),
            ),
            AppError::InvalidRequest(ref msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request", msg.clone())
            }
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal error occurred".to_string(),
            ),
        };

        // Build JSON response body
        let body = Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        }));

        // Return the response with status code and JSON body
        (status, body).into_response()
    }
}
