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

    // Railway Compatibility: Map Railway's $PORT to VEX_PORT
    if let Ok(port) = std::env::var("PORT") {
        if std::env::var("VEX_PORT").is_err() {
            tracing::info!("Railway detected: Mapping PORT {} to VEX_PORT", port);
            std::env::set_var("VEX_PORT", port);
        }
    }

    // Railway Compatibility: Ensure VEX_JWT_SECRET exists to prevent startup crash.
    // In production, the user should override this in the Railway Dashboard for security.
    if std::env::var("VEX_JWT_SECRET").is_err() {
        tracing::warn!("VEX_JWT_SECRET not found! Using a temporary fallback secret.");
        std::env::set_var("VEX_JWT_SECRET", "railway-default-fallback-secret-32-chars-long");
    }

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
