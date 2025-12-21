//! HTTP handlers for webhook endpoint management.
//!
//! This module provides API endpoints for businesses to register,
//! list, and delete webhook endpoints that receive transaction notifications.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::webhook::{WebhookEndpointRequest, WebhookEndpointResponse};
use crate::services::webhook_service;

/// Register a new webhook endpoint.
///
/// # Request Body
///
/// ```json
/// {
///   "url": "https://example.com/webhook"
/// }
/// ```
///
/// # Response
///
/// Returns 201 Created with the webhook endpoint details.
/// The `secret` is only returned once during creation.
///
/// ```json
/// {
///   "id": "550e8400-e29b-41d4-a716-446655440000",
///   "url": "https://example.com/webhook",
///   "secret": "a1b2c3d4e5f6...",
///   "is_active": true,
///   "created_at": "2025-01-15T10:30:00Z"
/// }
/// ```
///
/// # Security
///
/// - Requires valid API key authentication
/// - HTTPS URLs required (HTTP localhost allowed for development)
/// - Secret is 64-character hex string for HMAC-SHA256
pub async fn create_webhook(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<WebhookEndpointRequest>,
) -> Result<impl IntoResponse, AppError> {
    let endpoint =
        webhook_service::create_webhook_endpoint(&pool, auth.api_key_id, request).await?;

    Ok((StatusCode::CREATED, Json(endpoint)))
}

/// List all active webhook endpoints.
///
/// # Response
///
/// Returns array of webhook endpoints (secrets NOT included).
///
/// ```json
/// [
///   {
///     "id": "550e8400-e29b-41d4-a716-446655440000",
///     "url": "https://example.com/webhook",
///     "is_active": true,
///     "created_at": "2025-01-15T10:30:00Z"
///   }
/// ]
/// ```
///
/// # Security
///
/// - Requires valid API key authentication
/// - Returns only webhooks belonging to authenticated business
/// - Secrets are never returned in list operations
pub async fn list_webhooks(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<WebhookEndpointResponse>>, AppError> {
    let webhooks = webhook_service::list_webhook_endpoints(&pool, auth.api_key_id).await?;

    Ok(Json(webhooks))
}

/// Delete a webhook endpoint (soft delete).
///
/// # Response
///
/// Returns 204 No Content on success.
///
/// # Process
///
/// Sets `is_active = false` to preserve event history.
/// The endpoint will no longer receive webhooks.
///
/// # Security
///
/// - Requires valid API key authentication
/// - Verifies webhook belongs to authenticated business
/// - Returns 404 if webhook not found or doesn't belong to business
pub async fn delete_webhook(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Path(webhook_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    webhook_service::delete_webhook_endpoint(&pool, auth.api_key_id, webhook_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
