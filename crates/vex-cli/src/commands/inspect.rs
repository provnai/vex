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

    /// Show detailed binary information (hex segments)
    #[arg(long, short = 'x')]
    pub hex: bool,
}

/// Run the inspect command
pub async fn run(args: InspectArgs) -> Result<()> {
    println!("{}", "📚 VEX Protocol Deep Inspection (v1.0)".bold().cyan());
    println!("{}", "═".repeat(45).cyan());
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

    if args.hex {
        let magic = &bytes[0..3];
        println!(
            "  {} {} ({})",
            "Magic Bytes:".dimmed(),
            hex::encode(magic).green(),
            String::from_utf8_lossy(magic).bold()
        );
    }

    println!("  {} {}", "Version:".dimmed(), header.version);
    println!(
        "  {} {}",
        "Agent ID (AID):".dimmed(),
        hex::encode(header.aid).blue()
    );
    println!(
        "  {} {}",
        "Capsule Root:".dimmed(),
        hex::encode(header.capsule_root).magenta()
    );
    println!(
        "  {} {}",
        "Nonce:".dimmed(),
        u64::from_be_bytes(header.nonce).to_string().yellow()
    );

    if args.hex {
        println!("  {} {} bytes", "Packet Len:".dimmed(), bytes.len());
    }
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
            println!("{}", "📦 TLV Binary Segments (v1.0):".bold());

            let segments = [
                (VepSegmentType::Intent, "Intent Capsule (JSON)"),
                (VepSegmentType::MagpieAst, "Formal Intent (AST)"),
                (VepSegmentType::Identity, "Silicon Ident (PCRs)"),
                (VepSegmentType::Witness, "Witness Proof (Ed25519)"),
            ];

            for (seg_type, label) in segments {
                if let Some(seg_bytes) = packet.get_segment_data(seg_type) {
                    println!(
                        "  {} [Type: {:02X}] [{} bytes]",
                        label.yellow(),
                        seg_type as u8,
                        seg_bytes.len()
                    );

                    if args.hex {
                        let hex_preview = if seg_bytes.len() > 32 {
                            format!("{}...", hex::encode(&seg_bytes[0..32]))
                        } else {
                            hex::encode(seg_bytes)
                        };
                        println!(
                            "     {} {}",
                            "Raw Hex:".dimmed(),
                            hex_preview.italic().dimmed()
                        );
                    }

                    if seg_type == VepSegmentType::Intent {
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(seg_bytes) {
                            let json_str = json.to_string();
                            let snippet: String = json_str.chars().take(80).collect();
                            println!(
                                "     {} {}",
                                "Preview:".dimmed(),
                                format!("{}...", snippet).italic()
                            );
                        }
                    }
                }
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
                "Verification failed because the VEP body does not match the header root hash."
                    .dimmed()
            );
        }
    }

    println!();
    println!("{} Read complete.", "✓".green().bold());

    Ok(())
}
