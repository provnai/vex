use anyhow::Result;
use attest_rs::{AttestAgent, AttestConfig, AuditStore, LocalStore};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch a directory for changes
    Watch {
        /// Directory to watch
        path: PathBuf,
    },
    /// Show agent identity
    Id,
    /// Run a command through the terminal interceptor
    Run {
        /// Command to execute
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Query the audit log
    Query {
        /// Limit the number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Verify the integrity and signatures of the audit log
    Verify {
        /// Use succinct ZK verification (Plonky3)
        #[arg(short, long)]
        succinct: bool,
    },
    /// Manage and test the System-Level Integrity (eBPF) module
    Kernel {
        #[command(subcommand)]
        command: KernelCommands,
    },
}

#[derive(Subcommand)]
enum KernelCommands {
    /// Test a policy against a simulated connection event
    Test {
        /// The IP address to test
        #[arg(long)]
        ip: String,
        /// The port to test
        #[arg(long)]
        port: u16,
        /// Path to the policy bytecode file (optional, defaults to allow-all)
        #[arg(long)]
        policy: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // 1. Load Configuration
    let config = AttestConfig::new().expect("Failed to load configuration");
    let cli = Cli::parse();

    // 2. Determine Identity Path (Same as before)
    let identity_path = config
        .identity_path
        .as_ref()
        .map(PathBuf::from)
        .expect("Identity path missing");
    if let Some(parent) = identity_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // 3. Initialize Identity
    let agent = if identity_path.exists() {
        println!("🔑 Found existing identity at {:?}", identity_path);
        println!("Enter password to unlock:");
        let password = rpassword::read_password()?;
        attest_rs::KeyManager::load(&identity_path, &password).await?
    } else {
        println!("🆕 Creating new identity at {:?}", identity_path);
        println!("Enter password to secure your new identity:");
        let password = rpassword::read_password()?;
        let agent = AttestAgent::new();
        attest_rs::KeyManager::save(&identity_path, &agent, &password).await?;
        agent
    };
    let agent = Arc::new(agent);
    println!("✅ Identity loaded: {}", agent.id);

    // 4. Initialize Store
    let local_store = Arc::new(LocalStore::new(&config.database_path).await?);
    let audit_store = Arc::new(Mutex::new(AuditStore::new(local_store.clone())));

    // 5. Execute Command
    match &cli.command {
        Commands::Id => {
            println!("Agent ID:     {}", agent.id);
            println!("VEX UUID:     {}", agent.to_vex_uuid());
            println!("Storage:      {}", config.database_path);
            println!("Identity:     {:?}", identity_path);
        }
        Commands::Watch { path } => {
            println!("🚀 Starting Attest Watchdog on: {:?}", path);
            let watcher = attest_rs::AttestWatcher::new(agent, audit_store.clone());
            watcher.watch(path).await?;
        }
        Commands::Run { command } => {
            let cmd_line = command.join(" ");
            if cmd_line.is_empty() {
                println!("Usage: attest run <command>");
                return Ok(());
            }
            println!("🕵️ Intercepting: {}", cmd_line);

            // Initialize Policy Engine
            let mut policy_engine = attest_rs::PolicyEngine::new();
            policy_engine.load_defaults();
            let policy_engine = Arc::new(tokio::sync::Mutex::new(policy_engine));

            let interceptor = attest_rs::AttestTerminalInterceptor::new(
                audit_store.clone(),
                agent.clone(),
                policy_engine,
            );
            interceptor.run_command(&cmd_line).await?;
        }
        Commands::Query { limit } => {
            println!("📜 Audit Log (Last {} events):", limit);
            let audit = audit_store.lock().await;
            let events = audit.local.get_all_events().await?;
            for event in events.iter().rev().take(*limit) {
                println!(
                    "[{}] {} | Type: {:?} | Data: {}",
                    event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    event.sequence_number,
                    event.event_type,
                    event.data
                );
            }
        }
        Commands::Verify { succinct } => {
            let audit = audit_store.lock().await;

            if *succinct {
                println!("🔍 Verifying succinct ZK integrity proofs (Plonky3)...");
                match audit.verify_zk_integrity().await {
                    Ok(true) => println!("✅ Succinct Audit Proof Verified: Entire history is mathematically certain."),
                    Ok(false) => {
                        println!("❌ ZK PROOF FAILURE: Potential history rewrite or proof mismatch!");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        println!("❌ Verification Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("🔍 Verifying cryptographic integrity and signatures...");
                match audit
                    .verify_integrity_with_key(&agent.signing_key.verifying_key())
                    .await
                {
                    Ok(true) => println!(
                        "✅ Audit Log Integrity Verified: All hashes and signatures are valid."
                    ),
                    Ok(false) => {
                        println!("❌ INTEGRITY BREACH: Tampering or invalid signatures detected!");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        println!("❌ Verification Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Kernel { command } => match command {
            KernelCommands::Test { ip, port, policy } => {
                println!("🛡️ System-Level Integrity (eBPF Simulator)");
                println!("   Target: {}:{}", ip, port);

                let interceptor = attest_rs::kernel::KernelInterceptor::new();

                if let Some(policy_path) = policy {
                    println!("   Loading Policy: {:?}", policy_path);
                    let bytecode = std::fs::read(policy_path)?;
                    interceptor.load_program(bytecode).await?;
                } else {
                    println!("   ⚠️ No policy loaded (Default: ALLOW)");
                }

                match interceptor.inspect_connect(ip, *port).await {
                    Ok(true) => println!("✅ VERDICT: ALLOW"),
                    Ok(false) => {
                        println!("⛔ VERDICT: DROP (Blocked by Policy)");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        println!("❌ Execution Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
    }

    Ok(())
}
