//! API key authentication middleware.
//!
//! This middleware intercepts every protected request to:
//! 1. Extract the API key from the Authorization header
//! 2. Hash it and verify it exists in the database
//! 3. Inject authentication context into the request
//! 4. Reject unauthorized requests with HTTP 401

use crate::{db::DbPool, error::AppError, models::api_key::ApiKey};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Authentication context attached to authenticated requests.
///
/// This struct is inserted into the request's extension map and can be
/// extracted by route handlers to know who made the request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// ID of the authenticated API key
    ///
    /// Used to filter database queries (e.g., only show accounts for this business)
    pub api_key_id: Uuid,

    /// Name of the business making the request
    pub business_name: String,
}

/// API key authentication middleware function.
///
/// # Flow
///
/// 1. Extract `Authorization: Bearer <key>` header from request
/// 2. Hash the `<key>` using SHA-256
/// 3. Query database for matching hash where `is_active = true`
/// 4. If found: inject `AuthContext` into request, call next handler
/// 5. If not found: return 401 Unauthorized error
///
/// # Headers
///
/// Expected header format:
/// ```
/// Authorization: Bearer abc123xyz
/// ```
///
/// # Arguments
///
/// * `State(pool)` - Database connection pool injected by Axum
/// * `request` - Incoming HTTP request (mutable to add extensions)
/// * `next` - Next middleware/handler in the chain
///
/// # Returns
///
/// - `Ok(Response)` if authenticated successfully (calls next handler)
/// - `Err(AppError::InvalidApiKey)` if authentication fails (returns 401)
pub async fn auth_middleware(
    State(pool): State<DbPool>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Step 1: Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::InvalidApiKey)?;

    // Step 2: Extract Bearer token
    // Expected format: "Bearer <api_key>"
    let api_key = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidApiKey)?;

    // Step 3: Hash the API key using SHA-256
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());

    let key_hash = hex::encode(hasher.finalize());

    // Step 4: Lookup hashed key in database
    let api_key_record = sqlx::query_as::<_, ApiKey>(
        "SELECT id, key_hash, business_name, created_at, is_active 
         FROM api_keys 
         WHERE key_hash = $1 AND is_active = true",
    )
    .bind(&key_hash)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::InvalidApiKey)?;

    // Step 5: Create authentication context
    let auth_context = AuthContext {
        api_key_id: api_key_record.id,
        business_name: api_key_record.business_name,
    };

    // Step 6: Inject context into request extensions
    // Route handlers can now extract this using Extension<AuthContext>
    request.extensions_mut().insert(auth_context);

    // Step 7: Call the next middleware/handler
    Ok(next.run(request).await)
}
