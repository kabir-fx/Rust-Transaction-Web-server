//! HTTP middleware components.
//!
//! Middleware are functions that run before route handlers.
//! They can:
//! - Authenticate requests
//! - Log requests
//! - Modify request/response
//! - Short-circuit requests (reject unauthorized)

/// API key authentication middleware
pub mod auth;
