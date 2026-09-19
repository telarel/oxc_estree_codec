use std::hint::black_box;

use criterion::{BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use oxc::allocator::Allocator;

use oxc_estree_codec::__internal::ProgramReader;

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
            BenchmarkId::new("ast_read", fixture.name),
            fixture,
            |b, fixture| {
                b.iter(|| {
                    let allocator: Allocator = Allocator::default();

                    let reader: ProgramReader<'_> =
                        ProgramReader::new(&allocator);

                    let program: oxc::ast::ast::Program<'_> = reader
                        .read(
                            black_box(&fixture.value),
                            fixture.source_type,
                            fixture.source,
                        )
                        .unwrap();

                    black_box(std::ptr::from_ref(&program));
                })
            },
        );
    }

    group.finish();
}
