//! Application configuration management.
//!
//! This module handles loading configuration from environment variables.
//! It uses the `envy` crate to automatically deserialize environment variables into a type-safe struct.

use serde::Deserialize;

/// Application configuration loaded from environment variables.
///
/// # Environment Variables
///
/// - `DATABASE_URL` (required): PostgreSQL connection string
/// - `SERVER_PORT` (optional): HTTP server port, defaults to 3000
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,

    #[serde(default = "default_port")]
    pub server_port: u16,
}

/// Default port if SERVER_PORT environment variable is not set.
fn default_port() -> u16 {
    3000
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// This method first attempts to load a `.env` file (which is optional),
    /// then reads environment variables and deserializes them into a Config struct.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required environment variables are missing (e.g., DATABASE_URL)
    /// - Environment variable values cannot be parsed into expected types
    pub fn from_env() -> Result<Self, envy::Error> {
        // Try to load .env file if it exists (does nothing if not found)
        dotenvy::dotenv().ok();

        // Parse environment variables into Config struct
        // Field names are automatically converted: database_url -> DATABASE_URL
        envy::from_env::<Config>()
    }
}
