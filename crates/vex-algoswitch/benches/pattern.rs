//! Pattern Detection Benchmarks

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use vex_algoswitch::detect_pattern;

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

fn bench_pattern_detection_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_random");

    for size in [100, 1000, 10000, 100000, 1000000].iter() {
        let data = generate_random(*size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b: &mut criterion::Bencher, _size| {
                b.iter(|| detect_pattern(&data));
            },
        );
    }
    group.finish();
}

fn bench_pattern_detection_sorted(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_sorted");

    for size in [100, 1000, 10000, 100000, 1000000].iter() {
        let data = generate_sorted(*size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b: &mut criterion::Bencher, _size| {
                b.iter(|| detect_pattern(&data));
            },
        );
    }
    group.finish();
}

fn bench_pattern_detection_nearly(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_nearly");

    for size in [100, 1000, 10000, 100000, 1000000].iter() {
        let data = generate_nearly_sorted(*size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b: &mut criterion::Bencher, _size| {
                b.iter(|| detect_pattern(&data));
            },
        );
    }
    group.finish();
}

fn bench_pattern_detection_few_unique(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_few_unique");

    for size in [100, 1000, 10000, 100000, 1000000].iter() {
        let data = generate_few_unique(*size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b: &mut criterion::Bencher, _size| {
                b.iter(|| detect_pattern(&data));
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_pattern_detection_random,
    bench_pattern_detection_sorted,
    bench_pattern_detection_nearly,
    bench_pattern_detection_few_unique,
);
criterion_main!(benches);
