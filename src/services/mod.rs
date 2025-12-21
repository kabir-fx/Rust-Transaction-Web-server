//! Business logic services.
//!
//! Services contain core business logic separated from HTTP handlers.
//! They handle database transactions, validation, and complex operations.

/// Transaction service for atomic credit, debit, and transfer operations
pub mod transaction_service;

/// Webhook service for endpoint registration and event delivery
pub mod webhook_service;
