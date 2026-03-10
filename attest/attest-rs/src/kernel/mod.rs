pub mod hooks;
pub mod vm;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The KernelInterceptor acts as the bridge between user-space events and our simulated eBPF kernel.
pub struct KernelInterceptor {
    vm: Arc<Mutex<vm::EbpfVm>>,
}

impl Default for KernelInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelInterceptor {
    pub fn new() -> Self {
        Self {
            vm: Arc::new(Mutex::new(vm::EbpfVm::new())),
        }
    }

    /// Load an eBPF program into the simulated kernel.
    pub async fn load_program(&self, bytecode: Vec<u8>) -> Result<()> {
        let mut vm = self.vm.lock().await;
        vm.load(bytecode)?;
        Ok(())
    }

    /// Simulate a socket connection attempt and return the eBPF verdict.
    pub async fn inspect_connect(&self, ip: &str, port: u16) -> Result<bool> {
        let vm = self.vm.lock().await;
        let context = hooks::ConnectContext {
            ip: ip.to_string(),
            port,
        };

        // Execute the eBPF program against this context
        let result = vm.execute(&context)?;

        // In eBPF, typically return code 1/PASS means allowed, 0/DROP means blocked.
        // Or specific enums. We'll use 1 = Allow, 0 = Block.
        Ok(result != 0)
    }
}
