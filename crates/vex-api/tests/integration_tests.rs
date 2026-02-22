use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`
use vex_api::{
    auth::{Claims, JwtAuth},
    routes::api_router,
    state::AppState,
    tenant_rate_limiter::{RateLimitTier, TenantRateLimiter},
};
use vex_llm::Metrics;
use vex_persist::sqlite::SqliteBackend;
use vex_queue::{QueueBackend, WorkerConfig, WorkerPool};

async fn setup_state() -> AppState {
    // 1. Auth with known secret
    let jwt = JwtAuth::new("test-secret-key-123");

    // 2. Metrics & Rate Limiter
    let metrics = Arc::new(Metrics::new());
    let rate_limiter = Arc::new(TenantRateLimiter::new(RateLimitTier::Standard));
    let a2a_state = Arc::new(vex_api::a2a::handler::A2aState::default());

    // 3. In-memory DB
    let db = SqliteBackend::new("sqlite::memory:").await.unwrap();

    // 4. Queue (using same DB pool)
    let queue_backend = vex_persist::queue::SqliteQueueBackend::new(db.pool().clone());
    let worker_pool = WorkerPool::new_with_arc(
        Arc::new(queue_backend) as Arc<dyn QueueBackend>,
        WorkerConfig::default(),
    );

    // 6. AppState
    AppState::new(
        jwt,
        rate_limiter,
        metrics,
        Arc::new(db),
        Arc::new(worker_pool),
        a2a_state,
        Arc::new(vex_llm::MockProvider::new(vec![])),
    )
}

#[tokio::test]
async fn test_full_lifecycle() {
    let state = setup_state().await;
    let router = api_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        vex_api::middleware::auth_middleware,
    ));

    // 1. Health Check
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response: Response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Auth Token Generation (Admin role)
    // Use for_user helper to ensure all fields are set correctly
    let claims = Claims::for_user("test-user", "admin", chrono::Duration::days(1));
    let token = state.jwt_auth().encode(&claims).unwrap();
    let auth_header = format!("Bearer {}", token);

    // 3. Create Agent
    let agent_req_body = serde_json::json!({
        "name": "Test Agent",
        "role": "Assistant",
        "max_depth": 3,
        "spawn_shadow": false
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/agents")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .body(Body::from(agent_req_body.to_string()))
        .unwrap();

    let response: Response = router.clone().oneshot(req).await.unwrap();

    if response.status() != StatusCode::OK {
        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        println!("Create Agent Failed: {} - {}", status, body_str);
        panic!("Create Agent failed");
    }
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let agent_res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let agent_id = agent_res["id"].as_str().expect("Agent ID missing");
    assert_eq!(agent_res["name"], "Test Agent");

    // 4. Execute Agent Job
    let execute_req_body = serde_json::json!({
        "prompt": "Hello world",
        "enable_adversarial": true,
        "max_debate_rounds": 1
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agents/{}/execute", agent_id))
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .body(Body::from(execute_req_body.to_string()))
        .unwrap();

    let response: Response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let execute_res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(execute_res["response"]
        .as_str()
        .unwrap()
        .starts_with("Job queued"));

    // 5. Verify Metrics (Admin only)
    let req = Request::builder()
        .uri("/metrics") // user friendly alias or /api/v1/metrics
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();

    let response: Response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metrics_text = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert!(metrics_text.contains("vex_llm_calls_total 1"));
    assert!(metrics_text.contains("vex_agents_created_total 1"));
}
