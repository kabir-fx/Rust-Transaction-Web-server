//! Transaction Service - Main Application Entry Point
//!
//! This is a REST API server for managing financial accounts and transactions. It provides secure, authenticated endpoints for creating accounts and executing transactions (credit, debit, transfer).
//!
//! # Architecture
//!
//! - **Web Framework**: Axum (async HTTP server)
//! - **Database**: PostgreSQL with sqlx (async queries)
//! - **Authentication**: API key with SHA-256 hashing
//! - **Format**: JSON requests/responses
//!
//! # Startup Flow
//!
//! 1. Load configuration from environment variables
//! 2. Create database connection pool
//! 3. Run database migrations
//! 4. Build HTTP router with routes and middleware
//! 5. Start server on configured port

mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;

use tracing_subscriber::EnvFilter;

use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with tracing subscriber. Reads RUST_LOG environment variable (defaults to "info" level)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create database pool
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database pool created");

    // Run migrations
    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations complete");

    let app = Router::new()
        // Account management routes (all protected by auth middleware)
        .route("/api/v1/accounts", post(handlers::accounts::create_account))
        .route("/api/v1/accounts", get(handlers::accounts::list_accounts))
        .route(
            "/api/v1/accounts/{id}",
            get(handlers::accounts::get_account),
        )
        // Transaction routes
        .route(
            "/api/v1/transactions/credit",
            post(handlers::transactions::create_credit),
        )
        .route(
            "/api/v1/transactions/debit",
            post(handlers::transactions::create_debit),
        )
        .route(
            "/api/v1/transactions/transfer",
            post(handlers::transactions::create_transfer),
        )
        .route(
            "/api/v1/transactions/{id}",
            get(handlers::transactions::get_transaction),
        )
        // Apply authentication middleware to all routes above
        // This runs BEFORE route handlers and:
        // 1. Validates API key from Authorization header
        // 2. Injects AuthContext into request
        // 3. Returns 401 if authentication fails
        .route_layer(axum_middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::auth_middleware,
        ))
        // Add distributed tracing middleware for observability
        // Logs request/response details for debugging
        .layer(TraceLayer::new_for_http())
        // Share database pool with all handlers via State extraction
        .with_state(pool);

    // Bind to network address and start server
    let addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    // Start serving HTTP requests
    // This blocks forever, handling requests concurrently with tokio
    axum::serve(listener, app).await?;

    Ok(())
}
