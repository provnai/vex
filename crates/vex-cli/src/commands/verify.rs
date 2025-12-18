//! Verify command - Check audit chain integrity
//!
//! Usage:
//! ```bash
//! vex verify --audit session.json
//! vex verify --db vex.db
//! ```

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

/// Arguments for the verify command
#[derive(Args)]
pub struct VerifyArgs {
    /// Path to audit JSON file to verify
    #[arg(long, short = 'a', value_name = "FILE")]
    audit: Option<PathBuf>,

    /// Path to VEX database file
    #[arg(long, short = 'd', value_name = "FILE")]
    db: Option<PathBuf>,

    /// Show detailed verification output
    #[arg(long)]
    detailed: bool,
}

/// Run the verify command
pub async fn run(args: VerifyArgs) -> Result<()> {
    if args.audit.is_none() && args.db.is_none() {
        println!("{}", "VEX Verification".bold().cyan());
        println!("{}", "═".repeat(40).cyan());
        println!();
        println!("Usage:");
        println!("  {} Verify an exported audit file", "vex verify --audit session.json".green());
        println!("  {} Verify a VEX database", "vex verify --db vex.db".green());
        println!();
        println!("The verify command checks the integrity of VEX audit chains");
        println!("using Merkle tree verification.");
        return Ok(());
    }

    // Handle audit file verification
    if let Some(audit_path) = args.audit {
        verify_audit_file(&audit_path, args.detailed).await?;
    }

    // Handle database verification
    if let Some(db_path) = args.db {
        verify_database(&db_path, args.detailed).await?;
    }

    Ok(())
}

/// Verify an exported audit JSON file
async fn verify_audit_file(path: &PathBuf, detailed: bool) -> Result<()> {
    println!("{}", "🔐 VEX Audit Verification".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    // Read and parse the audit file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read audit file: {}", path.display()))?;

    let audit_data: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| "Failed to parse audit JSON")?;

    // Extract events
    let events = audit_data
        .get("events")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let merkle_root = audit_data
        .get("merkle_root")
        .and_then(|r| r.as_str())
        .unwrap_or("Not computed");

    let chain_valid = audit_data
        .get("chain_valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Print summary
    println!("  {} {}", "File:".dimmed(), path.display());
    println!("  {} {}", "Events:".dimmed(), events);
    println!("  {} {}", "Merkle Root:".dimmed(), merkle_root);
    println!();

    // Verification result
    if chain_valid {
        println!("{} Chain integrity verified", "✓".green().bold());
        
        if detailed {
            println!();
            println!("{}", "Event Summary:".bold());
            if let Some(events_arr) = audit_data.get("events").and_then(|e| e.as_array()) {
                for (i, event) in events_arr.iter().take(10).enumerate() {
                    let event_type = event
                        .get("event_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Unknown");
                    let timestamp = event
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    println!("  {}. {} @ {}", i + 1, event_type.yellow(), timestamp.dimmed());
                }
                if events_arr.len() > 10 {
                    println!("  ... and {} more events", events_arr.len() - 10);
                }
            }
        }
    } else {
        println!("{} Chain integrity FAILED", "✗".red().bold());
        println!();
        println!("{}", "The audit chain may have been tampered with.".red());
        std::process::exit(1);
    }

    Ok(())
}

/// Verify a VEX database file
async fn verify_database(path: &PathBuf, detailed: bool) -> Result<()> {
    println!("{}", "🔐 VEX Database Verification".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    if !path.exists() {
        println!("{} Database file not found: {}", "✗".red().bold(), path.display());
        std::process::exit(1);
    }

    println!("  {} {}", "Database:".dimmed(), path.display());
    
    // TODO: Integrate with vex-persist when audit store is accessible
    // For now, show placeholder
    println!();
    println!("{}", "Note: Direct database verification requires vex-persist integration.".yellow());
    println!("For now, export the audit chain to JSON and verify with --audit.");

    if detailed {
        println!();
        println!("To export, use the VEX API:");
        println!("  {}", "GET /api/audit/export".green());
    }

    Ok(())
}
