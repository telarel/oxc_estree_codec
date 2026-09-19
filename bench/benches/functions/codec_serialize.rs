use std::hint::black_box;

use criterion::{BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use oxc::allocator::Allocator;

use oxc_estree_codec::{JsonToProgramOptions, ProgramToJsonOptions};

use crate::functions::fixtures::Fixture;

pub fn bench(
    criterion: &mut Criterion,
    fixtures: &[Fixture],
) {
    let mut group: BenchmarkGroup<'_, _> =
        criterion.benchmark_group("rust_to_js");

    for fixture in fixtures {
        let allocator: Allocator = Allocator::default();

        let program: oxc::ast::ast::Program<'_> =
            oxc_estree_codec::json_to_program(
                &fixture.json,
                JsonToProgramOptions {
                    allocator: &allocator,
                    source_type: fixture.source_type,
                    source_text: fixture.source,
                },
            )
            .unwrap();

        group.throughput(Throughput::Bytes(fixture.json.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("codec_serialize", fixture.name),
            fixture,
            |b, _fixture| {
                b.iter(|| {
                    let json: String = oxc_estree_codec::program_to_json(
                        black_box(&program),
                        ProgramToJsonOptions::new(),
                    );

                    black_box(json.len())
                })
            },
        );
    }

    group.finish();
}
