//! Transaction Service - Main Application Entry Point

mod config;
mod db;
mod error;
mod models;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
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
    
    tracing::info!("Database ready!");

    Ok(())
}
