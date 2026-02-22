use vex_algoswitch::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");

    // Small data
    let small_data: Vec<i64> = (0..100).collect();

        group.bench_function("small_100", |b: &mut criterion::Bencher| {
 b.iter(|| {
 let mut data = small_data.clone();
 algo::select(SortFamily::standard(), &mut data, Config::default())
 });
 });

    // Medium data
    let medium_data: Vec<i64> = (0..10000).collect();

    group.bench_function("medium_10000", |b: &mut criterion::Bencher| {
 b.iter(|| {
 let mut data = medium_data.clone();
 algo::select(SortFamily::standard(), &mut data, Config::default())
 });
 });

    // Large data
    let large_data: Vec<i64> = (0..100000).collect();

    group.bench_function("large_100000", |b: &mut criterion::Bencher| {
 b.iter(|| {
 let mut data = large_data.clone();
 algo::select(SortFamily::standard(), &mut data, Config::default())
 });
 });

    group.finish();
}

criterion_group!(benches, bench_sort);
criterion_main!(benches);
