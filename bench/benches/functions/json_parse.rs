use std::hint::black_box;

use criterion::{BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use sonic_rs::Value;

use crate::functions::fixtures::Fixture;

pub fn bench(
    criterion: &mut Criterion,
    fixtures: &[Fixture],
) {
    let mut group: BenchmarkGroup<'_, _> =
        criterion.benchmark_group("js_to_rust");

    for fixture in fixtures {
        group.throughput(Throughput::Bytes(fixture.json.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("json_parse", fixture.name),
            fixture,
            |b, fixture| {
                b.iter(|| {
                    let value: Value =
                        sonic_rs::from_str(black_box(fixture.json.as_str()))
                            .unwrap();

                    black_box(value)
                })
            },
        );
    }

    group.finish();
}
