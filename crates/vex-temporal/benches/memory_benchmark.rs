//! Benchmarks for temporal memory operations
//!
//! Run with: cargo bench -p vex-temporal

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_memory_insertion(c: &mut Criterion) {
    use vex_temporal::{EpisodicMemory, HorizonConfig};
    
    c.bench_function("memory_insertion", |b| {
        let mut memory = EpisodicMemory::new(HorizonConfig::default());
        b.iter(|| {
            memory.remember(black_box("Test memory content"), black_box(0.5));
        })
    });
}

fn bench_memory_by_importance(c: &mut Criterion) {
    use vex_temporal::{EpisodicMemory, HorizonConfig};
    
    let mut memory = EpisodicMemory::new(HorizonConfig::default());
    for i in 0..100 {
        memory.remember(&format!("Memory {}", i), (i as f64) / 100.0);
    }
    
    c.bench_function("memory_by_importance_100", |b| {
        b.iter(|| {
            black_box(memory.by_importance())
        })
    });
}

fn bench_memory_compression(c: &mut Criterion) {
    use vex_temporal::{TemporalCompressor, DecayStrategy};
    use chrono::Duration;
    
    let compressor = TemporalCompressor::new(DecayStrategy::Exponential, Duration::hours(24));
    let content = "This is a sample memory content that would typically be longer in a real application. It contains various details and information that the agent has accumulated over time.";
    
    c.bench_function("compression_50_percent", |b| {
        b.iter(|| {
            black_box(compressor.compress(content, 0.5))
        })
    });
}

fn bench_decay_calculation(c: &mut Criterion) {
    use vex_temporal::{TemporalCompressor, DecayStrategy};
    use chrono::{Duration, Utc};
    
    let compressor = TemporalCompressor::new(DecayStrategy::Exponential, Duration::hours(24));
    let old_time = Utc::now() - Duration::hours(12);
    
    c.bench_function("decay_calculation", |b| {
        b.iter(|| {
            black_box(compressor.importance(old_time, 0.8))
        })
    });
}

criterion_group!(
    memory_benches,
    bench_memory_insertion,
    bench_memory_by_importance,
    bench_memory_compression,
    bench_decay_calculation
);

criterion_main!(memory_benches);
