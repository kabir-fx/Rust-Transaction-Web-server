//! Database connection pool and migration management.
//!
//! This module provides utilities for:
//! - Creating and managing a PostgreSQL connection pool
//! - Running database migrations automatically

use sqlx::{Pool, Postgres};

/// Type alias for PostgreSQL connection pool.
///
/// Instead of writing `Pool<Postgres>` everywhere, we can use `DbPool`.
pub type DbPool = Pool<Postgres>;

/// Create a new PostgreSQL connection pool.
///
/// A connection pool maintains multiple database connections that can be reused across HTTP requests which is much more efficient than opening a new connection for each request.
///
/// # Arguments
///
/// * `database_url` - PostgreSQL connection string
///
/// # Configuration
///
/// - Maximum connections: 5 (configurable via PgPoolOptions)
/// - Connections are created lazily as needed
/// - Idle connections are kept alive for reuse
///
/// # Errors
///
/// Returns an error if:
/// - Database connection string is invalid
/// - Cannot connect to PostgreSQL server
/// - Database authentication fails
pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        // Limit concurrent connections
        .max_connections(5)
        .connect(database_url)
        .await
}

/// Run database migrations from the `migrations/` directory.
///
/// This function executes all SQL migration files in order. Migrations are tracked in a special `_sqlx_migrations` table, so each migration runs only once.
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Migration Files
///
/// Migration files must be in `migrations/` directory with format:
/// - `<timestamp>_<name>.sql` (e.g., `20250101000001_create_users.sql`)
///
/// # Errors
///
/// Returns an error if:
/// - Migration files cannot be read
/// - SQL syntax errors in migration files
/// - Database errors during migration execution
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    // The macro reads migrations at compile time from ./migrations directory
    sqlx::migrate!("./migrations").run(pool).await
}
