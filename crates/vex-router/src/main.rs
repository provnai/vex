//! VEX Router - Main entry point (standalone mode)

use tracing_subscriber;

#[cfg(feature = "standalone")]
use vex_router::Server;

#[cfg(feature = "standalone")]
use vex_router::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("\n🔀 VEX Router - Intelligent LLM Routing for VEX Protocol");
    println!("============================================================\n");

    #[cfg(feature = "standalone")]
    {
        let config = Config::default();

        println!("Configuration:");
        println!("  - Models: {}", config.models.len());
        println!("  - Default strategy: {:?}", config.default_strategy);
        println!(
            "  - Quality threshold: {:.0}%",
            config.quality_threshold * 100.0
        );
        println!("  - Learning enabled: {}", config.learning_enabled);
        println!();

        let server = Server::new(config);
        server.run().await?;
    }

    #[cfg(not(feature = "standalone"))]
    {
        // Library mode - show usage
        println!("VEX Router compiled in library mode.");
        println!();
        println!("Usage:");
        println!("  use vex_router::{Router, RoutingStrategy};");
        println!();
        println!("  let router = Router::builder()");
        println!("      .strategy(RoutingStrategy::Auto)");
        println!("      .build();");
        println!();
        println!("  let response = router.ask(\"Hello\").await?;");
    }

    Ok(())
}
