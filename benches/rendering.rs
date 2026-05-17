use criterion::{criterion_group, criterion_main, Criterion};

fn rendering_benchmarks(_c: &mut Criterion) {
    // Placeholder: rendering benchmarks will be added in plan 07-04
}

criterion_group!(benches, rendering_benchmarks);
criterion_main!(benches);
