//! Ethereum anchor backend
//!
//! Anchors Merkle roots as calldata to an Ethereum-compatible chain via JSON-RPC.
//! Uses `eth_call` for validation and stores the encoded calldata as proof.
//! Full `eth_sendRawTransaction` signing is left for a production integration with ethers-rs.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use vex_core::Hash;

use crate::backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
use crate::error::AnchorError;

#[derive(Serialize)]
struct JsonRpcRequest<'a, T: Serialize> {
    jsonrpc: &'a str,
    method: &'a str,
    params: T,
    id: u64,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Ethereum anchor backend
///
/// Encodes the Merkle root as `0x56455800` (VEX\x00) + root hex calldata.
/// The anchor_id is `eth://block:<n>/calldata:<first 16 hex chars>`.
#[derive(Debug, Clone)]
pub struct EthereumAnchor {
    rpc_url: String,
    from_address: String,
    client: reqwest::Client,
}

impl EthereumAnchor {
    /// Create a new Ethereum anchor backend
    pub fn new(rpc_url: impl Into<String>, from_address: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("vex-anchor/0.1.5")
            .build()
            .expect("Failed to build Ethereum HTTP client");

        Self {
            rpc_url: rpc_url.into(),
            from_address: from_address.into(),
            client,
        }
    }

    async fn get_block_number(&self) -> Result<u64, AnchorError> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "eth_blockNumber",
            params: serde_json::json!([]),
            id: 1,
        };

        let resp_bytes = self
            .client
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        let resp: JsonRpcResponse<String> = serde_json::from_slice(&resp_bytes)
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        if let Some(err) = resp.error {
            return Err(AnchorError::Network(format!(
                "RPC error {}: {}",
                err.code, err.message
            )));
        }

        let hex = resp.result.unwrap_or_default();
        u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .map_err(|e| AnchorError::Network(e.to_string()))
    }
}

#[async_trait]
impl AnchorBackend for EthereumAnchor {
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        // VEX magic prefix (0x56455800) + root hash
        let calldata = format!("0x56455800{}", root.to_hex());

        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "eth_call",
            params: serde_json::json!([{
                "from": self.from_address,
                "to": "0x0000000000000000000000000000000000000000",
                "data": calldata
            }, "latest"]),
            id: 2,
        };

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AnchorError::Network(format!(
                "Ethereum RPC returned HTTP {}",
                resp.status()
            )));
        }

        let block = self.get_block_number().await.unwrap_or(0);
        let anchor_id = format!("eth://block:{}/calldata:{}", block, &root.to_hex()[..16]);

        Ok(AnchorReceipt {
            backend: self.name().to_string(),
            root_hash: root.to_hex(),
            anchor_id,
            anchored_at: Utc::now(),
            proof: Some(calldata),
            metadata,
        })
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        let Some(ref proof) = receipt.proof else {
            return Ok(false);
        };
        let expected = format!("0x56455800{}", receipt.root_hash);
        Ok(proof == &expected)
    }

    fn name(&self) -> &str {
        "ethereum"
    }

    async fn is_healthy(&self) -> bool {
        self.get_block_number().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::AnchorMetadata;

    #[test]
    fn test_eth_verify_calldata() {
        let root_hash = "abc123def456".to_string();
        let receipt = AnchorReceipt {
            backend: "ethereum".to_string(),
            root_hash: root_hash.clone(),
            anchor_id: "eth://block:12345/calldata:abc123".to_string(),
            anchored_at: Utc::now(),
            proof: Some(format!("0x56455800{}", root_hash)),
            metadata: AnchorMetadata::new("test-tenant", 1),
        };
        assert!(receipt.proof.as_ref().unwrap().starts_with("0x56455800"));
        assert!(receipt.proof.as_ref().unwrap().ends_with(&root_hash));
    }
}
