use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    routing::any,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use vex_core::{
    Hash, IntentData, VepHeader, VepSegmentHeader, VEP_MAGIC, VEP_VERSION_V2,
};
use zerocopy::U32;

#[derive(Clone)]
struct AppState {
    _vex_api_url: String,
    target_url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        _vex_api_url: std::env::var("VEX_API_URL")
            .unwrap_or_else(|_| "http://localhost:8000".into()),
        target_url: std::env::var("VEX_TARGET_URL")
            .unwrap_or_else(|_| "http://localhost:3000/v2/vep".into()),
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

    // 1. Placeholder for scored authority logic (Phase 2.2 Integration)
    // In a real scenario, we'd call vex-api here to get a witness receipt for the intent.
    info!(
        "Simulating VEX-API pre-routing check for body of size {}...",
        body_bytes.len()
    );

    // 2. Encapsulate into VEP (Silicon Boundary)
    info!("Encapsulating payload into VEP Binary format (IntentSegment + Mock Auth/Ident/Sig)...");

    // Hash the original body for trace_root
    let trace_root = Hash::digest(&body_bytes).to_string();

    // Construct valid IntentData
    let intent_data = IntentData {
        id: "vex-sidecar-proxy-v0.2.0".into(),
        goal: "Forwarding request".into(),
        description: Some(format!("Tracing root: {}", trace_root)),
        ticket_id: None,
        constraints: vec![],
        acceptance_criteria: vec![],
        status: "OPEN".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        closed_at: None,
    };
    let intent_json = serde_jcs::to_vec(&intent_data).unwrap();

    // We'll create a well-formed VEP packet with all required segments
    let mut vep_buffer = Vec::new();

    // Mock segment data
    // Update these to match the new 4-pillar root if needed for strict validation
    let ident_data = b"{\"agent\":\"mock-hardware-id\",\"tpm\":\"MOCK_QUOTE\"}";
    let auth_data = b"{\"nonce\":12345,\"trace_root\":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}";
    let sig_data = vec![0u8; 64]; // Mock Ed25519 signature

    let _total_payload_len = intent_json.len() + ident_data.len() + auth_data.len() + sig_data.len();

    let header = VepHeader {
        magic: VEP_MAGIC,
        version: VEP_VERSION_V2,
        aid: [0u8; 32],
        capsule_root: [0u8; 32],
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
        length: U32::new(auth_data.len() as u32),
    };
    let seg_intent = VepSegmentHeader {
        segment_type: 3, // Intent
        offset: U32::new(payload_offset + ident_data.len() as u32 + auth_data.len() as u32),
        length: U32::new(intent_json.len() as u32),
    };
    let seg_sig = VepSegmentHeader {
        segment_type: 4, // Signature
        offset: U32::new(
            payload_offset
                + ident_data.len() as u32
                + auth_data.len() as u32
                + intent_json.len() as u32,
        ),
        length: U32::new(sig_data.len() as u32),
    };

    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&header));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_ident));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_auth));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_intent));
    vep_buffer.extend_from_slice(zerocopy::IntoBytes::as_bytes(&seg_sig));

    vep_buffer.extend_from_slice(ident_data);
    vep_buffer.extend_from_slice(auth_data);
    vep_buffer.extend_from_slice(&intent_json);
    vep_buffer.extend_from_slice(&sig_data);

    // 3. Forward to target
    info!(
        "Forwarding VEP packet ({} bytes, magic={:?}) to {}...",
        vep_buffer.len(),
        &vep_buffer[0..4],
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
