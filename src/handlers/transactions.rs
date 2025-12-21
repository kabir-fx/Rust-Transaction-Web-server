//! Transaction HTTP handlers.
//!
//! This module implements transaction-related API endpoints:
//! - POST /api/v1/transactions/credit - Add money to account
//! - POST /api/v1/transactions/debit - Remove money from account
//! - POST /api/v1/transactions/transfer - Move money between accounts
//! - GET /api/v1/transactions/:id - Get transaction details

use crate::{
    db::DbPool,
    error::AppError,
    middleware::auth::AuthContext,
    models::transaction::{CreditRequest, DebitRequest, TransactionResponse, TransferRequest},
    services::transaction_service,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use uuid::Uuid;

/// Credit an account (add money).
///
/// # Request Body
///
/// ```json
/// {
///   "account_id": "550e8400-...",
///   "amount_cents": 100000,
///   "description": "Initial deposit",
///   "idempotency_key": "deposit-001"
/// }
/// ```
///
/// # Response (201)
///
/// ```json
/// {
///   "id": "770e8400-...",
///   "transaction_type": "credit",
///   "to_account_id": "550e8400-...",
///   "amount_cents": 100000,
///   "status": "completed",
///   "created_at": "2025-12-21T16:00:00Z"
/// }
/// ```
pub async fn create_credit(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreditRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    // Verify account belongs to authenticated business
    let account_id: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE id = $1 AND api_key_id = $2")
            .bind(request.account_id)
            .bind(auth.api_key_id)
            .fetch_optional(&pool)
            .await?
            .ok_or(AppError::AccountNotFound)?;

    // Execute credit transaction
    let transaction = transaction_service::execute_credit(
        &pool,
        account_id,
        request.amount_cents,
        request.description,
        request.idempotency_key,
    )
    .await?;

    Ok(Json(transaction.into()))
}

/// Debit an account (remove money).
///
/// # Endpoint
///
/// `POST /api/v1/transactions/debit`
///
/// # Validation
///
/// - Account must have sufficient balance
/// - Account must belong to authenticated business
pub async fn create_debit(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<DebitRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    // Verify account ownership
    let account_id: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE id = $1 AND api_key_id = $2")
            .bind(request.account_id)
            .bind(auth.api_key_id)
            .fetch_optional(&pool)
            .await?
            .ok_or(AppError::AccountNotFound)?;

    // Execute debit transaction
    let transaction = transaction_service::execute_debit(
        &pool,
        account_id,
        request.amount_cents,
        request.description,
        request.idempotency_key,
    )
    .await?;
    Ok(Json(transaction.into()))
}

/// Transfer money between accounts.
///
/// # Atomicity
///
/// Both accounts are updated in a single database transaction.
/// Either both succeed or both fail.
///
/// # Validation
///
/// - Both accounts must belong to authenticated business
/// - Source must have sufficient balance
/// - Accounts must be different
pub async fn create_transfer(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<TransferRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    // Verify both accounts belong to authenticated business
    // We fetch IDs to ensure they exist and belong to the user
    use sqlx::Row;
    let accounts = sqlx::query("SELECT id FROM accounts WHERE id = ANY($1) AND api_key_id = $2")
        .bind(&[request.from_account_id, request.to_account_id])
        .bind(auth.api_key_id)
        .fetch_all(&pool)
        .await?;

    if accounts.len() != 2 {
        return Err(AppError::AccountNotFound);
    }

    // Execute transfer
    let transaction = transaction_service::execute_transfer(
        &pool,
        request.from_account_id,
        request.to_account_id,
        request.amount_cents,
        request.description,
        request.idempotency_key,
    )
    .await?;

    Ok(Json(transaction.into()))
}

/// Get transaction by ID.
///
/// # Security
///
/// Returns 404 if transaction doesn't involve any accounts
/// belonging to the authenticated business.
pub async fn get_transaction(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<TransactionResponse>, AppError> {
    let transaction = transaction_service::get_transaction_by_id(&pool, transaction_id)
        .await?
        .ok_or(AppError::AccountNotFound)?;

    // Verify transaction involves at least one account owned by this business
    let has_access: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM accounts
            WHERE api_key_id = $1
            AND (id = $2 OR id = $3)
        )
        "#,
    )
    .bind(auth.api_key_id)
    .bind(transaction.from_account_id)
    .bind(transaction.to_account_id)
    .fetch_one(&pool)
    .await?;

    if !has_access {
        return Err(AppError::AccountNotFound);
    }

    Ok(Json(transaction.into()))
}
