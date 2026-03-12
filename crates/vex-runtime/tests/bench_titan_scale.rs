use std::sync::Arc;
use tokio::time::Instant;
use uuid::Uuid;
use vex_runtime::gate::titan::SecurityProfile;
use vex_runtime::gate::{Gate, GenericGateMock, TitanGate};

#[tokio::test]
async fn bench_titan_scale_concurrency() {
    let mock_llm = Arc::new(vex_llm::MockProvider::constant(""));
    let inner_mock = Arc::new(GenericGateMock);
    let chora = vex_chora::client::make_mock_client();
    let identity = vex_hardware::api::AgentIdentity::new();
    let gate = Arc::new(TitanGate::new(
        inner_mock,
        mock_llm,
        chora,
        identity,
        SecurityProfile::Standard,
    ));

    let num_requests = 50;
    let mut tasks = Vec::new();

    println!(
        "Starting scale benchmark with {} concurrent Magpie verifications...",
        num_requests
    );
    let start_time = Instant::now();

    for i in 0..num_requests {
        let gate_clone = gate.clone();
        tasks.push(tokio::spawn(async move {
            let intent = format!(";; Intent number {}", i);
            gate_clone
                .execute_gate(Uuid::new_v4(), "Benchmarking scale", &intent, 1.0, vec![])
                .await
        }));
    }

    let results = futures::future::join_all(tasks).await;
    let duration = start_time.elapsed();

    let mut success_count = 0;
    for res in results {
        match res {
            Ok(cap) => {
                if cap.outcome == "ALLOW" {
                    success_count += 1;
                } else {
                    println!("Request halted. Reason: {}", cap.reason_code);
                }
            }
            Err(e) => eprintln!("Task panicked: {:?}", e),
        }
    }

    println!("--- Benchmark Results ---");
    println!("Total Requests: {}", num_requests);
    println!("Successful (ALLOW): {}", success_count);
    println!("Total Duration: {:?}", duration);
    println!(
        "Average Latency per Request (Total/Count): {:?}",
        duration / (num_requests as u32)
    );

    assert_eq!(
        success_count, num_requests,
        "All concurrent requests should have passed."
    );
}
