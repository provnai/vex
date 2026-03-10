use crate::cloud::client::ProvnCloudClient;
use crate::id::AttestAgent;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use vex_anchor::{AnchorBackend, AnchorError, AnchorMetadata, AnchorReceipt};
use vex_core::Hash;

/// ProvnAnchor implements the VEX AnchorBackend trait for ProvnCloud.
///
/// This is the "Perfect Connection" bridge:
/// VEX (Root) -> Attest (Adapter) -> Provn-SDK (Signature) -> ProvnCloud (L1/L3 Anchor)
pub struct ProvnAnchor {
    pub client: Arc<ProvnCloudClient>,
    pub agent: Arc<AttestAgent>,
}

impl std::fmt::Debug for ProvnAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProvnAnchor").finish_non_exhaustive()
    }
}

#[async_trait]
impl AnchorBackend for ProvnAnchor {
    fn name(&self) -> &str {
        "provncloud"
    }

    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        // 1. Prepare the claim data (Merging VEX root with Attest metadata)
        let claim_data = format!("vex:root:{}", root.to_hex());

        // 2. Sign with Provn-SDK identity
        let signed_claim = self.agent.sign_claim(claim_data);

        // 3. Forward to ProvnCloud API via Client
        // We use the client to push the signed claim
        let response = self
            .client
            .upload_event_raw(signed_claim, Some(metadata.clone()))
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        // 4. Construct VEX Receipt from ProvnCloud Response
        Ok(AnchorReceipt {
            backend: self.name().to_string(),
            root_hash: root.to_hex(),
            anchor_id: response.anchor_id.unwrap_or_else(|| "pending".to_string()),
            anchored_at: Utc::now(),
            proof: response.solscan_url,
            // metadata: metadata, // VEX AnchorReceipt might handle metadata differently or it was a field name issue
            metadata,
        })
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        // Implementation: Check status from ProvnCloud
        // This confirms the L1/L3 dual-anchoring status
        let status = self
            .client
            .get_status(&receipt.anchor_id)
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        Ok(status == "confirmed" || status == "anchored")
    }

    async fn is_healthy(&self) -> bool {
        self.client.is_healthy().await
    }
}
