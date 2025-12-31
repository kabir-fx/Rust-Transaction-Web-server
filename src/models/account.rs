//! Account data models and API request/response types.
//!
//! This module defines:
//! - `Account`: Database entity representing an account
//! - `CreateAccountRequest`: Request body for creating accounts
//! - `AccountResponse`: Response body returned to clients

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents an account record from the database.
///
/// # Database Table
///
/// Maps to the `accounts` table. Each account:
/// - Belongs to one business (via `api_key_id`)
/// - Has a balance stored in cents (to avoid floating-point errors)
///
/// # Balance Storage
///
/// Balances are stored as `i64` cents to avoid floating-point precision issues.
///
/// For example:
/// - $10.50 is stored as 1050 cents
/// - $100.00 is stored as 10000 cents
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Account {
    /// Unique identifier for this account
    pub id: Uuid,

    /// Foreign key to the API key (business) that owns this account
    ///
    /// This ensures accounts are isolated per business. When querying accounts, we will always filter by `api_key_id` to prevent one business from accessing another's accounts.
    pub api_key_id: Uuid,

    /// Human-readable name for this account
    pub account_name: String,

    /// Current balance in cents (not dollars)
    ///
    /// Must be >= 0 (enforced by database CHECK constraint).
    /// Using i64 allows balances up to ~92 quadrillion dollars.
    pub balance_cents: i64,

    /// Currency code (ISO 4217, 3 letters)
    ///
    /// Examples: "USD", "EUR", "GBP"
    /// Currently defaults to "USD" but stored for future multi-currency support.
    pub currency: String,

    /// Timestamp when account was created
    pub created_at: DateTime<Utc>,

    /// Timestamp of last balance update
    ///
    /// Updated automatically by database triggers or application code
    /// when transactions modify the balance.
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a new account.
///
/// # JSON Example
///
/// ```json
/// {
///   "account_name": "My Savings Account",
///   "currency": "USD",
///   "initial_balance_cents": 10000
/// }
/// ```
///
/// # Validation
///
/// - `account_name`: Required, any non-empty string
/// - `currency`: Optional, defaults to "USD"
/// - `initial_balance_cents`: Optional, defaults to 0
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    /// Name for the new account
    pub account_name: String,

    /// Currency code (defaults to "USD" if not provided)
    #[serde(default = "default_currency")]
    pub currency: String,

    /// Initial balance in cents (defaults to 0 if not provided)
    #[serde(default)]
    pub initial_balance_cents: i64,
}

/// Default currency value when not specified in request.
fn default_currency() -> String {
    "USD".to_string()
}

/// Response body for account endpoints.
///
/// This struct is returned to API clients.
///
/// # JSON Example
///
/// ```json
/// {
///   "id": "550e8400-e29b-41d4-a716-446655440000",
///   "account_name": "My Account",
///   "balance_cents": 100000,
///   "currency": "USD",
///   "created_at": "2025-12-20T10:00:00Z",
///   "updated_at": "2025-12-20T10:00:00Z"
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct AccountResponse {
    /// Account unique identifier
    pub id: Uuid,

    /// Account name
    pub account_name: String,

    /// Current balance in cents
    pub balance_cents: i64,

    /// Currency code
    pub currency: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Convert database Account to API AccountResponse.
///
/// This transformation - Removes the internal `api_key_id` field
impl From<Account> for AccountResponse {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            account_name: account.account_name,
            balance_cents: account.balance_cents,
            currency: account.currency,
            created_at: account.created_at,
            updated_at: account.updated_at,
        }
    }
}
