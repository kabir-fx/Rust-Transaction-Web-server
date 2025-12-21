//! HTTP request handlers (route handlers).
//!
//! Each handler is an async function that:
//! 1. Receives HTTP request data (JSON body, URL params, etc.)
/// HTTP handlers for various API endpoints
///
/// Handlers receive HTTP requests, validate input, call service layer,
/// and return HTTP responses.

/// Account management handlers
pub mod accounts;

/// Health check handler for monitoring
pub mod health;

/// Transaction handlers for credit, debit, and transfer operations
pub mod transactions;

/// Webhook endpoint management handlers
pub mod webhooks;
