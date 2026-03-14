use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use vex_core::segment::{
    AuthorityData, Capsule, CryptoData, IdentityData, IntentData, WitnessData,
};
use vex_core::VEP_MAGIC;
use vex_hardware::api::AgentIdentity;

#[derive(Clone)]
struct AppState {
    identity: Arc<AgentIdentity>,
    target_url: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize real hardware identity
    let identity = Arc::new(AgentIdentity::new());
    info!(
        "VEX Sidecar initialized with Agent ID: {}",
        identity.agent_id
    );

    let state = AppState {
        identity,
        target_url: std::env::var("TARGET_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    };

    // Build our router
    let app = Router::new()
        .route("/*path", post(proxy_handler))
        .with_state(state);

    // Run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("VEX Sidecar proxy listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body_bytes: Bytes,
) -> impl IntoResponse {
    // Check for VEP header to see if it's already enveloped
    let is_vep = body_bytes.starts_with(&VEP_MAGIC);

    if is_vep {
        info!("Request is already VEP enveloped. Forwarding to target...");
        return forward_request(&state.target_url, headers, body_bytes).await;
    }

    info!(
        "Simulating VEX-API pre-routing check for body of size {}...",
        body_bytes.len()
    );

    // Encapsulate into VEP (Silicon Boundary)
    info!("Encapsulating payload into VEP Binary format...");

    // Hash the original body for tracking
    let payload_hash = hex::encode(Sha256::digest(&body_bytes));

    // 1. Construct IntentData (Hardened v0.1)
    let intent = IntentData {
        request_sha256: payload_hash.clone(),
        confidence: 1.0,
        capabilities: vec!["proxy-forwarding".to_string()],
        magpie_source: None,
    };

    // 2. Construct IdentityData (Real Hardware ID)
    let identity = IdentityData {
        aid: state.identity.agent_id.clone(),
        identity_type: "hardware-rooted".to_string(),
        pcrs: None,
    };

    // 3. Construct AuthorityData (Mocking a local "pass")
    let authority = AuthorityData {
        capsule_id: format!("sidecar-{}", uuid::Uuid::new_v4()),
        outcome: "ALLOW".to_string(),
        reason_code: "PASSED_SIDECAR".to_string(),
        trace_root: payload_hash.clone(),
        nonce: 1,
        gate_sensors: serde_json::Value::Null,
    };

    // 4. Construct WitnessData (Local timestamp)
    let witness = WitnessData {
        chora_node_id: "sidecar-local".to_string(),
        receipt_hash: payload_hash.clone(), // In proxy mode, we link to payload
        timestamp: chrono::Utc::now().timestamp() as u64,
        metadata: serde_json::Value::Null,
    };

    // 5. Build Pillar Hashes
    let intent_hash = intent.to_jcs_hash().unwrap().to_hex();

    fn hash_seg<T: serde::Serialize>(seg: &T) -> String {
        let jcs = serde_jcs::to_vec(seg).unwrap();
        hex::encode(Sha256::digest(&jcs))
    }

    let authority_hash = hash_seg(&authority);
    let identity_hash = hash_seg(&identity);
    let witness_hash = witness.to_commitment_hash().unwrap();

    // 6. Assemble Hardened Capsule
    let mut capsule = Capsule {
        capsule_id: authority.capsule_id.clone(),
        intent,
        authority,
        identity,
        witness,
        intent_hash,
        authority_hash,
        identity_hash,
        witness_hash,
        capsule_root: String::new(),
        crypto: CryptoData {
            algo: "ed25519".to_string(),
            public_key_endpoint: "/public_key".to_string(),
            signature_scope: "capsule_root".to_string(),
            signature_b64: String::new(), // Signed below
        },
        request_commitment: None,
    };

    // 7. Compute Root and Sign
    let root = capsule.to_composite_hash().unwrap();
    capsule.capsule_root = root.to_hex();

    let sig_bytes = state.identity.sign(&root.0);
    capsule.crypto.signature_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig_bytes);

    info!(
        "Capsule hardened and signed. Root={}",
        &capsule.capsule_root[..8]
    );

    // 8. Build VEP binary (Legacy TLV for now, but with hardened capsule)
    // In a real implementation, we would use the VEP builder.
    let capsule_json = serde_json::to_vec(&capsule).unwrap();

    // For now, we'll just return the capsule JSON as the body to the target
    // in this simulation path.
    forward_request(&state.target_url, headers, Bytes::from(capsule_json)).await
}

async fn forward_request(target_url: &str, headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let mut req = client.post(target_url).body(body);

    for (name, value) in headers.iter() {
        if name != "host" {
            req = req.header(name, value);
        }
    }

    match req.send().await {
        Ok(res) => {
            let status = res.status();
            let body = res.bytes().await.unwrap_or_default();
            (status, body).into_response()
        }
        Err(e) => {
            error!("Forwarding failed: {}", e);
            (StatusCode::BAD_GATEWAY, "Target unreachable").into_response()
        }
    }
}
