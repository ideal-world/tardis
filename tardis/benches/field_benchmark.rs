use criterion::{criterion_group, criterion_main, Criterion};

use tardis::TardisFuns;

pub fn field_process(c: &mut Criterion) {
    c.bench_function("FIELD: incr_by_base62", |b| b.iter(|| TardisFuns::field.incr_by_base62("a9999")));
    c.bench_function("FIELD: incr_by_base36", |b| b.iter(|| TardisFuns::field.incr_by_base36("a9999")));
}

criterion_group!(benches, field_process);
criterion_main!(benches);
