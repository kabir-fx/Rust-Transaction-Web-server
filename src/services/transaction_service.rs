//! Transaction service - Core business logic for financial transactions.
//!
//! This service handles:
//! - Atomic balance updates
//! - Idempotency checking
//! - Balance validation
//! - Database transaction management
//!
//! # Atomicity Guarantees
//!
//! All balance updates happen within PostgreSQL transactions.
//! The database ensures all-or-nothing execution.

use crate::{db::DbPool, error::AppError, models::transaction::Transaction};
use uuid::Uuid;

/// Execute a credit transaction (add money to account).
///
/// # Process
///
/// 1. Check for duplicate idempotency key
/// 2. Start database transaction
/// 3. Lock and update account balance
/// 4. Record transaction
/// 5. Commit (or rollback on error)
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `account_id` - Account to credit
/// * `amount_cents` - Amount to add (must be positive)
/// * `description` - Optional description
/// * `idempotency_key` - Optional key for deduplication
///
/// # Returns
///
/// The created (or existing) transaction record
///
/// # Errors
///
/// - `AccountNotFound`: Account doesn't exist
/// - `InvalidRequest`: Amount is zero or negative
/// - `Database`: Database error occurred
pub async fn execute_credit(
    pool: &DbPool,
    account_id: Uuid,
    amount_cents: i64,
    description: Option<String>,
    idempotency_key: Option<String>,
) -> Result<Transaction, AppError> {
    // Validate amount
    if amount_cents <= 0 {
        return Err(AppError::InvalidRequest(
            "Amount must be positive".to_string(),
        ));
    }

    // Check for duplicate idempotency key
    if let Some(ref key) = idempotency_key {
        if let Some(existing) = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        {
            return Ok(existing);
        }
    }

    // Start db transaction
    let mut tx = pool.begin().await?;

    // Lock the account and update balance
    // FOR UPDATE ensures no other transaction can modify this row
    let updated_count = sqlx::query(
        r#"
        UPDATE accounts
        SET balance_cents = balance_cents + $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(amount_cents)
    .bind(account_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if updated_count == 0 {
        tx.rollback().await?;
        return Err(AppError::AccountNotFound);
    }

    // Record the transaction
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (
            transaction_type,
            to_account_id,
            amount_cents,
            description,
            idempotency_key,
            status
        )
        VALUES ('credit', $1, $2, $3, $4, 'completed')
        RETURNING *
        "#,
    )
    .bind(account_id)
    .bind(amount_cents)
    .bind(description)
    .bind(idempotency_key)
    .fetch_one(&mut *tx)
    .await?;

    // Commit all changes atomically
    tx.commit().await?;

    Ok(transaction)
}

/// Execute a debit transaction (remove money from account).
pub async fn execute_debit(
    pool: &DbPool,
    account_id: Uuid,
    amount_cents: i64,
    description: Option<String>,
    idempotency_key: Option<String>,
) -> Result<Transaction, AppError> {
    // Validate amount
    if amount_cents <= 0 {
        return Err(AppError::InvalidRequest(
            "Amount must be positive".to_string(),
        ));
    }

    // Check for duplicate idempotency key
    if let Some(ref key) = idempotency_key {
        if let Some(existing) = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        {
            return Ok(existing);
        }
    }

    // Start database transaction
    let mut tx = pool.begin().await?;
    // Lock account and check balance
    let balance_cents: i64 =
        sqlx::query_scalar("SELECT balance_cents FROM accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::AccountNotFound)?;

    // Validate sufficient balance
    if balance_cents < amount_cents {
        tx.rollback().await?;
        return Err(AppError::InsufficientBalance);
    }

    // Update balance
    sqlx::query(
        r#"
        UPDATE accounts
        SET balance_cents = balance_cents - $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(amount_cents)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;

    // Record transaction
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (
            transaction_type,
            from_account_id,
            amount_cents,
            description,
            idempotency_key,
            status
        )
        VALUES ('debit', $1, $2, $3, $4, 'completed')
        RETURNING *
        "#,
    )
    .bind(account_id)
    .bind(amount_cents)
    .bind(description)
    .bind(idempotency_key)
    .fetch_one(&mut *tx)
    .await?;
    // Commit atomically
    tx.commit().await?;

    Ok(transaction)
}

/// Execute a transfer transaction (move money between accounts).
pub async fn execute_transfer(
    pool: &DbPool,
    from_account_id: Uuid,
    to_account_id: Uuid,
    amount_cents: i64,
    description: Option<String>,
    idempotency_key: Option<String>,
) -> Result<Transaction, AppError> {
    // Validate amount
    if amount_cents <= 0 {
        return Err(AppError::InvalidRequest(
            "Amount must be positive".to_string(),
        ));
    }

    // Prevent transferring to same account
    if from_account_id == to_account_id {
        return Err(AppError::InvalidRequest(
            "Cannot transfer to same account".to_string(),
        ));
    }

    // Check for duplicate idempotency key
    if let Some(ref key) = idempotency_key {
        if let Some(existing) = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        {
            return Ok(existing);
        }
    }
    // Start database transaction - THIS IS THE MAGIC
    let mut tx = pool.begin().await?;

    // Lock source account and check balance
    // FOR UPDATE prevents other transactions from modifying
    let from_balance: i64 =
        sqlx::query_scalar("SELECT balance_cents FROM accounts WHERE id = $1 FOR UPDATE")
            .bind(from_account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::AccountNotFound)?;

    if from_balance < amount_cents {
        tx.rollback().await?;
        return Err(AppError::InsufficientBalance);
    }

    // Lock destination account
    let to_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1 FOR UPDATE)")
            .bind(to_account_id)
            .fetch_one(&mut *tx)
            .await?;

    if !to_exists {
        tx.rollback().await?;
        return Err(AppError::AccountNotFound);
    }

    // Update both balances atomically
    sqlx::query(
        "UPDATE accounts SET balance_cents = balance_cents - $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(amount_cents)
    .bind(from_account_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE accounts SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(amount_cents)
    .bind(to_account_id)
    .execute(&mut *tx)
    .await?;

    // Record transaction
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (
            transaction_type,
            from_account_id,
            to_account_id,
            amount_cents,
            description,
            idempotency_key,
            status
        )
        VALUES ('transfer', $1, $2, $3, $4, $5, 'completed')
        RETURNING *
        "#,
    )
    .bind(from_account_id)
    .bind(to_account_id)
    .bind(amount_cents)
    .bind(description)
    .bind(idempotency_key)
    .fetch_one(&mut *tx)
    .await?;

    // Commit ALL changes atomically
    // If this fails, everything rolls back
    tx.commit().await?;

    Ok(transaction)
}

/// Get transaction by ID.
pub async fn get_transaction_by_id(
    pool: &DbPool,
    transaction_id: Uuid,
) -> Result<Option<Transaction>, AppError> {
    let transaction = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = $1")
        .bind(transaction_id)
        .fetch_optional(pool)
        .await?;

    Ok(transaction)
}
