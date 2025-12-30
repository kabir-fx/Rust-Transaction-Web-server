//! Transaction data models and API request/response types.
//!
//! This module defines:
//! - `Transaction`: Database entity representing a transaction
//! - Request types for credit, debit, and transfer operations
//! - `TransactionResponse`: Response body returned to clients

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a transaction record from the database.
///
/// # Database Table
///
/// Maps to the `transactions` table. Each transaction:
/// - Has a unique ID and optional idempotency key
/// - References one or two accounts (depending on type)
/// - Stores amount in cents (never floats!)
/// - Tracks status (completed, failed, pending)
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Transaction {
    /// Unique identifier for this transaction
    pub id: Uuid,

    /// Optional idempotency key for preventing duplicates
    ///
    /// If client sends same idempotency_key twice, the second request
    /// returns the original transaction instead of creating a duplicate.
    pub idempotency_key: Option<String>,

    /// Type of transaction (credit, debit, or transfer)
    pub transaction_type: String,

    /// Source account (for debit and transfer)
    ///
    /// NULL for credit transactions (money appearing from nowhere)
    pub from_account_id: Option<Uuid>,

    /// Destination account (for credit and transfer)
    ///
    /// NULL for debit transactions (money disappearing to nowhere)
    pub to_account_id: Option<Uuid>,

    /// Amount in cents
    ///
    /// Must be positive (enforced by CHECK constraint)
    pub amount_cents: i64,

    /// Currency code (ISO 4217)
    pub currency: String,

    /// Human-readable description
    pub description: Option<String>,

    /// Transaction status
    ///
    /// - "completed": Successfully applied
    /// - "failed": Rejected (e.g., insufficient funds)
    /// - "pending": In progress (future use)
    pub status: String,

    /// When transaction was created
    pub created_at: DateTime<Utc>,

    /// Additional metadata (JSON)
    pub metadata: Option<serde_json::Value>,
}

/// Request to credit (add money to) an account.
///
/// # JSON Example
///
/// ```json
/// {
///   "account_id": "550e8400-e29b-41d4-a716-446655440000",
///   "amount_cents": 100000,
///   "description": "Initial deposit",
///   "idempotency_key": "deposit-2025-001"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct CreditRequest {
    /// Account to credit (add money to)
    pub account_id: Uuid,

    /// Amount to add in cents
    pub amount_cents: i64,

    /// Optional description
    pub description: Option<String>,

    /// Optional idempotency key to prevent duplicates
    pub idempotency_key: Option<String>,
}

/// Request to debit (remove money from) an account.
///
/// # JSON Example
///
/// ```json
/// {
///   "account_id": "550e8400-e29b-41d4-a716-446655440000",
///   "amount_cents": 5000,
///   "description": "Monthly fee",
///   "idempotency_key": "fee-2025-12"
/// }
/// ```
///
/// # Validation
///
/// - Account must have sufficient balance
/// - Amount must be positive
/// - Account must belong to authenticated business
#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    /// Account to debit (remove money from)
    pub account_id: Uuid,

    /// Amount to remove in cents
    pub amount_cents: i64,

    /// Optional description
    pub description: Option<String>,

    /// Optional idempotency key to prevent duplicates
    pub idempotency_key: Option<String>,
}

/// Request to transfer money between accounts.
///
/// # JSON Example
///
/// ```json
/// {
///   "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
///   "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
///   "amount_cents": 25000,
///   "description": "Payment for services",
///   "idempotency_key": "invoice-789"
/// }
/// ```
///
/// # Atomicity Guarantee
///
/// BOTH accounts are updated in the same database transaction.
/// If debit fails, credit doesn't happen. If credit fails, debit is rolled back.
#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    /// Account to transfer from (will decrease)
    pub from_account_id: Uuid,

    /// Account to transfer to (will increase)
    pub to_account_id: Uuid,

    /// Amount to transfer in cents
    pub amount_cents: i64,

    /// Optional description
    pub description: Option<String>,

    /// Optional idempotency key to prevent duplicates
    pub idempotency_key: Option<String>,
}

/// Response returned for transaction operations.
///
/// # JSON Example
///
/// ```json
/// {
///   "id": "770e8400-e29b-41d4-a716-446655440002",
///   "transaction_type": "transfer",
///   "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
///   "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
///   "amount_cents": 25000,
///   "currency": "USD",
///   "description": "Payment for services",
///   "status": "completed",
///   "created_at": "2025-12-21T16:00:00Z"
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub transaction_type: String,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount_cents: i64,
    pub currency: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Convert database Transaction to API TransactionResponse.
///
/// This removes internal fields like metadata and idempotency_key
/// that clients don't need to see.
impl From<Transaction> for TransactionResponse {
    fn from(transaction: Transaction) -> Self {
        Self {
            id: transaction.id,
            transaction_type: transaction.transaction_type,
            from_account_id: transaction.from_account_id,
            to_account_id: transaction.to_account_id,
            amount_cents: transaction.amount_cents,
            currency: transaction.currency,
            description: transaction.description,
            status: transaction.status,
            created_at: transaction.created_at,
        }
    }
}
