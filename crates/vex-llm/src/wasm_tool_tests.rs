#[cfg(test)]
mod tests {
    use crate::wasm_tool::WasmTool;
    use crate::tool::{Tool, ToolDefinition, Capability};
    use serde_json::json;
    use wat::parse_str as wat2wasm;


    #[tokio::test]
    async fn test_wasm_tool_basic() {
        let def = ToolDefinition::new("test", "test description", r#"{"type": "object"}"#);
        // We'll use a simpler WAT that just returns a fixed JSON
        let wasm = wat2wasm(r#"
            (module
                (memory (export "memory") 1)
                (func (export "vex_allocate") (param i32) (result i32) (i32.const 0))
                (func (export "vex_execute") (param i32 i32) (result i64)
                    ;; Let's do it byte by byte for "{"ok":true}"
                    ;; { o k " : t r u e }
                    (i32.store8 (i32.const 0) (i32.const 123)) ;; {
                    (i32.store8 (i32.const 1) (i32.const 34))  ;; "
                    (i32.store8 (i32.const 2) (i32.const 111)) ;; o
                    (i32.store8 (i32.const 3) (i32.const 107)) ;; k
                    (i32.store8 (i32.const 4) (i32.const 34))  ;; "
                    (i32.store8 (i32.const 5) (i32.const 58))  ;; :
                    (i32.store8 (i32.const 6) (i32.const 116)) ;; t
                    (i32.store8 (i32.const 7) (i32.const 114)) ;; r
                    (i32.store8 (i32.const 8) (i32.const 117)) ;; u
                    (i32.store8 (i32.const 9) (i32.const 101)) ;; e
                    (i32.store8 (i32.const 10) (i32.const 125)) ;; }
                    
                    i64.const 11 ;; len=11, ptr=0
                )
            )
        "#).unwrap().to_vec();

        let tool = WasmTool::new(def, wasm, vec![Capability::PureComputation]);
        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result, json!({"ok": true}));
    }

    #[tokio::test]
    async fn test_wasm_fuel_limit() {
        let def = ToolDefinition::new("infinite", "infinite loop", r#"{"type": "object"}"#);
        let wasm = wat2wasm(r#"
            (module
                (memory (export "memory") 1)
                (func (export "vex_allocate") (param i32) (result i32) (i32.const 0))
                (func (export "vex_execute") (param i32 i32) (result i64)
                    (loop
                        br 0
                    )
                    i64.const 0
                )
            )
        "#).unwrap().to_vec();

        let tool = WasmTool::new(def, wasm, vec![Capability::PureComputation])
            .with_fuel_limit(1000);
            
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("timeout") || err.to_string().contains("trapped"));
    }

    #[tokio::test]
    async fn test_wasm_memory_limit() {
        let def = ToolDefinition::new("oom", "too much memory", r#"{"type": "object"}"#);
        let wasm = wat2wasm(r#"
            (module
                (memory (export "memory") 1 100)
                (func (export "vex_allocate") (param i32) (result i32) (i32.const 0))
                (func (export "vex_execute") (param i32 i32) (result i64)
                    ;; Try to grow memory beyond limits (1MB = 16 pages)
                    (memory.grow (i32.const 50)) ;; 50 pages = ~3.2MB
                    drop
                    
                    ;; Return {"ok":true}
                    (i32.store8 (i32.const 0) (i32.const 123)) ;; {
                    (i32.store8 (i32.const 1) (i32.const 125)) ;; }
                    
                    ;; Packed result: ptr=0, len=2
                    i64.const 2
                )
            )
        "#).unwrap().to_vec();

        let tool = WasmTool::new(def, wasm, vec![Capability::PureComputation])
            .with_memory_limit(1024 * 1024); // 1MB limit
            
        let result = tool.execute(json!({})).await;
        assert!(result.is_ok()); 
    }
}
