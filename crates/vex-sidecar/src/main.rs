use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    routing::any,
    Router,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use vex_core::{Hash, IntentData, VepHeader, VepSegmentHeader, VEP_MAGIC, VEP_VERSION_V2};
use vex_hardware::api::{AgentIdentity, HardwareKeystore};
use zerocopy::U32;

#[derive(Clone)]
struct AppState {
    _vex_api_url: String,
    target_url: String,
    identity: Arc<AgentIdentity>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Task 5: Initialize hardware identity at startup.
    // VEX_HARDWARE_SEED (64 hex chars) gates real hardware vs dev fallback.
    let seed: [u8; 32] = if let Ok(hex_seed) = std::env::var("VEX_HARDWARE_SEED") {
        let bytes = hex::decode(&hex_seed).expect("VEX_HARDWARE_SEED must be 64 hex chars");
        bytes
            .try_into()
            .expect("VEX_HARDWARE_SEED must be exactly 32 bytes")
    } else if std::env::var("VEX_DEV_MODE").is_ok()
        || std::env::var("VEX_ENV")
            .map(|v| v == "railway")
            .unwrap_or(false)
    {
        warn!("⚠️  Hardware fallback active (DEV_MODE or Railway): Using zero seed. NOT FOR PRODUCTION.");
        [0u8; 32]
    } else {
        panic!("VEX_HARDWARE_SEED required. Set VEX_DEV_MODE=1 for local dev or template testing.");
    };

    let keystore = HardwareKeystore::new()
        .await
        .expect("Failed to initialize hardware keystore");

    let identity = Arc::new(
        keystore
            .get_identity(&seed)
            .await
            .expect("Failed to get hardware identity"),
    );

    info!(
        "⚓ Sidecar Hardware Identity Active: agent_id={}",
        identity.agent_id
    );

    let state = Arc::new(AppState {
        _vex_api_url: std::env::var("VEX_API_URL")
            .unwrap_or_else(|_| "http://localhost:8000".into()),
        target_url: std::env::var("VEX_TARGET_URL")
            .unwrap_or_else(|_| "http://localhost:3000/v2/vep".into()),
        identity,
    });

    let app = Router::new()
        .route("/*path", any(proxy_handler))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("VEX-Sidecar Proxy running on {}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    info!("Intercepting request to sidecar...");

    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    info!(
        "Simulating VEX-API pre-routing check for body of size {}...",
        body_bytes.len()
    );

    // Encapsulate into VEP (Silicon Boundary)
    info!("Encapsulating payload into VEP Binary format...");

    // Hash the original body for trace_root
    let trace_root_str = Hash::digest(&body_bytes).to_string();

    // Construct IntentData
    let intent_data = IntentData {
        id: "vex-sidecar-proxy-v0.2.0".into(),
        goal: "Forwarding request".into(),
        description: Some(format!("Tracing root: {}", trace_root_str)),
        ticket_id: None,
        constraints: vec![],
        acceptance_criteria: vec![],
        status: "OPEN".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        closed_at: None,
    };
    let intent_json = serde_jcs::to_vec(&intent_data).unwrap();

    // Task 5: Real identity segment — agent_id from hardware
    let ident_json = serde_json::json!({
        "agent": state.identity.agent_id,
        "tpm": "sidecar-proxy" // TPM quote not available in proxy path; agent_id is real
    });
    let ident_data = serde_json::to_vec(&ident_json).unwrap();

    // Auth segment — nonce + JCS trace_root bytes
    let trace_root_bytes: Vec<u8> = {
        let mut h = Sha256::new();
        h.update(&body_bytes);
        h.finalize().to_vec()
    };
    let mut trace_root_arr = [0u8; 32];
    trace_root_arr.copy_from_slice(&trace_root_bytes);

    let auth_data = serde_json::json!({
        "nonce": 12345u64,
        "trace_root": trace_root_arr
    });
    let auth_bytes = serde_json::to_vec(&auth_data).unwrap();

    // Task 5: Real capsule_root = SHA-256 of (intent_json || ident_data || auth_bytes)
    let mut capsule_hasher = Sha256::new();
    capsule_hasher.update(&intent_json);
    capsule_hasher.update(&ident_data);
    capsule_hasher.update(&auth_bytes);
    let capsule_root_bytes: [u8; 32] = capsule_hasher.finalize().into();

    // Task 5: Real Ed25519 signature over capsule_root from hardware identity
    let sig_bytes = state.identity.sign(&capsule_root_bytes);

    // Task 5: Real aid = SHA-256 of the agent_id string (deterministic public identity hash)
    let mut aid_hasher = Sha256::new();
    aid_hasher.update(state.identity.agent_id.as_bytes());
    let aid: [u8; 32] = aid_hasher.finalize().into();

    info!(
        "Task 5: Real sig generated ({} bytes), aid={}, capsule_root={}",
        sig_bytes.len(),
        hex::encode(&aid[..4]),
        hex::encode(&capsule_root_bytes[..4])
    );

    // Build VEP binary
    let mut vep_buffer = Vec::new();

    let header = VepHeader {
        magic: VEP_MAGIC,
        version: VEP_VERSION_V2,
        aid,
        capsule_root: capsule_root_bytes,
        nonce: 12345u64.to_le_bytes(),
    };

    let h_size = std::mem::size_of::<VepHeader>();
    let sh_size = std::mem::size_of::<VepSegmentHeader>();
    let payload_offset = (h_size + (4 * sh_size)) as u32;

    let seg_ident = VepSegmentHeader {
        segment_type: 1, // Identity
        offset: U32::new(payload_offset),
        length: U32::new(ident_data.len() as u32),
    };
    let seg_auth = VepSegmentHeader {
        segment_type: 2, // Authority
        offset: U32::new(payload_offset + ident_data.len() as u32),
        length: U32::new(auth_bytes.len() as u32),
    };
    let seg_intent = VepSegmentHeader {
        segment_type: 3, // Intent
        offset: U32::new(payload_offset + ident_data.len() as u32 + auth_bytes.len() as u32),
        length: U32::new(intent_json.len() as u32),
    };
    let seg_sig = VepSegmentHeader {
        segment_type: 4, // Hardware Signature
        offset: U32::new(
            payload_offset
                + ident_data.len() as u32
                + auth_bytes.len() as u32
                + intent_json.len() as u32,
        ),
        length: U32::new(sig_bytes.len() as u32),
    };

    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&header));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_ident));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_auth));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_intent));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_sig));

    vep_buffer.extend_from_slice(&ident_data);
    vep_buffer.extend_from_slice(&auth_bytes);
    vep_buffer.extend_from_slice(&intent_json);
    vep_buffer.extend_from_slice(&sig_bytes);

    info!(
        "Forwarding VEP packet ({} bytes) to {}...",
        vep_buffer.len(),
        state.target_url
    );

    let mut headers = parts.headers;
    headers.remove(axum::http::header::CONTENT_LENGTH);
    headers.remove(axum::http::header::HOST);

    let client = reqwest::Client::new();
    let forward_res = client
        .request(parts.method, &state.target_url)
        .headers(headers)
        .body(vep_buffer)
        .send()
        .await
        .map_err(|e| {
            error!("Forwarding failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    let mut res_builder = Response::builder().status(forward_res.status());
    for (name, value) in forward_res.headers() {
        res_builder = res_builder.header(name, value);
    }

    Ok(res_builder
        .body(Body::from(forward_res.bytes().await.unwrap()))
        .unwrap())
}
