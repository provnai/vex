//! WASM-based Tool Sandbox using Wasmtime
//!
//! This module provides `WasmTool`, which executes tool logic inside a
//! secure WebAssembly sandbox with:
//! - Memory limits (prevent OOM)
//! - CPU limits (Fuel, prevent infinite loops)
//! - Capability-based I/O isolation (WASI)

use async_trait::async_trait;
use serde_json::Value;
use wasmtime::{Config, Engine, Linker, Module, ResourceLimiter, Result as AnyhowResult, Store};
use wasmtime_wasi::WasiCtxBuilder;
// In wasmtime 22.0, Preview 1 resides in its own module
use wasmtime_wasi::preview1::{self, WasiP1Ctx};

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// A tool executed inside a secure WASM sandbox.
pub struct WasmTool {
    definition: ToolDefinition,
    module_bytes: Vec<u8>,
    capabilities: Vec<Capability>,
    memory_limit_bytes: usize,
    fuel_limit: u64,
}

impl WasmTool {
    /// Create a new WASM-sandboxed tool.
    ///
    /// **Capability enforcement:** Only `Environment` is enforced by WASI Preview 1.
    /// `Network` and `FileSystem` capabilities are recorded but NOT enforced by the
    /// current WASI Preview 1 runtime — they serve as documentation of intent.
    /// If `capabilities` is empty, the tool runs with no extra permissions (safest default).
    pub fn new(
        definition: ToolDefinition,
        module_bytes: Vec<u8>,
        capabilities: Vec<Capability>,
    ) -> Self {
        if capabilities.contains(&Capability::Network)
            || capabilities.contains(&Capability::FileSystem)
        {
            tracing::warn!(
                tool = %definition.name,
                "Tool requests Network/FileSystem capabilities which are not enforced by WASI Preview 1"
            );
        }
        Self {
            definition,
            module_bytes,
            capabilities,
            memory_limit_bytes: 64 * 1024 * 1024, // 64MB default
            fuel_limit: 10_000_000,               // 10M instructions default
        }
    }

    /// Set memory limit in bytes
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit_bytes = limit;
        self
    }

    /// Set fuel (CPU) limit
    pub fn with_fuel_limit(mut self, limit: u64) -> Self {
        self.fuel_limit = limit;
        self
    }
}

// Data passed to the WASM store
struct WasmStoreData {
    wasi: WasiP1Ctx,
    memory_limit: usize,
    table_elements_limit: u32,
}

// Resource table for WASI
// In Preview 1 we might still need a table for some extensions,
// but WasiView trait is not required for the core Linker.
// Let's keep the table in the data struct if needed later.

impl ResourceLimiter for WasmStoreData {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> AnyhowResult<bool> {
        if desired > self.memory_limit {
            return Ok(false);
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: u32,
        desired: u32,
        _maximum: Option<u32>,
    ) -> AnyhowResult<bool> {
        if desired > self.table_elements_limit {
            return Ok(false);
        }
        Ok(true)
    }
}

#[async_trait]
impl Tool for WasmTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        // 1. Configure Engine with Fuel support
        let mut config = Config::new();
        config.consume_fuel(true);
        config.async_support(true);
        config.wasm_bulk_memory(true);
        config.wasm_multi_value(true);
        config.wasm_reference_types(true);

        let engine = Engine::new(&config).map_err(|e| {
            ToolError::execution_failed(
                self.definition.name,
                format!("Failed to create WASM engine: {}", e),
            )
        })?;

        // 2. Load Module
        let module = Module::from_binary(&engine, &self.module_bytes).map_err(|e| {
            ToolError::execution_failed(
                self.definition.name,
                format!("Failed to load WASM module: {:?}", e),
            )
        })?;

        // 3. Setup WASI & Resource Limits
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdout().inherit_stderr();

        if self.capabilities.contains(&Capability::Environment) {
            builder.inherit_env();
        }

        if self.capabilities.contains(&Capability::Network) {
            // WASI Preview 1 does not have a standard network inheritance like Preview 2.
            // Custom host calls will be needed for fine-grained network access.
        }

        if self.capabilities.contains(&Capability::FileSystem) {
            // For now, we don't preopen directories unless a specific path is requested.
            // In a real implementation, we would map a sandbox directory here.
        }

        let wasi = builder.build_p1();
        let table_elements_limit = 1000; // Default table elements limit

        let mut store = Store::new(
            &engine,
            WasmStoreData {
                wasi,
                memory_limit: self.memory_limit_bytes,
                table_elements_limit,
            },
        );
        store.limiter(|s| s); // Use the ResourceLimiter implementation

        // 4. Set Fuel
        store.set_fuel(self.fuel_limit).map_err(|e| {
            ToolError::execution_failed(
                self.definition.name,
                format!("Failed to set WASM fuel: {}", e),
            )
        })?;

        // 5. Link WASI
        let mut linker = Linker::new(&engine);
        // In wasmtime 22.0, Preview 1 resides in its own module if using core Linker
        preview1::add_to_linker_async(&mut linker, |s: &mut WasmStoreData| &mut s.wasi).map_err(
            |e| {
                ToolError::execution_failed(
                    self.definition.name,
                    format!("Failed to link WASI: {}", e),
                )
            },
        )?;

        // 6. Instantiate
        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|e| {
                ToolError::execution_failed(
                    self.definition.name,
                    format!("Failed to instantiate WASM: {}", e),
                )
            })?;

        // 7. JSON Protocol Bridge
        // We expect:
        // - vex_allocate(size: u32) -> u32 (Pointer)
        // - vex_execute(ptr: u32, len: u32) -> u64 (Packed result ptr/len)
        let allocate = instance
            .get_typed_func::<u32, u32>(&mut store, "vex_allocate")
            .map_err(|_| {
                ToolError::execution_failed(
                    self.definition.name,
                    "WASM module must export 'vex_allocate(u32) -> u32'",
                )
            })?;

        let execute_fn = instance
            .get_typed_func::<(u32, u32), u64>(&mut store, "vex_execute")
            .map_err(|_| {
                ToolError::execution_failed(
                    self.definition.name,
                    "WASM module must export 'vex_execute(u32, u32) -> u64'",
                )
            })?;

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            ToolError::execution_failed(self.definition.name, "WASM module must export 'memory'")
        })?;

        // 8. Pass Arguments
        let input_json = serde_json::to_vec(&args)
            .map_err(|e| ToolError::invalid_args(self.definition.name, e.to_string()))?;
        let input_len = input_json.len() as u32;
        let input_ptr = allocate
            .call_async(&mut store, input_len)
            .await
            .map_err(|e| {
                ToolError::execution_failed(
                    self.definition.name,
                    format!("Failed to allocate WASM memory: {}", e),
                )
            })?;

        memory
            .write(&mut store, input_ptr as usize, &input_json)
            .map_err(|e| {
                ToolError::execution_failed(
                    self.definition.name,
                    format!("Failed to write to WASM memory: {}", e),
                )
            })?;

        // 9. Execute
        let result_packed = execute_fn
            .call_async(&mut store, (input_ptr, input_len))
            .await
            .map_err(|e| {
                if format!("{:?}", e).contains("OutOfFuel") {
                    ToolError::timeout(self.definition.name, 0)
                } else {
                    ToolError::execution_failed(
                        self.definition.name,
                        format!("WASM execution trapped: {}", e),
                    )
                }
            })?;

        // 10. Extract Result with strict allocation limits to prevent OOM
        let output_ptr = (result_packed >> 32) as u32;
        let output_len = (result_packed & 0xFFFFFFFF) as u32;

        // HARDENING (Phase 8): Prevent host memory exhaustion (OOM) from malicious WASM output lengths
        const MAX_WASM_OUTPUT_BYTES: u32 = 10 * 1024 * 1024; // 10MB Limit
        if output_len > MAX_WASM_OUTPUT_BYTES {
            return Err(ToolError::execution_failed(
                self.definition.name,
                format!("WASM returned an output size exceeding the strict 10MB limit ({} bytes requested)", output_len),
            ));
        }

        let mut output_buf = vec![0u8; output_len as usize];
        memory
            .read(&mut store, output_ptr as usize, &mut output_buf)
            .map_err(|e| {
                ToolError::execution_failed(
                    self.definition.name,
                    format!("Failed to read from WASM memory: {}", e),
                )
            })?;

        let output_value: Value = serde_json::from_slice(&output_buf).map_err(|e| {
            ToolError::execution_failed(
                self.definition.name,
                format!("WASM returned invalid JSON: {}", e),
            )
        })?;

        Ok(output_value)
    }
}
