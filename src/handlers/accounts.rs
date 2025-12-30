//! Account management HTTP handlers.
//!
//! This module implements the account-related API endpoints:
//! - POST /api/v1/accounts - Create new account
//! - GET /api/v1/accounts/:id - Get account by ID
//! - GET /api/v1/accounts - List all accounts for authenticated business

use crate::{
    db::DbPool,
    error::AppError,
    middleware::auth::AuthContext,
    models::account::{Account, AccountResponse, CreateAccountRequest},
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use uuid::Uuid;

/// Create a new account.
///
/// # Endpoint
///
/// `POST /api/v1/accounts`
///
/// # Authentication
///
/// Requires valid API key in Authorization header.
///
/// # Request Body
///
/// ```json
/// {
///   "account_name": "My Account",
///   "currency": "USD"  // optional, defaults to USD
/// }
/// ```
///
/// # Response
///
/// - **Success (201 Created)**: Returns the created account
/// - **Error (401)**: Invalid API key
/// - **Error (500)**: Database error
///
/// ```json
/// {
///   "id": "550e8400-e29b-41d4-a716-446655440000",
///   "account_name": "My Account",
///   "balance_cents": 0,
///   "currency": "USD",
///   "created_at": "2025-12-20T10:00:00Z",
///   "updated_at": "2025-12-20T10:00:00Z"
/// }
/// ```
///
/// # Arguments
///
/// * `State(pool)` - Database connection pool (injected by Axum)
/// * `Extension(auth)` - Authentication context (injected by auth middleware)
/// * `Json(request)` - Deserialized JSON request body
///
/// # Database Operation
///
/// Inserts a new row into `accounts` table with:
/// - api_key_id from auth context (ensures ownership)
/// - account_name from request
/// - currency from request (or "USD" default)
/// - balance_cents initialized to 0
pub async fn create_account(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO accounts (api_key_id, account_name, currency, balance_cents)
        VALUES ($1, $2, $3, $4)
        RETURNING id, api_key_id, account_name, balance_cents, currency, created_at, updated_at
        "#,
    )
    // Link to authenticated business
    .bind(auth.api_key_id)
    .bind(request.account_name)
    .bind(&request.currency)
    .bind(request.initial_balance_cents)
    .fetch_one(&pool)
    .await?;

    // Convert Account to AccountResponse (removes api_key_id)
    Ok(Json(account.into()))
}

/// Get a specific account by ID.
///
/// # Authentication
///
/// Requires valid API key. Returns 404 if account doesn't exist OR
/// belongs to a different business (prevents leaking existence of other accounts).
///
/// # URL Parameters
///
/// - `id` - UUID of the account to retrieve
///
/// # Response
///
/// - **Success (200 OK)**: Returns account details
/// - **Error (404)**: Account not found or not owned by authenticated business
/// - **Error (401)**: Invalid API key
///
/// # Security Note
///
/// The query filters by BOTH `id` AND `api_key_id` to ensure businesses
/// can only access their own accounts. This prevents:
/// - Account enumeration attacks
/// - Unauthorized access to other businesses' data
///
/// # Arguments
///
/// * `State(pool)` - Database connection pool
/// * `Extension(auth)` - Authentication context
/// * `Path(account_id)` - Account UUID from URL path
pub async fn get_account(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<AccountResponse>, AppError> {
    // Query account by ID AND api_key_id (security filter)
    let account = sqlx::query_as::<_, Account>(
        r#"
        SELECT id, api_key_id, account_name, balance_cents, currency, created_at, updated_at
        FROM accounts
        WHERE id = $1 AND api_key_id = $2
        "#,
    )
    .bind(account_id)
    // Ensure account belongs to this business
    .bind(auth.api_key_id)
    .fetch_optional(&pool)
    .await?
    // Return 404 if not found
    .ok_or(AppError::AccountNotFound)?;

    Ok(Json(account.into()))
}

/// List all accounts for the authenticated business.
///
/// # Endpoint
///
/// `GET /api/v1/accounts`
///
/// # Authentication
///
/// Requires valid API key.
///
/// # Response
///
/// - **Success (200 OK)**: Returns array of accounts (may be empty)
/// - **Error (401)**: Invalid API key
///
/// ```json
/// [
///   {
///     "id": "550e8400-e29b-41d4-a716-446655440000",
///     "account_name": "Account 1",
///     "balance_cents": 100000,
///     "currency": "USD",
///     "created_at": "2025-12-20T10:00:00Z",
///     "updated_at": "2025-12-20T10:00:00Z"
///   },
///   {
///     "id": "660e8400-e29b-41d4-a716-446655440001",
///     "account_name": "Account 2",
///     "balance_cents": 50000,
///     "currency": "USD",
///     "created_at": "2025-12-20T11:00:00Z",
///     "updated_at": "2025-12-20T11:00:00Z"
///   }
/// ]
/// ```
///
/// # Ordering
///
/// Accounts are returned in reverse chronological order (newest first).
///
/// # Arguments
///
/// * `State(pool)` - Database connection pool
/// * `Extension(auth)` - Authentication context
pub async fn list_accounts(
    State(pool): State<DbPool>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    // Fetch all accounts for this business
    let accounts = sqlx::query_as::<_, Account>(
        r#"
        SELECT id, api_key_id, account_name, balance_cents, currency, created_at, updated_at
        FROM accounts
        WHERE api_key_id = $1
        ORDER BY created_at DESC
        "#,
    )
    // Only fetch accounts for authenticated business
    .bind(auth.api_key_id)
    .fetch_all(&pool)
    .await?;

    // Convert each Account to AccountResponse
    let responses: Vec<AccountResponse> = accounts.into_iter().map(Into::into).collect();

    Ok(Json(responses))
}
