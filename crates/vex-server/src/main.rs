//! VEX Server - Standalone entry point for the VEX Protocol API
//!
//! This crate serves as a thin wrapper around `vex-api` to provide
//! a runnable binary for production deployments without modifying
//! the core library crate.

use anyhow::Result;
use vex_api::{ServerConfig, VexServer};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing using the standard configuration from vex-api
    vex_api::server::init_tracing();

    tracing::info!("🚀 Starting VEX Protocol Server...");

    // Load server configuration from environment variables (e.g., VEX_PORT)
    let config = ServerConfig::from_env();

    // Initialize the server state and dependencies
    let server = VexServer::new(config).await.map_err(|e| {
        tracing::error!("Failed to initialize server: {}", e);
        e
    })?;

    // Run the server with graceful shutdown support
    server.run().await.map_err(|e| {
        tracing::error!("Server error during execution: {}", e);
        e
    })?;

    Ok(())
}
