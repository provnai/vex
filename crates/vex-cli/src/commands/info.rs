//! Info command - Show system information
//!
//! Usage:
//! ```bash
//! vex info
//! ```

use anyhow::Result;
use clap::Args;
use colored::Colorize;

/// Arguments for the info command
#[derive(Args)]
pub struct InfoArgs;

/// Run the info command
pub fn run(_args: InfoArgs) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    println!(
        "{}",
        "VEX - Verified Evolutionary Xenogenesis".bold().cyan()
    );
    println!("{}", "═".repeat(50).cyan());
    println!();

    println!("{}", "Version Information:".bold());
    println!("  {} {}", "CLI Version:".dimmed(), version.green());
    println!("  {} {}", "vex-core:".dimmed(), "0.1.1".green());
    println!("  {} {}", "vex-llm:".dimmed(), "0.1.1".green());
    println!("  {} {}", "vex-persist:".dimmed(), "0.1.1".green());
    println!();

    println!("{}", "Features:".bold());
    println!("  {} Cryptographic Merkle verification", "✓".green());
    println!("  {} Adversarial consensus (multi-LLM)", "✓".green());
    println!("  {} Tool execution with audit trail", "✓".green());
    println!("  {} Self-correcting genome evolution", "✓".green());
    println!();

    println!("{}", "Built-in Tools:".bold());
    let registry = vex_llm::tools::builtin_registry();
    for name in registry.names() {
        println!("  {} {}", "•".cyan(), name.green());
    }
    println!();

    println!("{}", "Configuration:".bold());
    println!("  {} Check README.md for environment variables", "ℹ".blue());
    println!();

    println!("{}", "Links:".bold());
    println!(
        "  {} {}",
        "Repository:".dimmed(),
        "https://github.com/provnai/vex".underline()
    );
    println!(
        "  {} {}",
        "Documentation:".dimmed(),
        "https://provnai.dev/docs".underline()
    );
    println!();

    Ok(())
}
