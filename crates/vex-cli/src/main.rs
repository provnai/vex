//! VEX CLI - Command-line interface for verified AI agent tooling
//!
//! # Usage
//!
//! ```bash
//! # Verify an audit chain
//! vex verify --audit session.json
//!
//! # List available tools
//! vex tools list
//!
//! # Run a tool directly
//! vex tools run calculator '{"expression": "2+2"}'
//!
//! # Show version and configuration
//! vex info
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod commands;

use commands::{info, tools, verify};

/// VEX - Verified Evolutionary Xenogenesis
///
/// Command-line interface for provable AI agent tooling.
/// Every action is cryptographically verified.
#[derive(Parser)]
#[command(
    name = "vex",
    version,
    about = "VEX CLI - Verified AI Agent Tooling",
    long_about = "VEX provides cryptographically-verified AI agent tools.\n\n\
                  Every tool execution is hashed into a Merkle chain,\n\
                  enabling tamper-proof audit trails and verification."
)]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify audit chain integrity
    #[command(name = "verify")]
    Verify(verify::VerifyArgs),

    /// Tool management and execution
    #[command(name = "tools")]
    Tools(tools::ToolsArgs),

    /// Show system information
    #[command(name = "info")]
    Info(info::InfoArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup tracing based on verbosity
    setup_logging(cli.verbose);

    // Execute command
    match cli.command {
        Commands::Verify(args) => verify::run(args).await,
        Commands::Tools(args) => tools::run(args).await,
        Commands::Info(args) => info::run(args),
    }
}

/// Setup logging based on verbosity level
fn setup_logging(verbosity: u8) {
    use tracing_subscriber::EnvFilter;

    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(filter))
        )
        .init();
}

/// Print a success message with a checkmark
#[allow(dead_code)]
pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print an error message with an X
#[allow(dead_code)]
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Print a warning message
#[allow(dead_code)]
pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".yellow().bold(), msg);
}

/// Print an info message
#[allow(dead_code)]
pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".blue().bold(), msg);
}
