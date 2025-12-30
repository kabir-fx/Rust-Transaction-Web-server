//! Webhook service for managing endpoints and sending events.
//!
//! This module handles webhook endpoint registration, event delivery,
//! and HMAC signature generation for secure webhook verification.

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::transaction::Transaction;
use crate::models::webhook::{
    NewWebhookEvent, WebhookEndpoint, WebhookEndpointRequest, WebhookEndpointResponse,
    WebhookPayload,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Create a new webhook endpoint.
///
/// # Process
///
/// 1. Validate URL format
/// 2. Generate cryptographically secure secret (32 bytes)
/// 3. Store endpoint in database
/// 4. Return endpoint with secret (only shown once)
///
/// # Security
///
/// - HTTPS is required for production endpoints
/// - HTTP localhost is allowed for testing
/// - Secret is 64 hex characters (32 bytes of randomness)
pub async fn create_webhook_endpoint(
    pool: &DbPool,
    api_key_id: Uuid,
    request: WebhookEndpointRequest,
) -> Result<WebhookEndpointResponse, AppError> {
    // Validate URL
    validate_webhook_url(&request.url)?;

    // Generate secure random secret (32 bytes = 64 hex chars)
    let secret = generate_secret();

    // Insert into database
    let endpoint = sqlx::query_as::<_, WebhookEndpoint>(
        r#"
        INSERT INTO webhook_endpoints (api_key_id, url, secret)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(api_key_id)
    .bind(&request.url)
    .bind(&secret)
    .fetch_one(pool)
    .await?;

    // Return response with secret included (only time it's shown)
    Ok(WebhookEndpointResponse::from(endpoint.clone()).with_secret(secret))
}

/// List all webhook endpoints for an API key.
///
/// # Security
///
/// - Filters by api_key_id (authenticated business only)
/// - Does NOT return secrets
/// - Only returns active endpoints by default
pub async fn list_webhook_endpoints(
    pool: &DbPool,
    api_key_id: Uuid,
) -> Result<Vec<WebhookEndpointResponse>, AppError> {
    let endpoints = sqlx::query_as::<_, WebhookEndpoint>(
        "SELECT * FROM webhook_endpoints WHERE api_key_id = $1 AND is_active = true ORDER BY created_at DESC",
    )
    .bind(api_key_id)
    .fetch_all(pool)
    .await?;

    // Convert to response format (secrets excluded)
    Ok(endpoints.into_iter().map(|e| e.into()).collect())
}

/// Delete a webhook endpoint (soft delete).
///
/// # Process
///
/// 1. Verify endpoint exists and belongs to authenticated business
/// 2. Set is_active = false (preserve event history)
///
/// # Security
///
/// - Verifies ownership by api_key_id
pub async fn delete_webhook_endpoint(
    pool: &DbPool,
    api_key_id: Uuid,
    endpoint_id: Uuid,
) -> Result<(), AppError> {
    let result = sqlx::query(
        "UPDATE webhook_endpoints SET is_active = false WHERE id = $1 AND api_key_id = $2",
    )
    .bind(endpoint_id)
    .bind(api_key_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::WebhookNotFound);
    }

    Ok(())
}

/// Send webhook notification for a transaction to all registered endpoints.
///
/// # Process
///
/// 1. Fetch all active webhook endpoints for the transaction's API key
/// 2. For each endpoint, send webhook with signed payload
/// 3. Log all delivery attempts
///
/// # Error Handling
///
/// - Individual webhook failures are logged but don't fail the overall operation
/// - Transaction success is independent of webhook delivery
pub async fn notify_transaction_webhooks(
    pool: &DbPool,
    transaction: &Transaction,
    api_key_id: Uuid,
) -> Result<(), AppError> {
    // Fetch active webhook endpoints for this API key
    let endpoints = sqlx::query_as::<_, WebhookEndpoint>(
        "SELECT * FROM webhook_endpoints WHERE api_key_id = $1 AND is_active = true",
    )
    .bind(api_key_id)
    .fetch_all(pool)
    .await?;

    // Send webhook to each endpoint
    for endpoint in endpoints {
        if let Err(e) = send_webhook(pool, &endpoint, transaction).await {
            tracing::error!("Failed to send webhook to {}: {:?}", endpoint.url, e);
            // Continue to next endpoint even if one fails
        }
    }

    Ok(())
}

/// Send a single webhook with HMAC signature.
///
/// # Process
///
/// 1. Build webhook payload
/// 2. Generate HMAC-SHA256 signature
/// 3. Send HTTP POST with signature header
/// 4. Record event in database
///
/// # Headers Sent
///
/// - `Content-Type: application/json`
/// - `X-Webhook-Signature: sha256=<hex>`
/// - `X-Webhook-Event-Id: <uuid>`
///
/// # Timeout
///
/// 5 seconds per webhook (prevents hanging on slow endpoints)
async fn send_webhook(
    pool: &DbPool,
    endpoint: &WebhookEndpoint,
    transaction: &Transaction,
) -> Result<(), AppError> {
    let event_id = Uuid::new_v4();

    // Build payload
    let payload = WebhookPayload::new(event_id, transaction.clone());
    let payload_json = serde_json::to_string(&payload)
        .map_err(|e| AppError::InvalidRequest(format!("Failed to serialize payload: {}", e)))?;

    // Generate HMAC signature
    let signature = generate_signature(&endpoint.secret, &payload_json);

    // Send HTTP POST
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::InvalidRequest(format!("HTTP client error: {}", e)))?;

    let response = client
        .post(&endpoint.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", &signature)
        .header("X-Webhook-Event-Id", event_id.to_string())
        .body(payload_json.clone())
        .send()
        .await;

    // Record event in database
    let (status, body) = match response {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let body = resp.text().await.ok();
            (Some(status), body)
        }
        Err(e) => {
            let error_msg = format!("Request failed: {}", e);
            tracing::error!("{}", error_msg);
            (None, Some(error_msg))
        }
    };

    // Create webhook event record
    let payload_value = serde_json::from_str::<serde_json::Value>(&payload_json)
        .map_err(|e| AppError::InvalidRequest(format!("Failed to parse payload: {}", e)))?;

    let event = NewWebhookEvent::new(
        event_id,
        endpoint.id,
        transaction.id,
        payload_value,
        status,
        body,
    );

    // Store event record
    sqlx::query(
        r#"
        INSERT INTO webhook_events (
            id,
            webhook_endpoint_id,
            transaction_id,
            payload,
            response_status,
            response_body
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(event.id)
    .bind(event.webhook_endpoint_id)
    .bind(event.transaction_id)
    .bind(event.payload)
    .bind(event.response_status)
    .bind(event.response_body)
    .execute(pool)
    .await?;

    Ok(())
}

/// Generate HMAC-SHA256 signature for webhook payload.
///
/// # Format
///
/// `sha256=<hex_encoded_hmac>`
///
/// # Verification
///
/// Clients should:
/// 1. Extract signature from `X-Webhook-Signature` header
/// 2. Compute HMAC-SHA256(secret, request_body)
/// 3. Compare using constant-time comparison
fn generate_signature(secret: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key length is valid");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

/// Generate cryptographically secure random secret.
///
/// # Output
///
/// 64 hex characters (32 random bytes)
fn generate_secret() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

/// Validate webhook URL format.
///
/// # Rules
///
/// - Must be valid URL
/// - Must be HTTPS (HTTP localhost allowed for development)
/// - Maximum 2048 characters
fn validate_webhook_url(url: &str) -> Result<(), AppError> {
    if url.len() > 2048 {
        return Err(AppError::InvalidWebhookUrl(
            "URL exceeds 2048 characters".to_string(),
        ));
    }

    // Parse URL
    let parsed = url::Url::parse(url)
        .map_err(|_| AppError::InvalidWebhookUrl("Invalid URL format".to_string()))?;

    // Check scheme
    match parsed.scheme() {
        "https" => Ok(()),
        "http" => {
            // Allow HTTP for localhost/127.0.0.1 (testing)
            if parsed.host_str() == Some("localhost")
                || parsed.host_str() == Some("127.0.0.1")
                || parsed.host_str() == Some("0.0.0.0")
            {
                Ok(())
            } else {
                Err(AppError::InvalidWebhookUrl(
                    "HTTP is only allowed for localhost. Use HTTPS for production.".to_string(),
                ))
            }
        }
        _ => Err(AppError::InvalidWebhookUrl(
            "URL must use HTTP or HTTPS".to_string(),
        )),
    }
}
