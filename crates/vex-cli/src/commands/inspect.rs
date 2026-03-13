use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;
use vex_core::vep::VepPacket;
use vex_core::vep::VepSegmentType;

/// Arguments for the inspect command
#[derive(Args)]
pub struct InspectArgs {
    /// Path to binary VEP file to inspect
    #[arg(value_name = "FILE")]
    pub path: PathBuf,
}

/// Run the inspect command
pub async fn run(args: InspectArgs) -> Result<()> {
    println!("{}", "🔍 VEX Payload Inspection".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    // Read binary file
    let bytes = std::fs::read(&args.path)
        .with_context(|| format!("Failed to read VEP file: {}", args.path.display()))?;

    // Parse VEP using the core VepPacket logic
    let packet = VepPacket::new(&bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse binary VEP header: {}", e))?;

    let header = packet.header();

    // Display Header Info
    println!("  {} {}", "File:".dimmed(), args.path.display());
    println!("  {} {}", "Version:".dimmed(), header.version);
    println!(
        "  {} {}",
        "Agent ID (AID):".dimmed(),
        hex::encode(header.aid)
    );
    println!(
        "  {} {}",
        "Capsule Root:".dimmed(),
        hex::encode(header.capsule_root)
    );
    println!(
        "  {} {}",
        "Nonce:".dimmed(),
        u64::from_be_bytes(header.nonce)
    );
    println!();

    // Try to reconstruct the capsule for more detail
    println!("{}", "🛡️ Verification Status:".bold());
    match packet.to_capsule() {
        Ok(capsule) => {
            let outcome_color = match capsule.authority.outcome.to_uppercase().as_str() {
                "ALLOW" => "green",
                "HALT" => "red",
                "ESCALATE" => "yellow",
                _ => "white",
            };
            println!(
                "  {} {}",
                "Outcome:".dimmed(),
                capsule.authority.outcome.color(outcome_color).bold()
            );
            println!(
                "  {} {}",
                "Reason Code:".dimmed(),
                capsule.authority.reason_code
            );
            println!(
                "  {} {}",
                "HW Identity (AID):".dimmed(),
                capsule.identity.aid
            );
            if let Some(pcrs) = &capsule.identity.pcrs {
                println!("  {}", "Hardware PCR State (at time of signing):".dimmed());
                let mut sorted_indices: Vec<_> = pcrs.keys().collect();
                sorted_indices.sort();
                for index in sorted_indices {
                    let hash = &pcrs[index];
                    println!(
                        "    {} {} {}",
                        "PCR".dimmed(),
                        index.to_string().cyan().bold(),
                        hash.italic()
                    );
                }
            }

            println!();
            println!("{}", "📄 Evidence Summary:".bold());

            if let Some(intent_bytes) = packet.get_segment_data(VepSegmentType::Intent) {
                println!(
                    "  {} [{} bytes]",
                    "Intent Segment".yellow(),
                    intent_bytes.len()
                );
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(intent_bytes) {
                    let json_str = json.to_string();
                    let snippet: String = json_str.chars().take(100).collect();
                    println!("     {}", format!("{}...", snippet).italic().dimmed());
                }
            }

            if let Some(ast_bytes) = packet.get_segment_data(VepSegmentType::MagpieAst) {
                println!(
                    "  {} [{} bytes]",
                    "Formal Witness (Magpie AST)".yellow(),
                    ast_bytes.len()
                );
            }

            if let Some(witness_bytes) = packet.get_segment_data(VepSegmentType::Witness) {
                println!(
                    "  {} [{} bytes]",
                    "Witness Segment".yellow(),
                    witness_bytes.len()
                );
            }
        }
        Err(e) => {
            println!(
                "  {} {}",
                "Error:".red().bold(),
                format!("Capsule reconstruction failed: {}", e).dimmed()
            );
            println!(
                "  {}",
                "The packet header is valid, but internal segments may be corrupted or missing."
                    .dimmed()
            );
        }
    }

    println!();
    println!("{} Read complete.", "✓".green().bold());

    Ok(())
}
