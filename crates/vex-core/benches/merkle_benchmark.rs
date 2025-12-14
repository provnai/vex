use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use vex_core::merkle::{Hash, MerkleTree};

fn generate_leaves(n: usize) -> Vec<(String, Hash)> {
    (0..n)
        .map(|i| {
            let data = format!("data-{}", i);
            let hash = Hash::digest(data.as_bytes());
            (format!("id-{}", i), hash)
        })
        .collect()
}

fn bench_tree_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("MerkleTree::from_leaves");

    for size in [10, 100, 1000, 10000].iter() {
        let leaves = generate_leaves(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &leaves, |b, leaves| {
            b.iter(|| MerkleTree::from_leaves(black_box(leaves.clone())))
        });
    }
    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let size = 1000;
    let leaves = generate_leaves(size);
    let tree = MerkleTree::from_leaves(leaves.clone());
    let target_hash = leaves[500].1.clone();

    c.bench_function("MerkleTree::contains", |b| {
        b.iter(|| tree.contains(black_box(&target_hash)))
    });
}

criterion_group!(benches, bench_tree_creation, bench_contains);
criterion_main!(benches);
