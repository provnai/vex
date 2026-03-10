use crate::error::AttestError;
use crate::persist::audit::{ActorType, AuditEventType, AuditStore};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Terminal Interceptor for command capture and signed session logging.
pub struct AttestTerminalInterceptor {
    audit: Arc<Mutex<AuditStore>>,
    agent: Arc<crate::id::AttestAgent>,
    policy: Arc<Mutex<crate::runtime::policy::PolicyEngine>>,
}

impl AttestTerminalInterceptor {
    pub fn new(
        audit: Arc<Mutex<AuditStore>>,
        agent: Arc<crate::id::AttestAgent>,
        policy: Arc<Mutex<crate::runtime::policy::PolicyEngine>>,
    ) -> Self {
        Self {
            audit,
            agent,
            policy,
        }
    }

    /// Run a command and capture its output through a high-fidelity relay
    pub async fn run_command(&self, cmd_line: &str) -> Result<(), AttestError> {
        // 0. Evaluate Policy
        {
            let policy = self.policy.lock().await;
            let ctx = crate::runtime::policy::ActionContext {
                action_type: "command".into(),
                target: cmd_line.into(),
                agent_id: self.agent.id.clone(),
                ..Default::default()
            };
            let (allowed, results) = policy.should_allow(&ctx);

            if !allowed {
                let violation_msg = results
                    .iter()
                    .filter(|r| r.action == crate::runtime::policy::PolicyAction::Block)
                    .map(|r| r.message.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                println!("❌ POLICY VIOLATION: {}", violation_msg);

                // Log Violation to Audit
                let audit = self.audit.lock().await;
                audit
                    .log(
                        AuditEventType::PolicyViolated,
                        None,
                        serde_json::json!({
                            "command": cmd_line,
                            "violation": violation_msg,
                            "action": "blocked"
                        }),
                        ActorType::System,
                        &self.agent,
                    )
                    .await?;

                return Err(AttestError::Alignment(format!(
                    "Command blocked by policy: {}",
                    violation_msg
                )));
            }

            // Handle Warnings (Notify user but allow execution)
            let warnings = results
                .iter()
                .filter(|r| r.action == crate::runtime::policy::PolicyAction::Warn)
                .map(|r| r.message.clone())
                .collect::<Vec<_>>();

            for warn in warnings {
                println!("⚠️ POLICY WARNING: {}", warn);
            }
        }

        // 1. Log Execution Intent
        {
            let audit = self.audit.lock().await;
            audit
                .log(
                    AuditEventType::AgentExecuted,
                    None,
                    json!({ "command": cmd_line, "status": "starting" }),
                    ActorType::System,
                    &self.agent,
                )
                .await?;
        }

        let cmd_line_str = cmd_line.to_string();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);
        let (pid_tx, mut pid_rx) = tokio::sync::mpsc::channel::<u32>(1);

        // Execute in a blocking task to handle continuous synchronous I/O relay
        let relay_handle = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            #[cfg(windows)]
            let mut child = std::process::Command::new("cmd.exe")
                .arg("/c")
                .arg(&cmd_line_str)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Spawn failed: {}", e))?;

            #[cfg(not(windows))]
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd_line_str)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Spawn failed: {}", e))?;

            let pid = child.id();
            let _ = pid_tx.blocking_send(pid);

            let mut stdout = child.stdout.take().expect("Stdout not redirected");
            let mut stderr = child.stderr.take().expect("Stderr not redirected");

            // Dedicated thread for Stdout relay
            let tx_out = tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                while let Ok(n) = stdout.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if tx_out.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            });

            // Dedicated thread for Stderr relay
            let tx_err = tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                while let Ok(n) = stderr.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if tx_err.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            });

            // Drop original transmitter to allow receiver to terminate correctly
            drop(tx);

            child.wait().map_err(|e| format!("Wait failed: {}", e))
        });

        // 2. Start Network Watchman in parallel with the relay loop
        let mut network_activity = Vec::new();
        let (network_handle, watchman) = if let Some(pid) = pid_rx.recv().await {
            let wm = crate::runtime::network::NetworkWatchman::new(pid);
            (Some(wm.start()), Some(wm))
        } else {
            (None, None)
        };

        // 3. Consume output stream
        let mut session_captured = Vec::new();
        while let Some(chunk) = rx.recv().await {
            session_captured.extend(chunk);
        }

        // Wait for the relay task to finish
        let _ = relay_handle.await;

        // 4. Collect Network Activity
        if let (Some(handle), Some(wm)) = (network_handle, watchman) {
            wm.stop();
            network_activity = handle.await.unwrap_or_default();
        }

        // 5. Final Log Entry
        {
            let audit = self.audit.lock().await;
            audit
                .log(
                    AuditEventType::AgentExecuted,
                    None,
                    json!({
                        "command": cmd_line,
                        "status": "completed",
                        "transcript_len": session_captured.len(),
                        "network_activity": network_activity
                    }),
                    ActorType::System,
                    &self.agent,
                )
                .await?;
        }

        Ok(())
    }
}
