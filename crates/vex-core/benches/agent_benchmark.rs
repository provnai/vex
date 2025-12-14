//! Benchmarks for agent operations
//!
//! Run with: cargo bench -p vex-core

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use vex_core::{Agent, AgentConfig};

fn generate_agent_config(name: &str) -> AgentConfig {
    AgentConfig {
        name: name.to_string(),
        role: "Benchmark Agent".to_string(),
        max_depth: 3,
        spawn_shadow: false,
    }
}

/// Benchmark agent creation
fn bench_agent_creation(c: &mut Criterion) {
    c.bench_function("agent_creation", |b| {
        b.iter(|| {
            let config = generate_agent_config("BenchAgent");
            black_box(Agent::new(config))
        })
    });
}

/// Benchmark child spawning
fn bench_child_spawning(c: &mut Criterion) {
    let root = Agent::new(generate_agent_config("Root"));
    
    c.bench_function("child_spawning", |b| {
        b.iter(|| {
            let config = generate_agent_config("Child");
            black_box(root.spawn_child(config))
        })
    });
}

/// Benchmark batch agent creation at different scales
fn bench_batch_agent_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_agent_creation");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let agents: Vec<Agent> = (0..size)
                    .map(|i| Agent::new(generate_agent_config(&format!("Agent{}", i))))
                    .collect();
                black_box(agents)
            })
        });
    }
    
    group.finish();
}

criterion_group!(
    agent_benches,
    bench_agent_creation,
    bench_child_spawning,
    bench_batch_agent_creation
);

criterion_main!(agent_benches);
