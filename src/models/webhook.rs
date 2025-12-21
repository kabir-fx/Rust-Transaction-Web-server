//! Webhook models for endpoint registration and event delivery.
//!
//! This module defines the data structures for managing webhook endpoints
//! and tracking webhook event deliveries.
//!
//! # Webhook Flow
//!
//! 1. Business registers a webhook endpoint via `POST /api/v1/webhooks`
//! 2. System generates a secret for HMAC signature verification
//! 3. When transactions occur, system sends webhook with signed payload
//! 4. Business verifies signature using the secret
//!
//! # Security
//!
//! - Secrets are only shown once during registration
//! - Payloads are signed using HMAC-SHA256
//! - HTTPS is required for production endpoints

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::transaction::Transaction;

/// Webhook endpoint registered by a business.
///
/// # Database Table
///
/// Maps to the `webhook_endpoints` table.
///
/// # Secret Storage
///
/// The `secret` is stored in plaintext (required for HMAC generation)
/// but never returned in list/get operations for security.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub api_key_id: Uuid,
    pub url: String,
    pub secret: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Request to register a new webhook endpoint.
///
/// # Example
///
/// ```json
/// {
///   "url": "https://example.com/webhook"
/// }
/// ```
///
/// # Validation
///
/// - URL must be valid HTTPS (HTTP allowed for localhost in development)
/// - URL must not exceed 2048 characters
#[derive(Debug, Deserialize)]
pub struct WebhookEndpointRequest {
    pub url: String,
}

/// Response when registering or retrieving a webhook endpoint.
///
/// # Security Note
///
/// The `secret` field is ONLY included when creating a new endpoint.
/// It is never returned in list/get operations.
///
/// # Example (Create Response)
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
#[derive(Debug, Serialize)]
pub struct WebhookEndpointResponse {
    pub id: Uuid,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<WebhookEndpoint> for WebhookEndpointResponse {
    fn from(endpoint: WebhookEndpoint) -> Self {
        Self {
            id: endpoint.id,
            url: endpoint.url,
            secret: None, // Never include secret by default
            is_active: endpoint.is_active,
            created_at: endpoint.created_at,
        }
    }
}

impl WebhookEndpointResponse {
    /// Create response with secret included (only for registration).
    pub fn with_secret(mut self, secret: String) -> Self {
        self.secret = Some(secret);
        self
    }
}

/// Webhook event delivery record.
///
/// # Database Table
///
/// Maps to the `webhook_events` table.
///
/// # Purpose
///
/// Tracks every webhook delivery attempt, including the payload sent,
/// HTTP response status, and any error messages.
#[derive(Debug, Clone, FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub webhook_endpoint_id: Uuid,
    pub transaction_id: Uuid,
    pub payload: serde_json::Value,
    pub sent_at: DateTime<Utc>,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
}

/// Webhook payload sent to the registered endpoint.
///
/// # Format
///
/// This is the JSON body sent in the HTTP POST request.
///
/// # Example
///
/// ```json
/// {
///   "event_type": "transaction.completed",
///   "event_id": "550e8400-e29b-41d4-a716-446655440000",
///   "created_at": "2025-01-15T10:30:00Z",
///   "data": {
///     "transaction": {
///       "id": "...",
///       "type": "transfer",
///       "amount_cents": 100000,
///       "from_account_id": "...",
///       "to_account_id": "...",
///       "status": "completed"
///     }
///   }
/// }
/// ```
///
/// # Signature Verification
///
/// The webhook includes an `X-Webhook-Signature` header with format:
/// `sha256=<hex_encoded_hmac>`
///
/// Clients should verify this by computing HMAC-SHA256(secret, json_body)
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Type of event (always "transaction.completed" in this phase)
    pub event_type: String,

    /// Unique identifier for this webhook event
    pub event_id: Uuid,

    /// When the event was created
    pub created_at: DateTime<Utc>,

    /// Event data containing transaction details
    pub data: WebhookData,
}

/// Data portion of the webhook payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookData {
    /// Transaction that triggered the webhook
    pub transaction: TransactionWebhookData,
}

/// Transaction data included in webhook payload.
///
/// This is a subset of the full Transaction model, containing
/// only the fields relevant for webhook consumers.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionWebhookData {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount_cents: i64,
    pub currency: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl From<Transaction> for TransactionWebhookData {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            transaction_type: t.transaction_type,
            from_account_id: t.from_account_id,
            to_account_id: t.to_account_id,
            amount_cents: t.amount_cents,
            currency: t.currency,
            description: t.description,
            status: t.status,
            created_at: t.created_at,
        }
    }
}

impl WebhookPayload {
    /// Create a new webhook payload for a transaction event.
    pub fn new(event_id: Uuid, transaction: Transaction) -> Self {
        Self {
            event_type: "transaction.completed".to_string(),
            event_id,
            created_at: Utc::now(),
            data: WebhookData {
                transaction: transaction.into(),
            },
        }
    }
}
