//! API Key model for authentication.
//!
//! API keys are used to authenticate businesses making requests to the API. They are stored in the database as SHA-256 hashes for security.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents an API key record from the database.
///
/// # Database Table
///
/// Maps to the `api_keys` table with columns:
/// - `id`: Unique identifier (UUID)
/// - `key_hash`: SHA-256 hash of the actual API key
/// - `business_name`: Name of the business this key belongs to
/// - `created_at`: When the key was created
/// - `is_active`: Whether the key is currently valid
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    /// Unique identifier for this API key
    pub id: Uuid,

    /// SHA-256 hash of the actual API key (64 hex characters)
    ///
    /// When a request comes in with "Bearer abc123", we:
    /// 1. Hash "abc123" with SHA-256
    /// 2. Look up this hash in the database
    /// 3. If found and active, authenticate the request
    pub key_hash: String,

    /// Human-readable name of the business using this API key
    pub business_name: String,

    /// Timestamp when this API key was created
    pub created_at: DateTime<Utc>,

    /// Whether this API key is currently active
    ///
    /// Inactive keys are rejected during authentication. This provides a way to revoke access without deleting the record.
    pub is_active: bool,
}
