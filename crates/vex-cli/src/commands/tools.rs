//! Tools command - List and execute VEX tools
//!
//! Usage:
//! ```bash
//! vex tools list
//! vex tools run calculator '{"expression": "2+2"}'
//! vex tools schema calculator
//! ```

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

use vex_llm::{tools::builtin_registry, ToolExecutor};

/// Arguments for the tools command
#[derive(Args)]
pub struct ToolsArgs {
    #[command(subcommand)]
    command: ToolsCommand,
}

#[derive(Subcommand)]
pub enum ToolsCommand {
    /// List all available tools
    #[command(name = "list")]
    List,

    /// Run a tool with JSON arguments
    #[command(name = "run")]
    Run {
        /// Name of the tool to run
        name: String,

        /// JSON arguments for the tool
        #[arg(default_value = "{}")]
        args: String,

        /// Output raw JSON (no formatting)
        #[arg(long)]
        raw: bool,
    },

    /// Show the JSON schema for a tool
    #[command(name = "schema")]
    Schema {
        /// Name of the tool
        name: String,
    },
}

/// Run the tools command
pub async fn run(args: ToolsArgs) -> Result<()> {
    match args.command {
        ToolsCommand::List => list_tools(),
        ToolsCommand::Run { name, args, raw } => run_tool(&name, &args, raw).await,
        ToolsCommand::Schema { name } => show_schema(&name),
    }
}

/// List all available tools
fn list_tools() -> Result<()> {
    let registry = builtin_registry();

    println!("{}", "🧰 VEX Built-in Tools".bold().cyan());
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
            Cell::new("Capabilities").fg(Color::Cyan),
        ]);

    for def in registry.definitions() {
        // Get capabilities for this tool
        // Safe: we iterate over definitions that exist in the registry
        if let Some(tool) = registry.get(def.name) {
            let caps: Vec<String> = tool
                .capabilities()
                .iter()
                .map(|c| format!("{:?}", c))
                .collect();

            table.add_row(vec![
                Cell::new(def.name).fg(Color::Green),
                Cell::new(def.description),
                Cell::new(caps.join(", ")).fg(Color::Yellow),
            ]);
        }
    }

    println!("{table}");
    println!();
    println!(
        "Run a tool: {}",
        "vex tools run <name> '<json_args>'".green()
    );
    println!("Show schema: {}", "vex tools schema <name>".green());

    Ok(())
}

/// Run a tool with JSON arguments
async fn run_tool(name: &str, args_str: &str, raw: bool) -> Result<()> {
    let registry = builtin_registry();
    let executor = ToolExecutor::new(registry);

    // Parse JSON arguments
    let args: serde_json::Value =
        serde_json::from_str(args_str).with_context(|| format!("Invalid JSON: {}", args_str))?;

    if !raw {
        println!("{} Running tool '{}'...", "⚙".blue(), name.green());
        println!();
    }

    // Execute the tool
    let result = executor
        .execute(name, args)
        .await
        .with_context(|| format!("Tool '{}' execution failed", name))?;

    if raw {
        // Raw JSON output
        println!("{}", serde_json::to_string_pretty(&result.output)?);
    } else {
        // Formatted output
        println!("{}", "Result:".bold());
        println!("{}", serde_json::to_string_pretty(&result.output)?);
        println!();
        println!("{} {}", "Hash:".dimmed(), result.hash);
        println!("{} {}ms", "Execution time:".dimmed(), result.execution_ms());
    }

    Ok(())
}

/// Show the JSON schema for a tool
fn show_schema(name: &str) -> Result<()> {
    let registry = builtin_registry();

    let tool = registry.get(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Tool '{}' not found. Run 'vex tools list' to see available tools.",
            name
        )
    })?;

    let def = tool.definition();

    println!("{} Schema for '{}'", "📋".cyan(), name.green().bold());
    println!();
    println!("{}", "Name:".bold());
    println!("  {}", def.name);
    println!();
    println!("{}", "Description:".bold());
    println!("  {}", def.description);
    println!();
    println!("{}", "Parameters (JSON Schema):".bold());

    // Pretty print the parameters JSON
    let params: serde_json::Value =
        serde_json::from_str(def.parameters).unwrap_or(serde_json::json!({}));
    println!("{}", serde_json::to_string_pretty(&params)?);

    println!();
    println!("{}", "OpenAI Format:".bold());
    println!("{}", serde_json::to_string_pretty(&def.to_openai_format())?);

    Ok(())
}
