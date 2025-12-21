//! HTTP request handlers (route handlers).
//!
//! Each handler is an async function that:
//! 1. Receives HTTP request data (JSON body, URL params, etc.)
//! 2. Performs business logic (database queries, validation)
//! 3. Returns HTTP response (JSON, status code)
/// Account management endpoints
pub mod accounts;