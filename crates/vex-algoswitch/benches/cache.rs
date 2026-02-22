//! Cache Benchmarks

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use algoswitch::{get_cached, cache_winner, clear_cache, DataPattern};

fn bench_cache_get(c: &mut Criterion) {
    // Pre-populate cache
    clear_cache();
    cache_winner(&DataPattern::Sorted, "insertionsort");
    cache_winner(&DataPattern::Random, "quicksort");
    cache_winner(&DataPattern::NearlySorted, "insertionsort");
    cache_winner(&DataPattern::FewUnique, "radixsort");
    cache_winner(&DataPattern::ReverseSorted, "insertionsort");
    
    let mut group = c.benchmark_group("cache_get");
    
    for _ in 0..1000 {
        group.bench_function("hit", |b| {
            b.iter(|| get_cached(&DataPattern::Sorted));
        });
    }
    
    group.finish();
}

fn bench_cache_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_set");
    
    group.bench_function("single_write", |b| {
        b.iter(|| {
            cache_winner(&DataPattern::Sorted, "insertionsort");
        });
    });
    
    group.finish();
}

fn bench_cache_miss(c: &mut Criterion) {
    clear_cache();
    
    let mut group = c.benchmark_group("cache_miss");
    
    // Test non-existent patterns
    for _ in 0..1000 {
        group.bench_function("miss", |b| {
            b.iter(|| get_cached(&DataPattern::Sorted));
        });
    }
    
    group.finish();
}

fn bench_cache_clear(c: &mut Criterion) {
    // Pre-populate
    for _ in 0..100 {
        cache_winner(&DataPattern::Sorted, "insertionsort");
        cache_winner(&DataPattern::Random, "quicksort");
    }
    
    let mut group = c.benchmark_group("cache_clear");
    
    group.bench_function("clear", |b| {
        b.iter(|| clear_cache());
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_cache_get,
    bench_cache_set,
    bench_cache_miss,
    bench_cache_clear,
);
criterion_main!(benches);
