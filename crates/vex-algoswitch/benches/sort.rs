//! Comprehensive Sorting Benchmarks
//!
//! Tests different algorithms with various data patterns and sizes

use vex_algoswitch::{detect_pattern, sort};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// ============================================================================
// Data Generation Helpers
// ============================================================================

fn generate_random(size: usize) -> Vec<i64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    (0..size)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            hasher.write_usize(i);
            (hasher.finish() % 100000) as i64
        })
        .collect()
}

fn generate_sorted(size: usize) -> Vec<i64> {
    (0..size as i64).collect()
}

fn generate_nearly_sorted(size: usize) -> Vec<i64> {
    let mut data: Vec<i64> = (0..size as i64).collect();
    for i in (0..data.len()).step_by(10) {
        if i + 1 < data.len() {
            data.swap(i, i + 1);
        }
    }
    data
}

fn generate_few_unique(size: usize) -> Vec<i64> {
    (0..size).map(|i| (i % 10) as i64).collect()
}

// ============================================================================
// Individual Algorithm Benchmarks - Random Data
// ============================================================================

fn bench_quicksort_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("quicksort_random");

    for size in [100, 1000, 10000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::quicksort(&mut d);
            });
        });
    }
    group.finish();
}

fn bench_mergesort_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("mergesort_random");

    for size in [100, 1000, 10000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::mergesort(&mut d);
            });
        });
    }
    group.finish();
}

fn bench_heapsort_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("heapsort_random");

    for size in [100, 1000, 10000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::heapsort(&mut d);
            });
        });
    }
    group.finish();
}

fn bench_insertionsort_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertionsort_random");

    for size in [100, 1000, 10000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::insertionsort(&mut d);
            });
        });
    }
    group.finish();
}

fn bench_radixsort_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("radixsort_random");

    for size in [100, 1000, 10000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::radixsort(&mut d);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Pattern-Specific Benchmarks - Sorted
// ============================================================================

fn bench_insertionsort_sorted(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertionsort_sorted");

    for size in [100, 1000, 10000].iter() {
        let data = generate_sorted(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::insertionsort(&mut d);
            });
        });
    }
    group.finish();
}

fn bench_quicksort_sorted(c: &mut Criterion) {
    let mut group = c.benchmark_group("quicksort_sorted");

    for size in [100, 1000, 10000].iter() {
        let data = generate_sorted(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::quicksort(&mut d);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Pattern-Specific Benchmarks - Nearly Sorted
// ============================================================================

fn bench_insertionsort_nearly(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertionsort_nearly_sorted");

    for size in [100, 1000, 10000].iter() {
        let data = generate_nearly_sorted(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::insertionsort(&mut d);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Pattern-Specific Benchmarks - Few Unique
// ============================================================================

fn bench_radixsort_few_unique(c: &mut Criterion) {
    let mut group = c.benchmark_group("radixsort_few_unique");

    for size in [100, 1000, 10000].iter() {
        let data = generate_few_unique(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b: &mut criterion::Bencher, _size| {
            b.iter(|| {
                let mut d = data.clone();
                sort::radixsort(&mut d);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Pattern Detection Benchmarks
// ============================================================================

fn bench_pattern_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_detection");

    for size in [100, 1000, 10000, 100000].iter() {
        let data = generate_random(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _size| {
            b.iter(|| {
                detect_pattern(&data);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Large Data Benchmarks
// ============================================================================

fn bench_large_100k(c: &mut Criterion) {
    let data = generate_random(100000);
    c.bench_function("quicksort_100k_random", |b| {
        b.iter(|| {
            let mut d = data.clone();
            sort::quicksort(&mut d);
        });
    });
}

fn bench_large_1m(c: &mut Criterion) {
    let data = generate_random(1000000);
    c.bench_function("quicksort_1m_random", |b| {
        b.iter(|| {
            let mut d = data.clone();
            sort::quicksort(&mut d);
        });
    });
}

// ============================================================================
// Run All Benchmarks
// ============================================================================

criterion_group!(
    benches,
    bench_quicksort_random,
    bench_mergesort_random,
    bench_heapsort_random,
    bench_insertionsort_random,
    bench_radixsort_random,
    bench_insertionsort_sorted,
    bench_quicksort_sorted,
    bench_insertionsort_nearly,
    bench_radixsort_few_unique,
    bench_pattern_detection,
    bench_large_100k,
    bench_large_1m,
);
criterion_main!(benches);
